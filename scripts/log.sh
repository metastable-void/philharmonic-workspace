#!/bin/sh
# scripts/log.sh — unified pretty-printed git-log front-end.
#
# Three modes, picked by exactly one of `--history` (default),
# `--audit`, or `--stats`. Each replaces a previously-separate
# script in this directory; the per-mode column shapes and
# defaults are preserved so existing callers (CI, agents,
# `update-stats-graph.sh`, `project-status.sh`, etc.) keep
# working unchanged once their invocations are migrated to
# `log.sh --<mode>`.
#
# Modes:
#
#   --history (default; replaces retired `git-log.sh`)
#       Columns: <short-sha> <YYYY-MM-DD> [<%G?>]
#                [<sign-off-label>] <author> | <subject>
#       Default count: 500.
#
#   --audit   (replaces retired `audit-log.sh`)
#       Columns: <short-sha> <ISO-to-seconds> [<%G?>]
#                [<sign-off-label>] <author>
#                | -<deletions> +<insertions>
#                | <Audit-Info trailer body>
#       Default count: 200.
#       Uses `git log --shortstat` to gather diffstats.
#
#   --stats   (replaces retired `stats-log.sh`)
#       Columns: <short-sha> <ISO-with-tz> <author>
#                | <files>F <lines>L (<code>C <docs>D)
#                | Δ +<files>F +<lines>L (+<code>C +<docs>D)
#       Default count: 200.
#       Falls back to `docs/stats-cache.tsv` when a commit's
#       `Code-stats:` trailer is absent (parent-repo only —
#       submodule views skip the cache because the SHAs do not
#       match). Stops scrubbing the iso-strict timezone offset
#       (`Z` / `+09:00` / `-HH:MM`) so `stats-graph` (xtask)
#       can RFC-3339-parse every plotted commit.
#
# Common to every mode:
#
#   - Default target is the parent workspace repo. Pass a
#     submodule path (relative to the workspace root) as a
#     positional argument to inspect that submodule's own
#     history. The path is validated against `git rev-parse
#     --show-toplevel` so subdirectories of the parent
#     (which share its history) are rejected.
#   - `-n <N>` (alias `--count <N>`, retained from
#     git-log.sh) overrides the per-mode default count. N
#     must be a positive integer.
#   - `--no-color` forces ANSI off via the shared `NO_COLOR`
#     gate in lib/colors.sh; otherwise colors auto-detect
#     (TTY + unset NO_COLOR).
#   - Sign-off label semantics:
#         [signed-off]        Signed-off-by trailer matches author email.
#         [unknown sign-off]  Trailers exist; none match (imported
#                             patch / co-author-only sign-off).
#         [NOT signed-off]    No Signed-off-by trailer at all
#                             (DCO violation, see CONTRIBUTING.md §4.3).
#   - Signature column (%G?) colors:
#         G                       → ok    (green)
#         U / X / Y               → warn  (yellow — valid-but-suspect)
#         B / R / E / N / other   → err   (red)
#
# Requires git ≥ 2.32 (uses `valueonly=true` and
# `separator=%x1f` on `%(trailers:key=…)`, added in 2.32).
# POSIX sh — see CONTRIBUTING.md §6.

set -eu

. "$(dirname -- "$0")/lib/script-help.sh"
script_help_handle "$@"


usage() {
    cat <<'EOF'
Usage: log.sh [--history | --audit | --stats]
              [-n <N> | --count <N>] [--no-color]
              [<submodule-path>] [-h | --help]

Modes (exactly one; --history if omitted):
  --history          Default. Short SHA + date + signature + sign-off + author | subject.
  --audit            Audit-Info trailer + diffstat + signature + sign-off.
  --stats            Code-stats trailer + per-commit delta (cache-backed).

Options:
  -n <N>, --count <N>  Last N commits (default 500 for --history; 200 otherwise).
  --no-color           Disable ANSI color output.
  <submodule-path>     Optional: target a submodule rather than the parent.
  -h, --help           Show this help and exit.

Examples:
  ./scripts/log.sh
  ./scripts/log.sh --audit -n 50
  ./scripts/log.sh --stats -n 1000 --no-color
  ./scripts/log.sh --history mechanics-core
  ./scripts/log.sh --no-color | grep -E '\[(N|NOT signed-off)\]'
EOF
}

mode=
count=
target=""
no_color=0

is_positive_int() {
    case "$1" in
        ''|*[!0-9]*) return 1 ;;
        0*)          return 1 ;;
        *)           return 0 ;;
    esac
}

set_mode() {
    if [ -n "$mode" ] && [ "$mode" != "$1" ]; then
        printf 'log.sh: --history / --audit / --stats are mutually exclusive\n\n' >&2
        usage >&2
        exit 2
    fi
    mode="$1"
}

