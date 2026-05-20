#!/bin/sh
# Workspace status: parent and every submodule.
# Shows dirty working trees, ahead/behind vs. the upstream
# branch, detached HEAD, and any tags that exist locally but
# not on origin (refs only — commits may already be pushed).
#
# The unpushed-tag probe is one `git ls-remote --tags --refs
# origin` per repo. Git doesn't track per-tag remote-status
# locally, and publish-crate.sh routinely tags commits already
# on origin, so reachability-based heuristics miss the
# tag-only-unpushed case. status.sh is read-only, so per-repo
# work runs in parallel and outputs collect through a temp
# dir for deterministic printing.
#
# With `--diff`, after the per-repo status blocks the script
# also emits a colored diff for every dirty file across the
# parent and every submodule. The diff covers both unstaged
# (`git diff`) and staged (`git diff --cached`) changes; raw
# `git diff` should not be invoked outside this wrapper (the
# workspace soft-bans it in CLAUDE.md / AGENTS.md). Untracked
# files are surfaced by status only — git won't diff them
# until they're staged.
#
# Colors: ANSI sequences are emitted only when stdout is a TTY
# and NO_COLOR is unset. Pass `--no-color` to force plain
# output. Git's own status coloring follows the same gate.
#
# POSIX sh only — see CONTRIBUTING.md §6.

set -eu

. "$(dirname -- "$0")/lib/script-help.sh"
script_help_handle "$@"


no_color=0
show_diff=0
while [ $# -gt 0 ]; do
    case "$1" in
        --no-color) no_color=1; shift ;;
        --diff)     show_diff=1; shift ;;
        -h|--help)
            printf 'Usage: status.sh [--no-color] [--diff] [-h|--help]\n'
            exit 0 ;;
        *)
            printf 'status.sh: unknown flag: %s\n' "$1" >&2
            exit 2 ;;
    esac
done

if [ "$no_color" -eq 1 ]; then
    NO_COLOR=1
    export NO_COLOR
fi
. "$(dirname -- "$0")/lib/colors.sh"
. "$(dirname -- "$0")/lib/workspace-cd.sh"

# git status coloring: `always` when our colors are on,
# `never` otherwise. C_RESET is non-empty iff colors are on.
if [ -n "$C_RESET" ]; then
    git_color=always
else
    git_color=never
fi
export git_color

tmpdir=$(mktemp -d 2>/dev/null) || {
    printf '%s!!! status.sh: mktemp failed%s\n' "$C_ERR" "$C_RESET" >&2
    exit 1
}
trap 'rm -rf "$tmpdir"' EXIT INT TERM

