#!/bin/sh
# scripts/audit-log.sh — display the workspace git log in a
# compliance-audit-friendly format. Each line carries the commit
# hash, ISO timestamp, signature and sign-off verdicts, author
# identity, diffstat, and the Audit-Info trailer recorded by
# commit-all.sh.
#
# Usage:
#   ./scripts/audit-log.sh                       # workspace, last 200
#   ./scripts/audit-log.sh -n 50                 # last 50
#   ./scripts/audit-log.sh philharmonic-types     # submodule
#   ./scripts/audit-log.sh --no-color | less      # pipe-friendly
#
# Output template (one line per commit):
#   <hash> <ISO> [<SIG>] [<sign-off>] <author> | -<del> +<ins> | <audit-info>
#
# Signature status (%G?):
#   G = good   U = good/untrusted   B = bad   X = expired
#   Y = expiring   R = revoked   E = can't check   N = none
#
# Sign-off label:
#   [signed-off]        Signed-off-by matches author email.
#   [unknown sign-off]  Signed-off-by present but no match.
#   [NOT signed-off]    No Signed-off-by trailer at all.
#
# POSIX sh only — see CONTRIBUTING.md §6.

set -eu

usage() {
    cat <<'EOF'
Usage: audit-log.sh [-n <N>] [--no-color] [<submodule-path>] [-h|--help]

Display workspace git history in compliance-audit format.

Output template:
  <hash> <ISO> [<SIG>] [<sign-off>] <author> | -<del> +<ins> | <audit-info>

Options:
  -n <N>          Show the last N commits (default 200).
  --no-color      Disable ANSI color output.
  -h, --help      Show this help and exit.
EOF
}

count=200
target=""
no_color=0

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
        -n)
            if [ $# -lt 2 ]; then
                printf 'audit-log.sh: -n requires an argument\n' >&2
                exit 2
            fi
            if ! is_positive_int "$2"; then
                printf 'audit-log.sh: -n argument must be a positive integer, got: %s\n' "$2" >&2
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
            printf 'audit-log.sh: unknown flag: %s\n' "$1" >&2
            exit 2 ;;
        *)
            if [ -n "$target" ]; then
                printf 'audit-log.sh: unexpected extra argument: %s\n' "$1" >&2
                exit 2
            fi
            target="$1"
            shift ;;
    esac
done

if [ $# -gt 0 ]; then
    if [ -n "$target" ]; then
        printf 'audit-log.sh: unexpected extra argument: %s\n' "$1" >&2
        exit 2
    fi
    target="$1"
fi

if [ "$no_color" -eq 1 ]; then
    NO_COLOR=1
    export NO_COLOR
fi
. "$(dirname -- "$0")/lib/colors.sh"
. "$(dirname -- "$0")/lib/workspace-cd.sh"

if [ -n "$target" ]; then
    if [ ! -d "$target" ]; then
        printf 'audit-log.sh: directory not found: %s\n' "$target" >&2
        exit 2
    fi
    if ! top=$(git -C "$target" rev-parse --show-toplevel 2>/dev/null); then
        printf 'audit-log.sh: not a git repository: %s\n' "$target" >&2
        exit 2
    fi
    abs_target=$(cd "$target" && pwd -P)
    if [ "$top" != "$abs_target" ]; then
        printf 'audit-log.sh: %s is not a git-repo root\n' "$target" >&2
        exit 2
    fi
fi

# Format: tab-separated fields.
# Field 1: short hash
# Field 2: ISO 8601 date to seconds
# Field 3: signature status (%G?)
# Field 4: Signed-off-by trailer(s), \x1f-separated
# Field 5: author email (for sign-off matching)
# Field 6: author display ("Name <email>")
# Field 7: Audit-Info trailer(s), \x1f-separated
#
# --shortstat appends a blank line then a stat summary line after
# each formatted entry. The awk below merges them.

git -C "${target:-.}" log -n "$count" --date=iso-strict \
  --pretty=tformat:'%h%x09%ad%x09%G?%x09%(trailers:key=Signed-off-by,valueonly=true,separator=%x1f)%x09%ae%x09%an <%ae>%x09%(trailers:key=Audit-Info,valueonly=true,separator=%x1f)' \
  --shortstat \
  | awk -F '\t' \
        -v ok="$C_OK" -v warn="$C_WARN" -v err="$C_ERR" \
        -v note="$C_NOTE" -v hdr="$C_HEADER" -v dim="$C_DIM" \
        -v reset="$C_RESET" '
    # Skip blank lines between format and shortstat output.
    /^[[:space:]]*$/ { next }

    # Stat line: " N file(s) changed, ..." — parse and attach to
    # the buffered commit.
    /changed/ {
        ins = 0; del = 0
        for (i = 1; i <= NF; i++) {
            if ($i ~ /insertion/) { ins = $(i-1) + 0 }
            if ($i ~ /deletion/)  { del = $(i-1) + 0 }
        }
        if (buffered) {
            emit(del, ins)
            buffered = 0
        }
        next
    }

    # Format line (contains tabs → awk splits into fields).
    NF >= 6 {
        # Flush any previously buffered commit that had no shortstat
        # (e.g. empty commits, merge commits without diff).
        if (buffered) { emit(0, 0) }

        b_hash = $1; b_iso = $2; b_sig = $3; b_sob_raw = $4
        b_email = $5; b_author = $6
        b_audit = (NF >= 7) ? $7 : ""

        # Sign-off status.
        if (b_sob_raw == "") {
            b_sob_label = "[NOT signed-off]"
            b_sob_color = err
        } else {
            n = split(b_sob_raw, sobs, "\037")
            matched = 0
            for (i = 1; i <= n; i++) {
                if (index(sobs[i], "<" b_email ">") > 0) {
                    matched = 1; break
                }
            }
            b_sob_label = matched ? "[signed-off]" : "[unknown sign-off]"
            b_sob_color = matched ? ok : warn
        }

        # Signature color.
        if      (b_sig == "G")                                b_sig_color = ok
        else if (b_sig == "U" || b_sig == "X" || b_sig == "Y") b_sig_color = warn
        else                                                   b_sig_color = err

        buffered = 1
    }

    function emit(del, ins) {
        # Truncate ISO to seconds (remove sub-second + timezone offset
        # for compactness — the Audit-Info trailer carries the Unix
        # epoch for canonical time).
        iso = b_iso
        sub(/\+.*$/, "", iso)
        sub(/-[0-9][0-9]:[0-9][0-9]$/, "", iso)

        audit = b_audit
        if (audit == "") audit = "-"

        printf "%s%s%s %s%s%s %s[%s]%s %s%s%s %s%s%s | %s-%d +%d%s | %s%s%s\n", \
            hdr, b_hash, reset, \
            dim, iso, reset, \
            b_sig_color, b_sig, reset, \
            b_sob_color, b_sob_label, reset, \
            note, b_author, reset, \
            dim, del, ins, reset, \
            dim, audit, reset
    }

    END {
        if (buffered) { emit(0, 0) }
    }'
