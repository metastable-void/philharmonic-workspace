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

cd "$(git rev-parse --show-toplevel)"

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

if [ ! -f "$crate/Cargo.toml" ]; then
    printf '!!! %s: not a workspace crate (no %s/Cargo.toml)\n' "$crate" "$crate" >&2
    exit 1
fi

# Extract the first `version = "..."` line from the package manifest.
# [package] comes before [dependencies] in every workspace crate.
version=$(awk -F'"' '/^version *=/ { print $2; exit }' "$crate/Cargo.toml")
if [ -z "$version" ]; then
    printf '!!! %s: could not parse version from Cargo.toml\n' "$crate" >&2
    exit 1
fi

tag="v$version"

printf '=== %s %s ===\n' "$crate" "$tag"

# Refuse to proceed if the submodule is in a bad state.
(
    cd "$crate"
    branch=$(git rev-parse --abbrev-ref HEAD)
    if [ "$branch" = "HEAD" ]; then
        echo "!!! $crate: detached HEAD; checkout a branch first" >&2
        exit 1
    fi
    if ! git diff --quiet \
        || ! git diff --cached --quiet \
        || [ -n "$(git ls-files --others --exclude-standard)" ]; then
        echo "!!! $crate: working tree dirty; commit or clean before publishing" >&2
        exit 1
    fi
    if git rev-parse --verify "refs/tags/$tag" >/dev/null 2>&1; then
        printf '!!! %s: tag %s already exists locally\n' "$crate" "$tag" >&2
        exit 1
    fi
)

# Sanity: always run --dry-run first. Catches missing README,
# unsatisfied dep versions, oversized tarballs, etc. before the real
# publish.
printf '=== cargo publish --dry-run -p %s ===\n' "$crate"
cargo publish --dry-run -p "$crate"

if [ "$dry_run" -eq 1 ]; then
    echo "=== --dry-run: stopping; no real publish, no tag ==="
    exit 0
fi

printf '=== cargo publish -p %s ===\n' "$crate"
cargo publish -p "$crate"

# Tag inside the submodule's own repo. Signed annotated tag matches
# the workspace commit-signing rule.
(
    cd "$crate"
    git tag -s "$tag" -m "release $crate $tag"
    printf '=== tagged %s %s (signed) ===\n' "$crate" "$tag"
)

echo
printf 'Published %s %s. Run ./scripts/push-all.sh to push the tag.\n' "$crate" "$tag"
