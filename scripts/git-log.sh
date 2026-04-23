#!/bin/sh
# Pretty-print the workspace git log with DCO sign-off and GPG/SSH
# signature status per commit. Parent workspace repo only (the
# script sources scripts/lib/workspace-cd.sh, which cd's to the
# workspace root before running). Default: last 500 commits;
# override with `-n <N>` or `--count <N>`.
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
# Typical post-filter:
#   ./scripts/git-log.sh | grep -E '\[(N|NOT signed-off)\]'
#
# Requires git >= 2.32 (uses `valueonly=true` and `separator=%x1f`
# on `%(trailers:key=...)`, added in 2.32). POSIX sh — see
# CONTRIBUTING.md §6.

set -eu

usage() {
    cat <<'EOF'
Usage: git-log.sh [-n <N> | --count <N>] [-h|--help]

Pretty-print workspace git log with DCO sign-off + GPG/SSH
signature status per commit (parent repo only).

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
EOF
}

count=500

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
            printf 'git-log.sh: unexpected argument: %s\n\n' "$1" >&2
            usage >&2
            exit 2 ;;
    esac
done

. "$(dirname -- "$0")/lib/workspace-cd.sh"

git log -n "$count" --date=short \
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
