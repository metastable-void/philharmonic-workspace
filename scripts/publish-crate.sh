#!/bin/sh
# scripts/publish-crate.sh — publish one Philharmonic crate to
# crates.io and tag the release in the crate's submodule repo.
#
# Usage:
#   ./scripts/publish-crate.sh [--dry-run] <crate-name>
#
# Flow:
#   1. Locate the crate's submodule directory; refuse if the tree
#      is dirty or in detached HEAD, or if the tag already exists.
#   2. Extract the version from the crate's own Cargo.toml; the
#      release tag is `v<version>` inside the submodule.
#   3. Run `cargo publish --dry-run` as a sanity check.
#   4. If --dry-run was passed, stop here. No tag is created.
#   5. Otherwise run `cargo publish`. Only on success do we create
#      a signed annotated tag in the submodule repo. A failed
#      publish must not leave a dangling tag.
#
# The tag is NOT pushed by this script. Run ./scripts/push-all.sh
# afterwards; it uses --follow-tags so tags pointing at pushed
# commits go up alongside the branch. See
# docs/design/13-conventions.md §Release tagging.
#
# Crates must be published in dependency order: cornerstone first
# (philharmonic-types), dependents after. `cargo publish` will
# refuse if a dependency version isn't yet on crates.io.
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu

. "$(dirname -- "$0")/lib/colors.sh"
. "$(dirname -- "$0")/lib/workspace-cd.sh"

dry_run=0
while [ $# -gt 0 ]; do
    case "$1" in
        --dry-run) dry_run=1; shift ;;
        --) shift; break ;;
        -*) printf 'unknown flag: %s\n' "$1" >&2; exit 2 ;;
        *) break ;;
    esac
done

if [ $# -ne 1 ]; then
    echo "Usage: $0 [--dry-run] <crate-name>" >&2
    exit 2
fi

crate=$1

# Refuse to publish in-tree (non-submodule) workspace members.
# They're dev tooling with `publish = false`, have no separate
# git repo for release tags, and running this script against one
# would (at best) fail at `cargo publish` and (worse) place a
# `v<version>` tag in the parent repo instead of the submodule —
# polluting the parent's tag namespace with a spurious release
# marker. Submodules carry a `.git` file at their root; in-tree
# members don't.
if [ -d "$crate" ] && [ ! -f "$crate/.git" ]; then
    printf '%s!!! %s: in-tree workspace member (not a submodule). publish-crate.sh%s\n' "$C_ERR" "$crate" "$C_RESET" >&2
    printf '    only supports submodule-backed crates; in-tree tooling like\n' >&2
    printf '    `xtask` is `publish = false` by design.\n' >&2
    exit 1
fi

# `crate-version.sh` validates that $crate/Cargo.toml exists and
# parses its version line; it exits non-zero with a clear error
# if either step fails.
version=$(./scripts/crate-version.sh "$crate")

tag="v$version"

printf '%s=== %s %s ===%s\n' "$C_HEADER" "$crate" "$tag" "$C_RESET"

# Refuse to proceed if the submodule is in a bad state.
(
    cd "$crate"
    branch=$(git rev-parse --abbrev-ref HEAD)
    if [ "$branch" = "HEAD" ]; then
        printf '%s!!! %s: detached HEAD; checkout a branch first%s\n' "$C_ERR" "$crate" "$C_RESET" >&2
        exit 1
    fi
    if ! git diff --quiet \
        || ! git diff --cached --quiet \
        || [ -n "$(git ls-files --others --exclude-standard)" ]; then
        printf '%s!!! %s: working tree dirty; commit or clean before publishing%s\n' "$C_ERR" "$crate" "$C_RESET" >&2
        exit 1
    fi
    if git rev-parse --verify "refs/tags/$tag" >/dev/null 2>&1; then
        printf '%s!!! %s: tag %s already exists locally%s\n' "$C_ERR" "$crate" "$tag" "$C_RESET" >&2
        exit 1
    fi
)

# Sanity: always run --dry-run first. Catches missing README,
# unsatisfied dep versions, oversized tarballs, etc. before the real
# publish.
printf '%s=== cargo publish --dry-run -p %s ===%s\n' "$C_HEADER" "$crate" "$C_RESET"
cargo publish --dry-run -p "$crate"

if [ "$dry_run" -eq 1 ]; then
    printf '%s=== --dry-run: stopping; no real publish, no tag ===%s\n' "$C_WARN" "$C_RESET"
    exit 0
fi

printf '%s=== cargo publish -p %s ===%s\n' "$C_HEADER" "$crate" "$C_RESET"
cargo publish -p "$crate"

# Tag inside the submodule's own repo. Signed annotated tag matches
# the workspace commit-signing rule.
(
    cd "$crate"
    git tag -s "$tag" -m "release $crate $tag"
    printf '%s=== tagged %s %s (signed) ===%s\n' "$C_OK" "$crate" "$tag" "$C_RESET"
)

echo
printf '%sPublished %s %s.%s Run ./scripts/push-all.sh to push the tag.\n' \
    "$C_OK" "$crate" "$tag" "$C_RESET"
