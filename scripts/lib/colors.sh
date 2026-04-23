# scripts/lib/colors.sh — ANSI color helpers for scripts/*.sh.
#
# Usage (from a script under scripts/):
#
#     #!/bin/sh
#     set -eu
#     . "$(dirname -- "$0")/lib/colors.sh"
#     . "$(dirname -- "$0")/lib/workspace-cd.sh"
#     ...
#     printf '%s=== heading %s ===%s\n' "$C_HEADER" "$thing" "$C_RESET"
#     printf '%sok%s %s\n'              "$C_OK"     "$C_RESET" "$thing"
#     printf '%s!!! %s: %s%s\n'         "$C_ERR"    "$prog" "$msg" "$C_RESET" >&2
#
# Colors are emitted **only when stdout is a TTY and `NO_COLOR` is
# unset**. In all other cases — piped-to-less, redirected-to-file,
# captured-by-CI, `NO_COLOR=1`-set — the variables expand to the
# empty string, so callers produce clean plain-text output without
# any conditional logic of their own.
#
# This is the single source of truth for "is color on?" across
# the workspace. Scripts that previously inlined
# `printf '\033[...m'` constants (setup.sh, codex-status.sh)
# migrate here so the TTY/NO_COLOR gate is uniform.
#
# Detection:
#
# - `[ -t 1 ]` — POSIX test for "fd 1 (stdout) refers to a
#   terminal." The standard and portable way to detect TTY.
# - `NO_COLOR` — de facto standard environment variable
#   (https://no-color.org/). Any non-empty value disables
#   color; an unset or empty value leaves it enabled.
#
# Variables exported (all strings — either an ANSI sequence or
# the empty string, so they're safe to `printf` unconditionally):
#
# - `C_RESET`  — SGR 0, returns rendering to default.
# - `C_BOLD`   — SGR 1, bold.
# - `C_DIM`    — SGR 2, dim.
# - `C_OK`     — SGR 32, green. Use for success verdicts.
# - `C_WARN`   — SGR 33, yellow. Use for warnings / attention.
# - `C_ERR`    — SGR 31, red. Use for errors / aborts.
# - `C_NOTE`   — SGR 36, cyan. Use for incidental info / file paths.
# - `C_HEADER` — SGR 34, blue. Use for section headers (`=== ... ===`).
#
# Scripts that emit machine-readable output on stdout
# (`show-dirty.sh`, `crate-version.sh`, `mktemp.sh`) MUST NOT
# source this file — the TTY check would correctly keep colors
# off for their usual callers, but sourcing it still adds
# shell-level noise where none is appropriate.
#
# POSIX sh — see CONTRIBUTING.md §6.

if [ -t 1 ] && [ -z "${NO_COLOR:-}" ]; then
    C_RESET=$(printf '\033[0m')
    C_BOLD=$(printf '\033[1m')
    C_DIM=$(printf '\033[2m')
    C_OK=$(printf '\033[32m')
    C_WARN=$(printf '\033[33m')
    C_ERR=$(printf '\033[31m')
    C_NOTE=$(printf '\033[36m')
    C_HEADER=$(printf '\033[34m')
else
    C_RESET=
    C_BOLD=
    C_DIM=
    C_OK=
    C_WARN=
    C_ERR=
    C_NOTE=
    C_HEADER=
fi

# Exported so `git submodule foreach`, `trap` subshells, and other
# child-shell contexts within the same script see the same values.
# Variables are either ANSI sequences or empty strings, so export
# is safe even in the color-off path.
export C_RESET C_BOLD C_DIM C_OK C_WARN C_ERR C_NOTE C_HEADER
