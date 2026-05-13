#!/bin/sh
# Workspace housekeeping: remove `target-main/debug` to free tmpfs.
#
# On hosts where `scripts/lib/cargo-target-dir.sh` has set up the
# `target-main` -> `/tmp/philharmonic-$(id -u)-target-main`
# symlink (RAM-backed tmpfs), the debug build cache is the bulk
# of `/tmp` usage. Run this script when `/tmp` is filling up to
# free that bulk without touching:
#
#   - target-main/release  (release builds)
#   - target-main/doc      (rustdoc output)
#   - target-main/<other>  (anything else cargo wrote)
#   - target-xtask/        (xtask's separate target dir;
#                           the user-facing tmpfs symlink only
#                           covers target-main)
#   - the cargo registry / git cache (under $CARGO_HOME)
#
# Cost: the next debug build is a cold rebuild (slow first
# compile, fast incrementals after). Acceptable trade vs. a
# full `/tmp` exhaustion that breaks every subsequent cargo
# run.
#
# Usage:
#   ./scripts/clean-target-debug.sh
#
# POSIX sh only — see CONTRIBUTING.md §6.

set -eu

WORKSPACE_ROOT=$(cd "$(dirname "$0")/.." && pwd -P)
cd "$WORKSPACE_ROOT"

if [ ! -e target-main/debug ] && [ ! -L target-main/debug ]; then
    echo "target-main/debug is not present — nothing to clean."
    exit 0
fi

echo "=== before ==="
df -Pk /tmp 2>/dev/null | head -2 || true
du -sh target-main/debug 2>/dev/null || true
echo

echo "=== removing target-main/debug ==="
rm -rf target-main/debug

echo
echo "=== after ==="
df -Pk /tmp 2>/dev/null | head -2 || true
echo
echo "Done. The next debug build will rebuild from scratch;"
echo "release / doc / xtask builds are unaffected."
