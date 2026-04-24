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

# Cooldown-mirror + workspace-pollution workarounds, applied
# only while this script runs. The committed
# `.cargo/config.toml` is NOT modified.
#
# 1. CARGO_REGISTRY_DEFAULT=crates-io
#    `.cargo/config.toml` has `[registry] default =
#    "menhera-cooldown"`. Without this override, `cargo publish`
#    would target the cooldown mirror; cargo then refuses with
#    "dependencies sourced from other registries" because every
#    third-party dep is tagged as coming from `crates-io`
#    internally, and it treats the mirror as a different
#    registry even when the mirror's `config.json` routes its
#    `api` to real crates.io. Setting the env here makes the
#    publish target crates-io directly (matching dep identity).
#
# 2. --config source.menhera-cooldown.registry =
#    "sparse+https://index.crates.menhera.org/0d/"
#    `.cargo/config.toml` also has
#    `[source.crates-io].replace-with = "menhera-cooldown"`,
#    which routes every source-resolution lookup (including
#    the verify-build phase of `cargo publish`) through the
#    Menhera mirror. The committed config points the
#    `menhera-cooldown` source at `/3d/`, the 3-day cooldown
#    endpoint — any workspace-internal dep published inside
#    that window (e.g., philharmonic-connector-common 0.2.0
#    when publishing impl-api the next day) is invisible and
#    verify-build fails with "no matching package". The `/0d/`
#    endpoint on the same Menhera proxy serves the index with
#    no cooldown filtering; redirecting `menhera-cooldown` to
#    it just for this invocation lets verify-build see fresh
#    deps without touching the committed source-replacement
#    wiring. Consumers pulling the newly-published crate
#    through the default `/3d/` mirror will still see the
#    usual ~3-day cooldown by design.
#
# 3. CARGO_TARGET_DIR=target-publish
#    Verify-build compiles the packaged tarball; without a
#    separate target directory, those artefacts would land in
#    the shared `target/` that normal development (and any
#    concurrent Codex session) depends on. Diverting to
#    `target-publish/` keeps the publish's build isolated —
#    same pattern as `scripts/xtask.sh` uses `target-xtask/`
#    (see `.gitignore` and CONTRIBUTING.md §8.1).
#
# Cargo.lock is NOT modified by this flow — empirically
# verified via the dry-run path; the workspace lockfile is
# unaffected because the sparse-index URL switch produces no
# resolution diff that would change source URLs in the lock.
CARGO_REGISTRY_DEFAULT="crates-io"
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target-publish}"
export CARGO_REGISTRY_DEFAULT CARGO_TARGET_DIR

pub_config='source.menhera-cooldown.registry = "sparse+https://index.crates.menhera.org/0d/"'

# Sanity: always run --dry-run first. Catches missing README,
# unsatisfied dep versions, oversized tarballs, etc. before the real
# publish.
printf '%s=== cargo publish --dry-run -p %s ===%s\n' "$C_HEADER" "$crate" "$C_RESET"
cargo publish --dry-run --config "$pub_config" -p "$crate"

if [ "$dry_run" -eq 1 ]; then
    printf '%s=== --dry-run: stopping; no real publish, no tag ===%s\n' "$C_WARN" "$C_RESET"
    exit 0
fi

printf '%s=== cargo publish -p %s ===%s\n' "$C_HEADER" "$crate" "$C_RESET"
cargo publish --config "$pub_config" -p "$crate"

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
