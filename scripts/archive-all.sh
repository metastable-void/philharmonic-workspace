#!/bin/sh
# scripts/archive-all.sh — bundle the parent workspace + every
# submodule's HEAD tree into a single zstd-compressed tarball at
# `archives/philharmonic-workspace-<HEAD_SHA>.tar.zst`.
#
# Pipeline:
#   1. `git archive HEAD` the parent into a tempfile, prefixed
#      with `philharmonic-workspace-<HEAD_SHA>/`.
#   2. `git submodule foreach --recursive` runs `git archive HEAD`
#      in each submodule, prefixed with
#      `philharmonic-workspace-<HEAD_SHA>/<displaypath>/`.
#   3. The per-tree (uncompressed) tarballs are concatenated into
#      one zstd-compressed output via the `tar-concatenate` xtask
#      bin (`./scripts/xtask.sh tar-concatenate -- --zstd ...`).
#
# Tempfiles come from `./scripts/mktemp.sh` and are removed on any
# exit path (normal, error, signal).
#
# Caveats:
#   - HEAD-only — staged or unstaged working-tree changes are NOT
#     captured. Commit (or stash) before archiving to include them.
#   - `git archive HEAD` inside a submodule reads the SUBMODULE's
#     own HEAD, not the parent's pinned gitlink. Right after
#     `pull-all.sh` / `setup.sh` the two match; if you've manually
#     checked out a different commit inside a submodule, the
#     archive captures that. Rerun `pull-all.sh` first if you want
#     the parent-pinned snapshot.
#   - The output filename uses only the parent's HEAD SHA. The
#     parent's tree pins each submodule's commit via gitlinks at
#     that SHA, so the parent SHA uniquely identifies the bundle.
#   - Aborts if any submodule is uninitialized — a partial archive
#     would be silently incomplete. Run `scripts/setup.sh` (or
#     `git submodule update --init --recursive`) first.
#
# POSIX sh — see CONTRIBUTING.md §6.

set -eu

. "$(dirname -- "$0")/lib/script-help.sh"
script_help_handle "$@"

. "$(dirname -- "$0")/lib/workspace-cd.sh"

head_sha=$(git rev-parse HEAD)
prefix="philharmonic-workspace-${head_sha}"
out_dir="archives"
out_path="${out_dir}/${prefix}.tar.zst"

# Refuse to archive with uninitialized submodules.
uninit=$(git submodule status --recursive | awk '/^-/{print $2}')
if [ -n "$uninit" ]; then
    printf '!!! archive-all.sh: uninitialized submodules:\n' >&2
    printf '%s\n' "$uninit" | sed 's/^/    /' >&2
    printf '    Run scripts/setup.sh (or `git submodule update --init --recursive`) first.\n' >&2
    exit 1
fi

mkdir -p "$out_dir"

# `tmp_list` holds one tempfile path per line, in concatenation
# order: parent first, then each submodule in `git submodule
# foreach` enumeration order.
tmp_list=$(./scripts/mktemp.sh archive-list)
trap '
    if [ -f "$tmp_list" ]; then
        while IFS= read -r p; do
            [ -n "$p" ] && rm -f "$p"
        done < "$tmp_list"
        rm -f "$tmp_list"
    fi
' EXIT INT HUP TERM

# Parent.
parent_tar=$(./scripts/mktemp.sh archive-parent)
printf '%s\n' "$parent_tar" >> "$tmp_list"
git archive --prefix="${prefix}/" -o "$parent_tar" HEAD
printf '  archived parent → %s\n' "$parent_tar" >&2

# Submodules. `git submodule foreach` runs each command via
# `sh -c` in the submodule's working dir, inheriting exported
# environment from this shell. `$toplevel` and `$displaypath`
# are set by foreach itself.
ARCHIVE_PREFIX="$prefix"
ARCHIVE_LIST="$tmp_list"
export ARCHIVE_PREFIX ARCHIVE_LIST
git submodule foreach --quiet --recursive '
    sub_tar=$("$toplevel/scripts/mktemp.sh" archive-sub)
    printf "%s\n" "$sub_tar" >> "$ARCHIVE_LIST"
    git archive --prefix="${ARCHIVE_PREFIX}/${displaypath}/" -o "$sub_tar" HEAD
    printf "  archived %s → %s\n" "$displaypath" "$sub_tar" >&2
'
unset ARCHIVE_PREFIX ARCHIVE_LIST

# Concatenate to the final zstd-compressed tarball. mktemp paths
# are `${TMPDIR:-/tmp}/<slug>.XXXXXX` with no spaces or special
# characters, so word-splitting on `cat` is safe here.
# shellcheck disable=SC2046
./scripts/xtask.sh tar-concatenate -- --zstd -o "$out_path" $(cat "$tmp_list")

printf 'wrote %s\n' "$out_path"
