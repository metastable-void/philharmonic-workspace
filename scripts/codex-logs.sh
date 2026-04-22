#!/bin/sh
# scripts/codex-logs.sh — find the latest Codex session spawned
# from Claude Code and print its rollout, routed through the
# `codex-fmt` xtask bin for a human-readable rendering.
#
# Codex CLI writes one JSONL rollout per session under
# $CODEX_HOME/sessions/YYYY/MM/DD/rollout-*.jsonl (default
# CODEX_HOME is ~/.codex). The first line of each rollout is a
# session_meta record carrying an "originator" field:
# "Claude Code" for sessions spawned via the codex:* plugin;
# other values (the VSCode extension's direct invocations,
# user-driven `codex` CLI runs, etc.) for everything else. This
# script filters on that field so it surfaces only agent-spawned
# sessions, which is what Claude cares about when reviewing a
# still-running dispatch.
#
# By default the JSONL stream is piped through
# `./scripts/xtask.sh codex-fmt --` so the reader sees a compact,
# color-highlighted timeline with tool calls, messages, and
# token-count lines — encrypted reasoning blobs are replaced by
# a length-only placeholder. Pass `--raw` to skip the formatter
# and emit pure JSONL for machine consumption (piping to `jq`,
# redirecting to a file, etc.).
#
# "Best effort" means:
# - Newest rollout first by filename order (filenames embed ISO
#   timestamps; alphabetical reverse equals time reverse).
# - First match wins; older Claude-spawned rollouts in the same
#   day are skipped once we reach the most recent one.
# - If no Claude-spawned rollout exists yet, exit 1 with a short
#   error rather than fall through to a random user-driven
#   session.
#
# Usage:
#   ./scripts/codex-logs.sh                     # formatted snapshot
#   ./scripts/codex-logs.sh -f | --follow       # formatted tail -f
#   ./scripts/codex-logs.sh --raw               # raw JSONL snapshot
#   ./scripts/codex-logs.sh --raw -f            # raw JSONL tail -f
#   ./scripts/codex-logs.sh -n | --no-color     # forward --no-color to codex-fmt
#   ./scripts/codex-logs.sh -h | --help         # usage
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu

script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"

follow=0
raw=0
no_color=0
while [ $# -gt 0 ]; do
    case "$1" in
        -f|--follow)   follow=1;   shift ;;
        --raw)         raw=1;      shift ;;
        -n|--no-color) no_color=1; shift ;;
        -h|--help)
            cat <<EOF
Usage: $0 [-f|--follow] [--raw] [-n|--no-color]

Print the latest Codex session spawned from Claude Code. By
default the rollout is piped through \`./scripts/xtask.sh
codex-fmt\` for a human-readable rendering. With -f/--follow,
behaves like \`tail -f\` (prints the whole file first, then
streams appends as Codex writes more).

Flags:
  -f, --follow    Stream appends in tail -f style.
  --raw           Emit pure JSONL; skip the codex-fmt rendering.
  -n, --no-color  Forward --no-color to codex-fmt (ignored with --raw).

Sessions live under \$CODEX_HOME/sessions/YYYY/MM/DD/rollout-*.jsonl
(default CODEX_HOME is ~/.codex).
EOF
            exit 0
            ;;
        --) shift; break ;;
        -*)
            printf '!!! codex-logs.sh: unknown flag: %s\n' "$1" >&2
            exit 2
            ;;
        *)
            printf '!!! codex-logs.sh: unexpected argument: %s\n' "$1" >&2
            exit 2
            ;;
    esac
done

codex_home="${CODEX_HOME:-$HOME/.codex}"
sessions_root="$codex_home/sessions"

if [ ! -d "$sessions_root" ]; then
    printf '!!! codex-logs.sh: sessions directory not found: %s\n' "$sessions_root" >&2
    printf '    (Set CODEX_HOME if Codex stores sessions elsewhere.)\n' >&2
    exit 1
fi

# Walk rollouts newest-first. The filename format
# rollout-YYYY-MM-DDTHH-MM-SS-<uuid>.jsonl sorts chronologically
# alphabetically, so reverse-alphabetical == reverse-chronological.
# The inner `while` loop runs in a subshell (pipeline element); to
# pull the first match out, we break on hit and `head -n 1` on the
# outer pipeline caps the result to one path.
latest=$(
    find "$sessions_root" -type f -name 'rollout-*.jsonl' -print 2>/dev/null \
        | sort -r \
        | while IFS= read -r f; do
            if head -n 1 "$f" 2>/dev/null \
                | grep -q '"originator":"Claude Code"'; then
                printf '%s\n' "$f"
                break
            fi
        done \
        | head -n 1
)

if [ -z "$latest" ]; then
    printf '!!! codex-logs.sh: no Claude-Code-spawned rollout found under %s\n' "$sessions_root" >&2
    exit 1
fi

# Header to stderr so stdout stays clean (either rendered text
# or raw JSONL) for piping / redirection.
printf '=== codex rollout: %s ===\n' "$latest" >&2

fmt_flags=
if [ "$no_color" -eq 1 ]; then
    fmt_flags='--no-color'
fi

# Producer is either a one-shot `cat` or a streaming `tail -f`.
# `tail -n +1 -f` starts at line 1 and then follows — both flags
# are POSIX and supported on GNU / BSD tail.
if [ "$raw" -eq 1 ]; then
    if [ "$follow" -eq 1 ]; then
        exec tail -n +1 -f "$latest"
    else
        exec cat "$latest"
    fi
else
    # Pipeline: `exec` replaces only a single command, not a
    # pipeline, so we let the shell orchestrate and rely on SIGINT
    # propagating to the process group.
    # shellcheck disable=SC2086
    if [ "$follow" -eq 1 ]; then
        tail -n +1 -f "$latest" | "$script_dir/xtask.sh" codex-fmt -- $fmt_flags
    else
        # shellcheck disable=SC2086
        cat "$latest" | "$script_dir/xtask.sh" codex-fmt -- $fmt_flags
    fi
fi
