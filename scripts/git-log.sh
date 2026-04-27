#!/bin/sh
# Pretty-print the workspace git log with DCO sign-off and GPG/SSH
# signature status per commit. Default target is the parent
# workspace repo; pass a submodule path (relative to the workspace
# root) as a positional argument to inspect that submodule's own
# history instead. Default count: last 500 commits; override with
# `-n <N>` or `--count <N>`.
#
# The script sources scripts/lib/workspace-cd.sh, which cd's to
# the workspace root before anything runs, so the submodule path
# is always resolved relative to that root regardless of where
# the script was invoked from.
#
# Columns:
#   <short-sha>  <YYYY-MM-DD>  [<%G?>]  [<sign-off-label>]  <author>  |  <subject>
#
# Signature status (%G?):
#   G = good
#   U = good, untrusted key
#   B = bad
#   X = good but expired
#   Y = good but key expiring
#   R = good but key revoked
#   E = cannot check (missing key, etc.)
#   N = no signature
#
# Sign-off label:
#   [signed-off]       — a Signed-off-by: trailer matches the author
#                        email (%ae).
#   [unknown sign-off] — commit has Signed-off-by: trailer(s) but none
#                        match the author email (imported patch,
#                        co-author-only sign-off, etc.).
#   [NOT signed-off]   — no Signed-off-by: trailer at all; violates
#                        the DCO rule (CONTRIBUTING.md §4.3).
#
# Typical post-filter — audit the entire workspace for commits that
# escaped the sign-off / signature invariants:
#
#   ./scripts/git-log.sh | grep -E '\[(N|NOT signed-off)\]'
#   for sub in philharmonic-types mechanics-core ...; do
#       ./scripts/git-log.sh "$sub" | grep -E '\[(N|NOT signed-off)\]'
#   done
#
# Requires git >= 2.32 (uses `valueonly=true` and `separator=%x1f`
# on `%(trailers:key=...)`, added in 2.32). POSIX sh — see
# CONTRIBUTING.md §6.
#
# Colors: ANSI sequences are emitted only when stdout is a TTY and
# the de-facto-standard `NO_COLOR` env var is unset. Pass
# `--no-color` to force plain output regardless. See
# scripts/lib/colors.sh for the shared TTY/NO_COLOR gate.

set -eu

usage() {
    cat <<'EOF'
Usage: git-log.sh [-n <N> | --count <N>] [--no-color] [<submodule-path>] [-h|--help]

Pretty-print workspace git log with DCO sign-off + GPG/SSH
signature status per commit.

Positional:
  <submodule-path>     Path to a submodule (relative to the
                       workspace root) whose history to display.
                       Omit to target the parent workspace repo.

Columns:
  <short-sha>  <YYYY-MM-DD>  [<%G?>]  [<sign-off-label>]  <author>  |  <subject>

Signature status (%G?):
  G = good                 U = good, untrusted key
  B = bad                  X = good but expired
  Y = good but expiring    R = good but revoked
  E = cannot check         N = no signature

Sign-off label:
  [signed-off]        A Signed-off-by: trailer matches the author email.
  [unknown sign-off]  Commit has Signed-off-by trailers but none match
                      the author email (imported patch, co-author, etc.).
  [NOT signed-off]    No Signed-off-by: trailer — violates the DCO rule.

Options:
  -n <N>, --count <N>  Show the last N commits (default 500). N must be
                       a positive integer.
  --no-color           Disable ANSI color output. By default colors are
                       on when stdout is a TTY and NO_COLOR is unset.
  -h, --help           Show this help and exit.

Examples:
  ./scripts/git-log.sh
  ./scripts/git-log.sh -n 50
  ./scripts/git-log.sh mechanics-core
  ./scripts/git-log.sh -n 200 philharmonic-types
  ./scripts/git-log.sh --no-color | grep -E '\[(N|NOT signed-off)\]'
EOF
}

count=500
target=""
no_color=0

# Positive-integer validation (no leading zero, no sign, non-empty).
is_positive_int() {
    case "$1" in
        ''|*[!0-9]*) return 1 ;;
        0*)          return 1 ;;
        *)           return 0 ;;
    esac
}

