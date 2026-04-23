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

set -eu

usage() {
    cat <<'EOF'
Usage: git-log.sh [-n <N> | --count <N>] [<submodule-path>] [-h|--help]

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
  -h, --help           Show this help and exit.

Examples:
  ./scripts/git-log.sh
  ./scripts/git-log.sh -n 50
  ./scripts/git-log.sh mechanics-core
  ./scripts/git-log.sh -n 200 philharmonic-types
EOF
}

count=500
target=""

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
  | awk -F '\t' '
    {
        hash = $1; date = $2; sig = $3; sob_raw = $4
        author_email = $5; author = $6; subject = $7

        if (sob_raw == "") {
            sob_label = "[NOT signed-off]"
        } else {
            n = split(sob_raw, sobs, "\037")
            matched = 0
            for (i = 1; i <= n; i++) {
                if (index(sobs[i], "<" author_email ">") > 0) {
                    matched = 1
                    break
                }
            }
            sob_label = matched ? "[signed-off]" : "[unknown sign-off]"
        }

        printf "%s %s [%s] %s %s | %s\n", hash, date, sig, sob_label, author, subject
    }'