# check_repo NAME IS_SUBMODULE
#   Probes the current working directory's repo. Writes a status
#   block to stdout, or nothing when IS_SUBMODULE=1 and the repo
#   is fully clean. Reads C_*, git_color from the environment.
check_repo() {
    name=$1
    is_submodule=$2

    dirty=$(git -c color.status="$git_color" status -s)
    ahead=""
    behind=""
    ab=$(git status --porcelain=v2 --branch 2>/dev/null \
        | grep "^# branch\.ab" || true)
    if [ -n "$ab" ]; then
        ahead=$(echo "$ab" | cut -d' ' -f3)
        behind=$(echo "$ab" | cut -d' ' -f4)
    fi
    diverged=0
    if [ -n "$ahead" ] && \
       { [ "$ahead" != "+0" ] || [ "$behind" != "-0" ]; }; then
        diverged=1
    fi
    branch=$(git rev-parse --abbrev-ref HEAD)
    detached=0
    if [ "$branch" = "HEAD" ]; then
        detached=1
    fi

    unpushed_tags=""
    if git remote get-url origin >/dev/null 2>&1; then
        local_tags=$(git tag)
        if [ -n "$local_tags" ]; then
            remote_tags=$(git ls-remote --tags --refs origin 2>/dev/null \
                | awk '{ sub(/^refs\/tags\//, "", $2); print $2 }')
            for tag in $local_tags; do
                if ! printf '%s\n' "$remote_tags" \
                        | grep -q -x -F -- "$tag"; then
                    unpushed_tags="$unpushed_tags $tag"
                fi
            done
        fi
    fi

    # Submodule path: silent when clean.
    if [ "$is_submodule" = "1" ] && \
       [ -z "$dirty" ] && [ "$diverged" = "0" ] && \
       [ "$detached" = "0" ] && [ -z "$unpushed_tags" ]; then
        return 0
    fi

    printf '%s=== %s ===%s\n' "$C_HEADER" "$name" "$C_RESET"
    if [ -n "$dirty" ]; then
        printf '%s\n' "$dirty"
    elif [ "$is_submodule" = "0" ]; then
        printf '  %s(clean)%s\n' "$C_OK" "$C_RESET"
    fi

    branch_name=$(git branch --show-current)
    if [ "$diverged" = "1" ]; then
        printf '  %s(branch %s: ahead=%s behind=%s)%s\n' \
            "$C_WARN" "$branch_name" "$ahead" "$behind" "$C_RESET"
    fi
    if [ "$detached" = "1" ]; then
        printf '  %s(detached HEAD)%s\n' "$C_ERR" "$C_RESET"
    fi
    if [ -n "$unpushed_tags" ]; then
        printf '  %s(unpushed tags:%s)%s\n' \
            "$C_WARN" "$unpushed_tags" "$C_RESET"
    fi
    echo
}

# Parent first (lexicographically via "000-").
( check_repo "parent" 0 > "$tmpdir/000-parent.out" ) &

# Submodules in parallel. Skip uninitialized ones (line prefix
# "-" in `git submodule status`).
git submodule status \
    | awk '$0 !~ /^-/ { print $2 }' \
    | sort > "$tmpdir/.paths"

i=0
while IFS= read -r path; do
    i=$((i + 1))
    seq=$(printf '%03d' "$i")
    name=$(basename "$path")
    (
        cd "$path" && check_repo "$name" 1
    ) > "$tmpdir/${seq}-${name}.out" &
done < "$tmpdir/.paths"

# wait may return non-zero if a backgrounded probe exited with
# an error (e.g. a submodule with an unusual remote config). The
# block was still written to its temp file, so don't propagate.
wait || true

for f in "$tmpdir"/[0-9]*.out; do
    [ -s "$f" ] && cat "$f"
done

# --diff: after the status blocks, emit a colored diff for the
# parent and every submodule with dirty working tree changes.
# Combines `git diff` (unstaged) and `git diff --cached` (staged)
# under one header per repo. Submodules with no dirty changes are
# skipped silently. Untracked files don't appear in git diff
# output by design — status already lists them as `??`.
if [ "$show_diff" -eq 1 ]; then
    print_diff_for_cwd() {
        _pdfc_name=$1
        _pdfc_unstaged=$(git -c color.diff="$git_color" diff)
        _pdfc_staged=$(git -c color.diff="$git_color" diff --cached)
        if [ -z "$_pdfc_unstaged" ] && [ -z "$_pdfc_staged" ]; then
            return 0
        fi
        printf '%s=== diff: %s ===%s\n' "$C_HEADER" "$_pdfc_name" "$C_RESET"
        if [ -n "$_pdfc_unstaged" ]; then
            printf '%s--- unstaged ---%s\n' "$C_DIM" "$C_RESET"
            printf '%s\n' "$_pdfc_unstaged"
        fi
        if [ -n "$_pdfc_staged" ]; then
            printf '%s--- staged ---%s\n' "$C_DIM" "$C_RESET"
            printf '%s\n' "$_pdfc_staged"
        fi
        echo
    }

    print_diff_for_cwd 'parent'
    while IFS= read -r path; do
        [ -n "$path" ] || continue
        name=$(basename "$path")
        ( cd "$path" && print_diff_for_cwd "$name" )
    done < "$tmpdir/.paths"
fi

exit 0
