#!/bin/sh
# scripts/cargo-audit.sh — run `cargo audit` across the workspace,
# auto-installing `cargo-audit` via `cargo install --locked` on
# first use. Flags vulnerable dependencies in `Cargo.lock` against
# the RustSec advisory database.
#
# Usage:
#   ./scripts/cargo-audit.sh [extra-args-forwarded-to-cargo-audit...]
#
# When to run:
#   - Before preparing a crate release (alongside
#     `check-api-breakage.sh`).
#   - Periodically during active development.
#   - Any time a dependency is bumped.
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu
. "$(dirname -- "$0")/lib/workspace-cd.sh"
. "$(dirname -- "$0")/lib/cargo-target-dir.sh"

if ! command -v cargo-audit >/dev/null 2>&1; then
    echo '=== installing cargo-audit ==='
    cargo install --locked cargo-audit
fi

printf '=== cargo audit %s ===\n' "$*"
# $@ is expanded as separate words; no --workspace (cargo-audit
# reads Cargo.lock at the root, which already covers every
# workspace member transitively).
cargo audit "$@"
