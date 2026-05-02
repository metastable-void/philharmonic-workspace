#!/bin/sh
# scripts/check-api-breakage.sh — run cargo-semver-checks for one
# workspace crate against a crates.io baseline.
#
# Usage:
#   ./scripts/check-api-breakage.sh <crate>              # baseline = latest on crates.io
#   ./scripts/check-api-breakage.sh <crate> <version>    # explicit baseline version
#
# Reports whether the in-tree <crate> has introduced API breakage
# relative to the baseline. A clean run ends with "no semver update
# required"; a breaking change shows "requires new major version"
# and lists the failing checks. Non-zero exit on breakage.
#
# Omit <version> to let cargo-semver-checks query crates.io for the
# most-recently-published version of <crate> and use that as the
# baseline. An explicit <version> (e.g. `0.2.2`) is useful when you
# want to compare against a specific historical release rather
# than whatever happens to be newest.
#
# Installs cargo-semver-checks via `cargo install --locked` if it's
# not already on PATH. The first invocation is therefore slower;
# re-runs only pay the actual semver-check cost.
#
# Run this before preparing a release for <crate>; see
# docs/design/13-conventions.md §API breakage detection. Not part
# of the pre-landing trio (fmt/clippy/test) — those check the
# current tree, this one checks the release contract.
#
# Why per-crate (not `--workspace --baseline-rev`): the parent
# repo here is a virtual workspace (no [package] table) composed
# of submodules. `cargo semver-checks --baseline-rev <rev>`
# `git clone`s the parent at <rev> to build the baseline, but that
# clone does not recurse into submodules, so workspace members
# don't exist at the baseline root and the tool aborts with
# "no `package` table" / "package not found". Per-crate mode with
# `--baseline-version` (or the tool's default, which resolves to
# the latest on crates.io) sidesteps the problem entirely: the
# baseline source comes from the registry, not a git clone of
# this repo.
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu

. "$(dirname -- "$0")/lib/workspace-cd.sh"
. "$(dirname -- "$0")/lib/cargo-target-dir.sh"
. "$(dirname -- "$0")/lib/cargo-noise-filter.sh"

if [ $# -lt 1 ] || [ $# -gt 2 ] || [ -z "$1" ]; then
    echo "Usage: $0 <crate> [<baseline-version>]" >&2
    exit 2
fi

crate=$1
baseline_version=${2:-}

# Validate that <crate> is an actual workspace member — gives a
# clear error up front rather than letting cargo fail opaquely.
# `crate-version.sh` exits non-zero with a pointed message if
# $crate/Cargo.toml is missing.
./scripts/crate-version.sh "$crate" >/dev/null

# cargo-semver-checks is a one-off install for the developer's
# environment; `cargo install --locked` avoids resolver drift.
if ! command -v cargo-semver-checks >/dev/null 2>&1; then
    echo "=== installing cargo-semver-checks ==="
    cargo install --locked cargo-semver-checks
fi

if [ -n "$baseline_version" ]; then
    printf '=== cargo semver-checks check-release -p %s --baseline-version %s ===\n' \
        "$crate" "$baseline_version"
    run_with_cargo_noise_filter cargo --color=always semver-checks check-release \
        -p "$crate" \
        --baseline-version "$baseline_version"
    exit $?
fi

# No explicit baseline: the tool queries crates.io for the newest
# published version of <crate> and uses that. Fails cleanly if the
# crate has never been published (no baseline to compare against).
printf '=== cargo semver-checks check-release -p %s (baseline = latest on crates.io) ===\n' \
    "$crate"
run_with_cargo_noise_filter cargo --color=always semver-checks check-release -p "$crate"