while [ $# -gt 0 ]; do
    case "$1" in
        -h|--help)
            usage; exit 0 ;;
        --history) set_mode history; shift ;;
        --audit)   set_mode audit;   shift ;;
        --stats)   set_mode stats;   shift ;;
        -n|--count)
            if [ $# -lt 2 ]; then
                printf 'log.sh: %s requires an argument\n\n' "$1" >&2
                usage >&2
                exit 2
            fi
            if ! is_positive_int "$2"; then
                printf 'log.sh: %s argument must be a positive integer, got: %s\n\n' "$1" "$2" >&2
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
            printf 'log.sh: unknown flag: %s\n\n' "$1" >&2
            usage >&2
            exit 2 ;;
        *)
            if [ -n "$target" ]; then
                printf 'log.sh: unexpected extra argument: %s (already targeting %s)\n\n' "$1" "$target" >&2
                usage >&2
                exit 2
            fi
            target="$1"
            shift ;;
    esac
done

# Trailing positional after `--`.
if [ $# -gt 0 ]; then
    if [ -n "$target" ] || [ $# -gt 1 ]; then
        printf 'log.sh: unexpected extra argument(s) after --: %s\n\n' "$*" >&2
        usage >&2
        exit 2
    fi
    target="$1"
fi

# Apply defaults: --history is the implicit default mode; per-mode count.
[ -n "$mode" ] || mode=history
if [ -z "$count" ]; then
    case "$mode" in
        history) count=500 ;;
        audit|stats) count=200 ;;
    esac
fi

if [ "$no_color" -eq 1 ]; then
    NO_COLOR=1
    export NO_COLOR
fi
. "$(dirname -- "$0")/lib/colors.sh"
. "$(dirname -- "$0")/lib/workspace-cd.sh"

# Validate optional submodule target. `git -C` takes the path as-is;
# we just need to make sure it names a real git-repo root, not a
# subdirectory of the parent (which would silently share its history).
if [ -n "$target" ]; then
    if [ ! -d "$target" ]; then
        printf 'log.sh: directory not found: %s\n' "$target" >&2
        exit 2
    fi
    if ! top=$(git -C "$target" rev-parse --show-toplevel 2>/dev/null); then
        printf 'log.sh: not a git repository: %s\n' "$target" >&2
        exit 2
    fi
    abs_target=$(cd "$target" && pwd -P)
    if [ "$top" != "$abs_target" ]; then
        printf 'log.sh: %s is not a git-repo root (submodule expected); its rev-parse --show-toplevel resolved to %s\n' \
            "$target" "$top" >&2
        exit 2
    fi
fi

case "$mode" in

history)
    # Columns: <short-sha> <YYYY-MM-DD> [<%G?>] [<sign-off>] <author> | <subject>
    TZ=Asia/Tokyo git -C "${target:-.}" log -n "$count" --date=short \
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

            if      (sig == "G")                              sig_color = ok
            else if (sig == "U" || sig == "X" || sig == "Y") sig_color = warn
            else                                              sig_color = err

            printf "%s%s%s %s%s%s %s[%s]%s %s%s%s %s%s%s | %s\n", \
                hdr, hash, reset, \
                ok, date, reset, \
                sig_color, sig, reset, \
                sob_color, sob_label, reset, \
                note, author, reset, \
                subject
        }'
    ;;

