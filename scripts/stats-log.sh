#!/bin/sh
# scripts/stats-log.sh — display the workspace git log with the
# Code-stats trailer recorded by commit-all.sh, plus a per-commit
# delta against the immediate predecessor.
#
# Mirrors audit-log.sh's command shape and styling; differs only
# in which trailer is shown and in computing deltas.
#
# Usage:
#   ./scripts/stats-log.sh                       # workspace, last 200
#   ./scripts/stats-log.sh -n 50                 # last 50
#   ./scripts/stats-log.sh philharmonic-types    # submodule
#   ./scripts/stats-log.sh --no-color | less     # pipe-friendly
#
# Output template (one line per commit):
#   <hash> <ISO> <author> | <files>F <lines>L (<code>C <docs>D) | Δ +<files>F +<lines>L (+<code>C +<docs>D)
#
# Commits without a Code-stats trailer (older than the trailer
# adoption) fall back to the sidecar cache at
# `docs/stats-cache.tsv` (populated by `scripts/backfill-stats.sh`);
# rows still missing from the cache print `-` for stats and `Δ -`
# for the delta. Cache fallback is parent-repo only — when a
# submodule path is passed, no cache is consulted (parent SHAs
# don't match submodule SHAs).
#
# Delta convention: each commit's delta is `current - immediate
# predecessor` when that predecessor is available in the fetched
# history. If either side has no Code-stats data (trailer or
# cached), the delta is `Δ -`.
#
# POSIX sh only — see CONTRIBUTING.md §6.

set -eu

usage() {
    cat <<'EOF'
Usage: stats-log.sh [-n <N>] [--no-color] [<submodule-path>] [-h|--help]

Display workspace git history with Code-stats trailer + per-commit deltas.

Output template:
  <hash> <ISO> <author> | <files>F <lines>L (<code>C <docs>D) | Δ +<files>F +<lines>L (+<code>C +<docs>D)

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
                printf 'stats-log.sh: -n requires an argument\n' >&2
                exit 2
            fi
            if ! is_positive_int "$2"; then
                printf 'stats-log.sh: -n argument must be a positive integer, got: %s\n' "$2" >&2
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
            printf 'stats-log.sh: unknown flag: %s\n' "$1" >&2
            exit 2 ;;
        *)
            if [ -n "$target" ]; then
                printf 'stats-log.sh: unexpected extra argument: %s\n' "$1" >&2
                exit 2
            fi
            target="$1"
            shift ;;
    esac
done

if [ $# -gt 0 ]; then
    if [ -n "$target" ]; then
        printf 'stats-log.sh: unexpected extra argument: %s\n' "$1" >&2
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
        printf 'stats-log.sh: directory not found: %s\n' "$target" >&2
        exit 2
    fi
    if ! top=$(git -C "$target" rev-parse --show-toplevel 2>/dev/null); then
        printf 'stats-log.sh: not a git repository: %s\n' "$target" >&2
        exit 2
    fi
    abs_target=$(cd "$target" && pwd -P)
    if [ "$top" != "$abs_target" ]; then
        printf 'stats-log.sh: %s is not a git-repo root\n' "$target" >&2
        exit 2
    fi
fi

# Format: tab-separated fields.
# Field 1: short hash
# Field 2: full hash (used to look up sidecar cache)
# Field 3: ISO 8601 date to seconds (TZ=Asia/Tokyo)
# Field 4: author display ("Name <email>")
# Field 5: Code-stats trailer (single line; empty if absent)
#
# We pull `count + 1` commits so the oldest displayed commit can
# still compute a delta against its predecessor when enough
# history exists. The awk below drops the extra trailing commit
# from output.

fetch=$(( count + 1 ))

# Cache fallback path: parent-repo only. Submodule views skip
# the cache entirely because their SHAs don't match.
cache_arg=""
if [ -z "$target" ] && [ -f docs/stats-cache.tsv ]; then
    cache_arg="docs/stats-cache.tsv"
fi

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

    # Skip blank lines that the trailer placeholder leaves
    # between commits in tformat output.
    /^[[:space:]]*$/ { next }

    # Buffer all rows. Newest is row 1.
    {
        n_rows++
        hash[n_rows]   = $1
        iso[n_rows]    = $3
        author[n_rows] = $4
        stats[n_rows]  = $5
        # Cache fallback when the trailer is absent.
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
        # Match: "<F> files, <L> total lines (<C> code lines, <D> docs lines)"
        if (match(s, /[0-9]+ files,[[:space:]]+[0-9]+ total lines[[:space:]]+\([0-9]+ code lines,[[:space:]]+[0-9]+ docs lines\)/) == 0) return
        run = substr(s, RSTART, RLENGTH)
        # Pull integers in order.
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

            # Keep the iso-strict timezone offset intact: `Z` for
            # UTC, `+HH:MM` / `-HH:MM` otherwise. The stats-graph
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
                if      (dl > 0) dcolor = note   # growth
                else if (dl < 0) dcolor = ok     # shrinkage
                else             dcolor = dim    # no change
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
    }
'
