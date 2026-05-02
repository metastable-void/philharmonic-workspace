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
. "$(dirname -- "$0")/lib/cargo-noise-filter.sh"

dry_run=0
no_verify=0
while [ $# -gt 0 ]; do
    case "$1" in
        --dry-run)   dry_run=1; shift ;;
        --no-verify) no_verify=1; shift ;;
        --) shift; break ;;
        -*) printf 'unknown flag: %s\n' "$1" >&2; exit 2 ;;
        *) break ;;
    esac
done

if [ $# -ne 1 ]; then
    echo "Usage: $0 [--dry-run] [--no-verify] <crate-name>" >&2
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
# 1. `cargo pub-fresh` — workspace alias in
#    `.cargo/config.toml` that expands to
#    `cargo publish --config 'registry.default = "crates-io"'
#                   --config 'source.menhera-cooldown.registry =
#                             "sparse+https://index.crates.menhera.org/0d/"'`.
#    (Name is `pub-fresh`, not `publish-fresh` or
#    `fresh-publish` — both of those break in cargo 1.95
#    with an extra `publish` positional appearing in the
#    alias expansion. See `.cargo/config.toml`'s comment on
#    the alias for the empirical matrix and the workaround.)
#
#    The two `--config` overrides together let a publish run
#    under the workspace's `.cargo/config.toml` cooldown-mirror
#    shape (which has `[registry] default = "menhera-cooldown"`
#    and `[source.crates-io].replace-with = "menhera-cooldown"`
#    pointing at the `/3d/` endpoint). The first override aims
#    the publish at `crates-io` so dep-registry identity
#    matches the third-party deps' own `crates-io` tag (cargo
#    otherwise refuses with "dependencies sourced from other
#    registries"). The second redirects the source-replacement
#    chain to the `/0d/` no-cooldown endpoint Yuka runs on the
#    same Menhera proxy, so the verify-build step sees
#    workspace-internal deps published inside the 3-day window
#    (e.g., `philharmonic-connector-common 0.2.0` the next day)
#    instead of failing with "no matching package".
#
#    Packaging the overrides as a cargo alias (rather than
#    passing them here per-invocation) means every entry point
#    to "publish under the workspace" converges on the same
#    flags — `cargo pub-fresh` from a shell works exactly
#    like this script does, and there's one place to maintain
#    the `/0d/` URL if the proxy ever moves.
#
# 2. CARGO_TARGET_DIR=target-publish
#    Verify-build compiles the packaged tarball; without a
#    separate target directory, those artefacts would land in
#    the shared `target/` that normal development (and any
#    concurrent Codex session) depends on. Diverting to
#    `target-publish/` keeps the publish's build isolated —
#    same pattern as `scripts/xtask.sh` uses `target-xtask/`
#    (see `.gitignore` and CONTRIBUTING.md §8.1).
#
# Cargo.lock is NOT modified by this flow — empirically
# verified; the `/3d/` → `/0d/` URL swap produces no
# resolution diff because the resolver still picks identical
# versions and the sparse-index URL is not recorded into the
# lockfile.
#
# Consumers pulling the newly-published crate through the
# workspace's default `/3d/` mirror are still 3-day-delayed
# by design — that is a feature of the cooldown setup, not a
# gap.
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target-publish}"
export CARGO_TARGET_DIR

verify_flag=""
if [ "$no_verify" -eq 1 ]; then
    verify_flag="--no-verify"
    printf '%s=== --no-verify: skipping verify-build (proxy index may be stale) ===%s\n' "$C_WARN" "$C_RESET"
fi

# Sanity: always run --dry-run first (unless --no-verify skips it).
if [ "$no_verify" -eq 0 ]; then
    printf '%s=== cargo pub-fresh --dry-run -p %s ===%s\n' "$C_HEADER" "$crate" "$C_RESET"
    run_with_cargo_noise_filter cargo pub-fresh --dry-run -p "$crate"
fi

if [ "$dry_run" -eq 1 ]; then
    printf '%s=== --dry-run: stopping; no real publish, no tag ===%s\n' "$C_WARN" "$C_RESET"
    exit 0
fi

# shellcheck disable=SC2086
printf '%s=== cargo pub-fresh %s -p %s ===%s\n' "$C_HEADER" "$verify_flag" "$crate" "$C_RESET"
# shellcheck disable=SC2086
run_with_cargo_noise_filter cargo pub-fresh $verify_flag -p "$crate"

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