audit)
    # Columns: <short-sha> <ISO-to-seconds> [<%G?>] [<sign-off>] <author>
    #          | -<del> +<ins> | <Audit-Info trailer>
    # `--shortstat` appends a blank line then a stat summary line
    # after each formatted entry; the awk merges them.
    TZ=Asia/Tokyo git -C "${target:-.}" log -n "$count" --date=iso-strict \
      --pretty=tformat:'%h%x09%ad%x09%G?%x09%(trailers:key=Signed-off-by,valueonly=true,separator=%x1f)%x09%ae%x09%an <%ae>%x09%(trailers:key=Audit-Info,valueonly=true,separator=%x1f)' \
      --shortstat \
      | awk -F '\t' \
            -v ok="$C_OK" -v warn="$C_WARN" -v err="$C_ERR" \
            -v note="$C_NOTE" -v hdr="$C_HEADER" -v dim="$C_DIM" \
            -v reset="$C_RESET" '
        # Skip blank lines between format and shortstat output.
        /^[[:space:]]*$/ { next }

        # Stat line: " N file(s) changed, ..." — parse and attach
        # to the buffered commit.
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

        NF >= 6 {
            # Flush any previously buffered commit that had no shortstat
            # (e.g. empty commits, merges without diff).
            if (buffered) { emit(0, 0) }

            b_hash = $1; b_iso = $2; b_sig = $3; b_sob_raw = $4
            b_email = $5; b_author = $6
            b_audit = (NF >= 7) ? $7 : ""

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

            if      (b_sig == "G")                                b_sig_color = ok
            else if (b_sig == "U" || b_sig == "X" || b_sig == "Y") b_sig_color = warn
            else                                                   b_sig_color = err

            buffered = 1
        }

        function emit(del, ins) {
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
    ;;

stats)
    # Columns: <short-sha> <ISO-with-tz> <author>
    #          | <files>F <lines>L (<code>C <docs>D)
    #          | Δ +<files>F +<lines>L (+<code>C +<docs>D)
    #
    # Cache fallback: parent-repo only. Submodule views skip the
    # cache because the SHAs do not match.
    cache_arg=""
    if [ -z "$target" ] && [ -f docs/stats-cache.tsv ]; then
        cache_arg="docs/stats-cache.tsv"
    fi

    # Pull `count + 1` commits so the oldest displayed commit can
    # still compute a delta against its predecessor.
    fetch=$(( count + 1 ))

    TZ=Asia/Tokyo git -C "${target:-.}" log -n "$fetch" --date=iso-strict \
      --pretty=tformat:'%h%x09%H%x09%ad%x09%an <%ae>%x09%(trailers:key=Code-stats,valueonly=true)' \
      | awk -F '\t' \
            -v keep="$count" \
            -v cache="$cache_arg" \
            -v ok="$C_OK" -v warn="$C_WARN" -v err="$C_ERR" \
            -v note="$C_NOTE" -v hdr="$C_HEADER" -v dim="$C_DIM" \
            -v reset="$C_RESET" '
        BEGIN {
            # Load the sidecar cache (`docs/stats-cache.tsv`) into a
            # full-SHA → Code-stats-equivalent-string map, so the
            # parse() function below can consume it without caring
            # whether the data came from a trailer or the cache.
            # Comment lines (`#`) and unreproducible markers are
            # skipped.
            if (cache != "") {
                while ((getline line < cache) > 0) {
                    if (line ~ /^#/) continue
                    n = split(line, parts, "\t")
                    if (n >= 5 && parts[1] != "" && parts[2] != "") {
                        cache_map[parts[1]] = sprintf( \
                            "%s files, %s total lines (%s code lines, %s docs lines)", \
                            parts[2], parts[3], parts[4], parts[5])
                    }
                }
                close(cache)
            }
        }

        /^[[:space:]]*$/ { next }

        {
            n_rows++
            hash[n_rows]   = $1
            iso[n_rows]    = $3
            author[n_rows] = $4
            stats[n_rows]  = $5
            if (stats[n_rows] == "" && ($2 in cache_map)) {
                stats[n_rows] = cache_map[$2]
            }
        }

        function parse(s, out) {
            delete out
            out["files"] = 0; out["lines"] = 0
            out["code"]  = 0; out["docs"]  = 0
            out["have"]  = 0
            if (s == "") return
            if (match(s, /[0-9]+ files,[[:space:]]+[0-9]+ total lines[[:space:]]+\([0-9]+ code lines,[[:space:]]+[0-9]+ docs lines\)/) == 0) return
            run = substr(s, RSTART, RLENGTH)
            n = 0
            tmp = run
            while (match(tmp, /[0-9]+/) > 0) {
                n++
                nums[n] = substr(tmp, RSTART, RLENGTH) + 0
                tmp = substr(tmp, RSTART + RLENGTH)
            }
            if (n < 4) return
            out["files"] = nums[1]
            out["lines"] = nums[2]
            out["code"]  = nums[3]
            out["docs"]  = nums[4]
            out["have"]  = 1
        }

        function sgnstr(n) {
            if (n > 0) return sprintf("+%d", n)
            return sprintf("%d", n)
        }

        END {
            total = n_rows + 0
            out_n = (total < keep) ? total : keep

            for (i = 1; i <= out_n; i++) {
                parse(stats[i],   curr)
                parse(stats[i+1], prev)

                # Keep the iso-strict timezone offset intact (`Z` for
                # UTC, `+HH:MM` / `-HH:MM` otherwise). The stats-graph
                # xtask bin parses this output via the chrono RFC 3339
                # path, which requires a timezone — stripping it
                # silently drops every plot point for commits whose
                # author/committer offset is not `Z`.
                iso_t = iso[i]

                if (curr["have"]) {
                    stats_str = sprintf("%dF %dL (%dC %dD)",
                        curr["files"], curr["lines"],
                        curr["code"],  curr["docs"])
                } else {
                    stats_str = "-"
                }

                if (curr["have"] && prev["have"]) {
                    df = curr["files"] - prev["files"]
                    dl = curr["lines"] - prev["lines"]
                    dc = curr["code"]  - prev["code"]
                    dd = curr["docs"]  - prev["docs"]
                    if      (dl > 0) dcolor = note
                    else if (dl < 0) dcolor = ok
                    else             dcolor = dim
                    delta_label = sprintf("Δ %sF %sL (%sC %sD)",
                        sgnstr(df), sgnstr(dl),
                        sgnstr(dc), sgnstr(dd))
                } else {
                    dcolor = dim
                    delta_label = "Δ -"
                }

                printf "%s%s%s %s%s%s %s%s%s | %s%s%s | %s%s%s\n", \
                    hdr, hash[i], reset, \
                    dim, iso_t, reset, \
                    note, author[i], reset, \
                    (curr["have"] ? "" : dim), stats_str, reset, \
                    dcolor, delta_label, reset
            }
        }'
    ;;

esac
