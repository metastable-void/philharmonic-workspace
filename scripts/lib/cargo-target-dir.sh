# scripts/lib/cargo-target-dir.sh — sourced helper that sets
# CARGO_TARGET_DIR to `target-main` when it isn't already set.
#
# Expected usage (near the top of scripts/<name>.sh, after
# `set -eu` and `. workspace-cd.sh`):
#
#     . "$(dirname -- "$0")/lib/cargo-target-dir.sh"
#
# Purpose: keep CLI / Codex cargo builds in `target-main/` so
# they don't contend with `rust-analyzer`'s use of `target/`.
# Without this, concurrent builds fight over
# `target/debug/.cargo-lock` and produce spurious lock-wait
# stalls and "could not compile" errors.
#
# Scripts that need a different target dir (e.g. xtask.sh uses
# `target-xtask`) should set CARGO_TARGET_DIR themselves
# *before* sourcing this snippet — the guard below preserves
# any existing value.
#
# Sourced — not executed directly. No shebang on purpose.
#
# POSIX sh only — see CONTRIBUTING.md §6.

CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target-main}"
export CARGO_TARGET_DIR
