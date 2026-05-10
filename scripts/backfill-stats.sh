#!/bin/sh
# scripts/backfill-stats.sh — compute Code-stats:-equivalent rows
# for parent-repo commits that landed before the `Code-stats:`
# trailer was added to commit-all.sh, and append them to the
# tracked sidecar cache at `docs/stats-cache.tsv`.
#
# Why a sidecar cache and not a history rewrite: the workspace's
# append-only history rule (CONTRIBUTING.md §4.4 / CLAUDE.md
# "history is append-only") forbids retroactively editing past
# commits to add the trailer. Recording the equivalent stats in a
# tracked, append-only TSV keeps history honest while restoring
# full coverage in `scripts/stats-log.sh` and the
# `update-stats-graph.sh` / `stats-graph` SVG pipeline.
#
# How: for each missing-trailer parent commit, this script
# reconstructs the workspace tree at that SHA via repeated
# `git archive` calls (parent + each gitlink-pinned submodule SHA),
# extracts into a tempdir, runs `tokei` over it, parses the Total
# line, and appends `<full-sha>\t<files>\t<total>\t<code>\t<docs>`
# to the cache. Mirrors the construction strategy in
# `scripts/archive-all.sh` — same `git archive HEAD` per repo,
# concatenated into one tree — but driven by parent-pinned gitlink
# SHAs (not each submodule's current HEAD) so we get the
# point-in-time workspace shape.
#
# Submodule resolution caveats:
#  - If the parent's `.gitmodules` at that commit references a
#    submodule path no longer present (deleted, or renamed in
#    parent-tree), the script first tries a *rename heuristic*:
#    scan every currently-initialized submodule's git db for the
#    pinned SHA. If exactly one matches, the historical commit
#    is reproducible — use that submodule's git db as the
#    source, and extract under the *historical* path (so tokei
#    sees the layout the parent commit recorded). Zero matches
#    or multiple matches → bail (or mark unreproducible under
#    `--allow-partial`).
#  - If a submodule SHA pinned at the parent commit is missing
#    from every local submodule's git db (typically because that
#    branch was force-pushed upstream), same bail behaviour.
#    Workaround in the error message: try `git -C <sub> fetch
#    --all` to re-fetch — sometimes the SHA is reachable from
#    another ref.
#  - With `--allow-partial`, unreproducible commits get a
#    `# unreproducible <sha>: <reason>` comment line in the cache
#    instead of a stats row, and the script continues; coverage
#    isn't 100% but the cache stays informative.
#
# Resumability: the cache is read at startup and any commit
# already present (whether as a stats row or an unreproducible
# marker) is skipped on rerun. Streaming append-on-success means
# a bail mid-run only loses the in-flight commit's progress.
#
# Usage:
#   ./scripts/backfill-stats.sh                      # bail on first failure
#   ./scripts/backfill-stats.sh --allow-partial      # mark and continue
#   ./scripts/backfill-stats.sh --dry-run            # list missing commits, don't compute
#   ./scripts/backfill-stats.sh -n <N>               # only process oldest N missing
#
# POSIX sh — see CONTRIBUTING.md §6.

set -eu
. "$(dirname -- "$0")/lib/workspace-cd.sh"

cache_file="docs/stats-cache.tsv"
allow_partial=0
dry_run=0
limit=0

usage() {
    cat <<'EOF'
Usage: backfill-stats.sh [--allow-partial] [--dry-run] [-n <N>] [-h|--help]

Backfill Code-stats:-equivalent rows for pre-trailer commits into
docs/stats-cache.tsv. Resumable; bails on unreproducible commits
unless --allow-partial.

Options:
  --allow-partial   Mark unreproducible commits in cache and continue.
  --dry-run         List missing commits, don't compute or write.
  -n <N>            Only process the oldest N missing commits.
  -h, --help        Show this help and exit.
EOF
}

