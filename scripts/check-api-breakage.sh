#!/bin/sh
# scripts/check-api-breakage.sh — run cargo-semver-checks across the
# workspace against an explicit baseline git revision.
#
# Usage:
#   ./scripts/check-api-breakage.sh [baseline-rev]
#
# baseline-rev defaults to origin/main. Pass a release tag (e.g.
# v0.2.2) or any commit-ish to compare against a different point.
#
# Installs cargo-semver-checks via `cargo install --locked` if it's
# not already on PATH. The first invocation is therefore slower; re-
# runs only pay the actual semver-check cost.
#
# This is not part of the pre-landing trio (fmt/clippy/test). Run it
# before preparing a crate release; see docs/design/13-conventions.md
# §API breakage detection.
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu

cd "$(git rev-parse --show-toplevel)"

baseline=${1:-origin/main}

if ! git rev-parse --verify "$baseline" >/dev/null 2>&1; then
    printf 'check-api-breakage.sh: baseline ref %s not found.\n' "$baseline" >&2
    printf '   Try `git fetch --tags origin` first, or pass an existing ref.\n' >&2
    exit 1
fi

# cargo-semver-checks is a one-off install for the developer's
# environment; `cargo install --locked` avoids resolver drift.
if ! command -v cargo-semver-checks >/dev/null 2>&1; then
    echo "=== installing cargo-semver-checks ==="
    cargo install --locked cargo-semver-checks
fi

echo "=== cargo semver-checks --workspace --all-features --baseline-rev $baseline ==="
cargo semver-checks \
    --workspace \
    --all-features \
    --baseline-rev "$baseline"