while [ $# -gt 0 ]; do
    case "$1" in
        -h|--help)
            usage; exit 0 ;;
        -n|--count)
            if [ $# -lt 2 ]; then
                printf 'git-log.sh: %s requires an argument\n\n' "$1" >&2
                usage >&2
                exit 2
            fi
            if ! is_positive_int "$2"; then
                printf 'git-log.sh: %s argument must be a positive integer, got: %s\n\n' "$1" "$2" >&2
                usage >&2
                exit 2
            fi
            count="$2"
            shift 2 ;;
        --no-color)
            no_color=1
            shift ;;
        --)
            shift; break ;;
        -*)
            printf 'git-log.sh: unknown flag: %s\n\n' "$1" >&2
            usage >&2
            exit 2 ;;
        *)
            if [ -n "$target" ]; then
                printf 'git-log.sh: unexpected extra argument: %s (already targeting %s)\n\n' "$1" "$target" >&2
                usage >&2
                exit 2
            fi
            target="$1"
            shift ;;
    esac
done

# Remaining positional args after `--` (if any) are unsupported.
if [ $# -gt 0 ]; then
    if [ -n "$target" ] || [ $# -gt 1 ]; then
        printf 'git-log.sh: unexpected extra argument(s) after --: %s\n\n' "$*" >&2
        usage >&2
        exit 2
    fi
    target="$1"
fi

# `--no-color` forces colors off via the shared `NO_COLOR` gate in
# lib/colors.sh; otherwise the helper auto-detects (TTY + unset
# NO_COLOR). The helper exports empty strings when off, so awk can
# concatenate the C_* vars unconditionally.
if [ "$no_color" -eq 1 ]; then
    NO_COLOR=1
    export NO_COLOR
fi
. "$(dirname -- "$0")/lib/colors.sh"
. "$(dirname -- "$0")/lib/workspace-cd.sh"

# Validate and resolve the optional submodule path. `git -C` takes
# the path as-is; we just need to make sure it names a real git-repo
# root (not a subdirectory of the parent, not a non-git directory,
# not a missing path).
if [ -n "$target" ]; then
    if [ ! -d "$target" ]; then
        printf 'git-log.sh: directory not found: %s\n' "$target" >&2
        exit 2
    fi
    if ! top=$(git -C "$target" rev-parse --show-toplevel 2>/dev/null); then
        printf 'git-log.sh: not a git repository: %s\n' "$target" >&2
        exit 2
    fi
    # Reject passing a subdirectory of the parent: if we resolve
    # inside the parent working tree (not at a submodule root),
    # `rev-parse --show-toplevel` returns the parent's root, not
    # the path the user passed. Compare absolute paths.
    abs_target=$(cd "$target" && pwd -P)
    if [ "$top" != "$abs_target" ]; then
        printf 'git-log.sh: %s is not a git-repo root (submodule expected); its rev-parse --show-toplevel resolved to %s\n' \
            "$target" "$top" >&2
        exit 2
    fi
fi

git -C "${target:-.}" log -n "$count" --date=short \
  --pretty=tformat:'%h%x09%ad%x09%G?%x09%(trailers:key=Signed-off-by,valueonly=true,separator=%x1f)%x09%ae%x09%an <%ae>%x09%s' \
  | awk -F '\t' \
        -v ok="$C_OK" -v warn="$C_WARN" -v err="$C_ERR" \
        -v note="$C_NOTE" -v hdr="$C_HEADER" -v reset="$C_RESET" '
    {
        hash = $1; date = $2; sig = $3; sob_raw = $4
        author_email = $5; author = $6; subject = $7

        if (sob_raw == "") {
            sob_label = "[NOT signed-off]"
            sob_color = err
        } else {
            n = split(sob_raw, sobs, "\037")
            matched = 0
            for (i = 1; i <= n; i++) {
                if (index(sobs[i], "<" author_email ">") > 0) {
                    matched = 1
                    break
                }
            }
            sob_label = matched ? "[signed-off]"        : "[unknown sign-off]"
            sob_color = matched ? ok                    : warn
        }

        # Signature status colors:
        #   G                       → ok    (green)
        #   U / X / Y               → warn  (yellow — valid-but-suspect)
        #   B / R / E / N / other   → err   (red)
        if      (sig == "G")                              sig_color = ok
        else if (sig == "U" || sig == "X" || sig == "Y") sig_color = warn
        else                                              sig_color = err

        # Color choices per column:
        #   short SHA    → header (blue, mirrors `git log` default-ish)
        #   date         → ok     (green, mirrors `git log` default)
        #   author       → note   (cyan)
        #   subject      → default
        # When colors are off, all C_* vars are empty strings and the
        # output collapses to the original plain-text form.
        printf "%s%s%s %s%s%s %s[%s]%s %s%s%s %s%s%s | %s\n", \
            hdr, hash, reset, \
            ok, date, reset, \
            sig_color, sig, reset, \
            sob_color, sob_label, reset, \
            note, author, reset, \
            subject
    }'