while [ $# -gt 0 ]; do
    case "$1" in
        -h|--help)        usage; exit 0 ;;
        --allow-partial)  allow_partial=1; shift ;;
        --dry-run)        dry_run=1; shift ;;
        -n)
            if [ $# -lt 2 ]; then
                printf '!!! backfill-stats.sh: -n requires an integer\n' >&2
                exit 2
            fi
            case "$2" in
                ''|*[!0-9]*) printf '!!! backfill-stats.sh: -n must be a positive integer\n' >&2; exit 2 ;;
            esac
            limit=$2
            shift 2 ;;
        *)
            printf '!!! backfill-stats.sh: unknown argument: %s\n' "$1" >&2
            usage >&2
            exit 2 ;;
    esac
done

if ! command -v tokei >/dev/null 2>&1; then
    printf '!!! backfill-stats.sh: tokei not on PATH. Install via `./scripts/tokei.sh` once (it auto-installs) and rerun.\n' >&2
    exit 1
fi

# Ensure docs/ exists for the cache.
mkdir -p docs

# All scratch state lives under one session tempdir in /tmp so
# the workspace tree stays untouched (we never `git checkout` or
# mutate submodule state). Single trap handles cleanup on any
# exit path. Per-commit reconstructions are subdirectories.
session_tmp=$(mktemp -d "${TMPDIR:-/tmp}/backfill-stats.XXXXXX")
trap 'rm -rf "$session_tmp" 2>/dev/null || true' EXIT INT HUP TERM

seen_file="$session_tmp/seen"
missing_file="$session_tmp/missing"
submodule_paths_file="$session_tmp/submodule-paths"
: > "$seen_file"
: > "$missing_file"

# Cache the list of currently-initialized submodule paths once.
# Used by the rename heuristic when a recorded gitlink path is
# absent from the current tree. `git submodule status` lines
# starting with `-` mark uninitialized submodules; filter those
# out — without an init'd git db there's nothing to scan.
git submodule status | awk '!/^-/ {print $2}' > "$submodule_paths_file"

if [ -f "$cache_file" ]; then
    # Stats rows: first tab-separated field is the SHA.
    awk -F '\t' '!/^#/ && NF >= 5 { print $1 }' "$cache_file" >> "$seen_file"
    # Unreproducible markers: `# unreproducible <sha>: <reason>`.
    awk '/^# unreproducible / { print $3 }' "$cache_file" | sed 's/:$//' >> "$seen_file"
fi

# Enumerate parent-repo commits without the Code-stats trailer,
# in oldest-first order so each new entry's predecessors (for
# delta computation) are likely already cached. Dropping the
# trailer check via the trailer-line absence is more robust than
# parsing the trailer body (a malformed trailer should still be
# treated as missing).
git log --reverse \
    --pretty=tformat:'%H%x09%(trailers:key=Code-stats,valueonly=true)' \
  | awk -F '\t' -v seen_file="$seen_file" '
        BEGIN {
            while ((getline line < seen_file) > 0) seen[line] = 1
            close(seen_file)
        }
        # Skip blank lines that the trailer placeholder leaves
        # between commits in tformat output.
        /^[[:space:]]*$/ { next }
        # If the trailer field is present (non-empty), the commit
        # already carries Code-stats; skip.
        ($2 != "") { next }
        # Already cached or marked? Skip.
        ($1 in seen) { next }
        { print $1 }
    ' >> "$missing_file"

missing_count=$(wc -l < "$missing_file" | tr -d ' ')

if [ "$missing_count" -eq 0 ]; then
    printf '=== backfill-stats: nothing to do (cache already covers every pre-trailer commit) ===\n' >&2
    exit 0
fi

if [ "$limit" -gt 0 ] && [ "$limit" -lt "$missing_count" ]; then
    head -n "$limit" "$missing_file" > "$missing_file.cap"
    mv "$missing_file.cap" "$missing_file"
    printf '=== backfill-stats: %d missing total, processing oldest %d (--n %d) ===\n' \
        "$missing_count" "$limit" "$limit" >&2
    missing_count=$limit
else
    printf '=== backfill-stats: %d missing commit(s) to process ===\n' "$missing_count" >&2
fi

if [ "$dry_run" -eq 1 ]; then
    printf '=== --dry-run: missing SHAs (oldest first) ===\n' >&2
    cat "$missing_file"
    exit 0
fi

# Initialise the cache file with a header comment if it doesn't
# exist yet.
if [ ! -f "$cache_file" ]; then
    cat > "$cache_file" <<'EOF'
# docs/stats-cache.tsv — backfilled Code-stats:-equivalent rows for
# pre-trailer commits. Generated by `scripts/backfill-stats.sh`;
# consulted by `scripts/stats-log.sh` as a fallback when a
# commit's `Code-stats:` trailer is absent.
#
# Format: tab-separated, one row per commit:
#   <full-sha>\t<files>\t<total>\t<code>\t<docs>
#
# Lines starting with `#` are comments. Unreproducible commits
# (when run with --allow-partial) appear as:
#   # unreproducible <full-sha>: <reason>
#
# Append-only: do not edit existing rows.
EOF
fi

# Process each missing commit.
processed=0
failed=0
while IFS= read -r sha; do
    [ -z "$sha" ] && continue
    processed=$((processed + 1))

    tmpdir=$(mktemp -d "$session_tmp/commit-XXXXXX")
    # All per-iteration scratch is under $session_tmp; the
    # session-level trap cleans it on any exit path. We also
    # `rm -rf "$tmpdir"` explicitly at the end of each
    # iteration to keep peak /tmp usage bounded across a
    # 464-commit run.

    # Extract parent tree.
    if ! git archive --format=tar "$sha" | tar -xf - -C "$tmpdir" 2>/dev/null; then
        rm -rf "$tmpdir"
        msg="git archive parent failed for $sha"
        if [ "$allow_partial" -eq 1 ]; then
            printf '# unreproducible %s: %s\n' "$sha" "$msg" >> "$cache_file"
            printf '  [%d/%d] %s SKIP (%s)\n' "$processed" "$missing_count" "$sha" "$msg" >&2
            failed=$((failed + 1))
            continue
        fi
        printf '!!! backfill-stats: %s\n' "$msg" >&2
        printf '    Workaround: rerun with --allow-partial to mark this commit and continue.\n' >&2
        printf '    Cache file: %s (%d entries appended this run)\n' "$cache_file" "$((processed - 1))" >&2
        exit 1
    fi

    # Enumerate gitlinks pinned at this parent SHA and extract each.
    gitlinks=$(git ls-tree -r "$sha" | awk '$1 == "160000" {print $3 "\t" $4}')
    sub_failure=""
    if [ -n "$gitlinks" ]; then
        # IFS handling: tab is the field sep; newlines are the
        # record sep. Process with `printf | while`.
        printf '%s\n' "$gitlinks" | while IFS="$(printf '\t')" read -r sub_sha sub_path; do
            [ -z "$sub_sha" ] && continue
            # Default source: same path as recorded.
            source_db="$sub_path"
            # If the recorded path is absent (parent-tree rename
            # since this commit), apply the rename heuristic:
            # scan every initialized submodule for the SHA. Use
            # that submodule's git db as the source; the extract
            # destination stays the historical path so tokei
            # sees the layout the parent commit recorded.
            if [ ! -d "$sub_path/.git" ] && [ ! -f "$sub_path/.git" ]; then
                matches=""
                match_count=0
                while IFS= read -r cur_path; do
                    [ -z "$cur_path" ] && continue
                    if git -C "$cur_path" cat-file -e "$sub_sha" 2>/dev/null; then
                        matches="${matches:+$matches }$cur_path"
                        match_count=$((match_count + 1))
                    fi
                done < "$submodule_paths_file"
                case "$match_count" in
                    0)
                        printf '%s\n' "missing submodule path '$sub_path' and no current submodule has SHA $sub_sha (added later, removed, or upstream force-pushed away)" > "$tmpdir/.backfill-fail"
                        exit 0
                        ;;
                    1)
                        source_db="$matches"
                        ;;
                    *)
                        printf '%s\n' "ambiguous: SHA $sub_sha is reachable in multiple current submodules ($matches); refusing to guess which historical path '$sub_path' aliased" > "$tmpdir/.backfill-fail"
                        exit 0
                        ;;
                esac
            fi
            # The SHA must be reachable in the source git db.
            if ! git -C "$source_db" cat-file -e "$sub_sha" 2>/dev/null; then
                printf '%s\n' "submodule '$source_db' missing object $sub_sha (try: git -C $source_db fetch --all)" > "$tmpdir/.backfill-fail"
                exit 0
            fi
            # Extract this submodule's tree under the historical
            # path. The source repo may differ from $sub_path
            # when the rename heuristic kicked in.
            mkdir -p "$tmpdir/$sub_path"
            if ! git -C "$source_db" archive --format=tar "$sub_sha" | tar -xf - -C "$tmpdir/$sub_path" 2>/dev/null; then
                printf '%s\n' "git archive failed for submodule '$sub_path' (source: $source_db) at $sub_sha" > "$tmpdir/.backfill-fail"
                exit 0
            fi
        done
        if [ -f "$tmpdir/.backfill-fail" ]; then
            sub_failure=$(cat "$tmpdir/.backfill-fail")
        fi
    fi

    if [ -n "$sub_failure" ]; then
        rm -rf "$tmpdir"
        if [ "$allow_partial" -eq 1 ]; then
            printf '# unreproducible %s: %s\n' "$sha" "$sub_failure" >> "$cache_file"
            printf '  [%d/%d] %s SKIP (%s)\n' "$processed" "$missing_count" "$sha" "$sub_failure" >&2
            failed=$((failed + 1))
            continue
        fi
        printf '!!! backfill-stats: %s at commit %s\n' "$sub_failure" "$sha" >&2
        printf '    Workaround: rerun with --allow-partial to mark this commit and continue.\n' >&2
        printf '    Cache file: %s (%d entries appended this run)\n' "$cache_file" "$((processed - 1))" >&2
        exit 1
    fi

    # Run tokei on the assembled tree.
    total_line=$( (cd "$tmpdir" && tokei) | grep '^[[:space:]]*Total' | head -1 || true)
    rm -rf "$tmpdir"

    if [ -z "$total_line" ]; then
        msg="tokei produced no Total line"
        if [ "$allow_partial" -eq 1 ]; then
            printf '# unreproducible %s: %s\n' "$sha" "$msg" >> "$cache_file"
            printf '  [%d/%d] %s SKIP (%s)\n' "$processed" "$missing_count" "$sha" "$msg" >&2
            failed=$((failed + 1))
            continue
        fi
        printf '!!! backfill-stats: %s for commit %s\n' "$msg" "$sha" >&2
        printf '    Workaround: rerun with --allow-partial to mark this commit and continue.\n' >&2
        exit 1
    fi

    files=$(printf '%s' "$total_line" | awk '{print $2}')
    lines=$(printf '%s' "$total_line" | awk '{print $3}')
    code=$(printf '%s' "$total_line"  | awk '{print $4}')
    docs=$(printf '%s' "$total_line"  | awk '{print $5}')

    # Sanity: all four must be integers.
    case "$files$lines$code$docs" in
        ''|*[!0-9]*)
            msg="parsed non-integer fields from tokei Total line: '$total_line'"
            if [ "$allow_partial" -eq 1 ]; then
                printf '# unreproducible %s: %s\n' "$sha" "$msg" >> "$cache_file"
                printf '  [%d/%d] %s SKIP (%s)\n' "$processed" "$missing_count" "$sha" "$msg" >&2
                failed=$((failed + 1))
                continue
            fi
            printf '!!! backfill-stats: %s\n' "$msg" >&2
            printf '    Workaround: rerun with --allow-partial to mark this commit and continue.\n' >&2
            exit 1
            ;;
    esac

    printf '%s\t%d\t%d\t%d\t%d\n' "$sha" "$files" "$lines" "$code" "$docs" >> "$cache_file"
    printf '  [%d/%d] %s OK (%dF %dL %dC %dD)\n' \
        "$processed" "$missing_count" "$sha" "$files" "$lines" "$code" "$docs" >&2
done < "$missing_file"

success=$((processed - failed))
printf '=== backfill-stats: done (%d processed, %d ok, %d unreproducible) ===\n' \
    "$processed" "$success" "$failed" >&2
