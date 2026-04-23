#!/bin/sh
# One-time (or after-fresh-clone) workspace setup.
#
# - Initializes and updates every submodule recursively so the other
#   scripts (status.sh, pull-all.sh, commit-all.sh, push-all.sh) and
#   `cargo check --workspace` have real checkouts to work with.
# - Warns if the Rust toolchain (`cargo`, `rustc`) isn't on PATH.
#   This is just a warning — the script still succeeds so that
#   non-Rust workflows (e.g. doc-only editing) aren't blocked.
#
# Idempotent: safe to rerun at any time.
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu

script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
workspace_root="$(cd -- "$script_dir/.." && pwd)"

if ! git -C "$workspace_root" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    printf 'setup.sh: unable to locate workspace Git repository from script path: %s\n' "$workspace_root" >&2
    exit 1
fi

cd "$workspace_root"
YELLOW=$(printf '\033[33m')
GREEN=$(printf '\033[32m')
BOLD=$(printf '\033[1m')
RESET=$(printf '\033[0m')

warn() {
    printf '%s!!! %s%s\n' "$YELLOW" "$*" "$RESET" >&2
}

ok() {
    printf '%s=== %s%s\n' "$GREEN" "$*" "$RESET"
}

ok "Initializing submodules recursively"
git submodule update --init --recursive

# push.recurseSubmodules=check makes `git push` on the parent refuse
# to advance if any referenced submodule commit is not on its remote.
# push-all.sh's guardrail depends on this being set; we configure it
# locally (not via .gitmodules, which can't carry arbitrary config)
# so a fresh clone is safe from the first push onward.
ok "Configuring push.recurseSubmodules=check"
git config --local push.recurseSubmodules check

ok "Configuring core.hooksPath=.githooks"
git config --local core.hooksPath .githooks

REPO_ROOT=$(pwd -P)
export REPO_ROOT

git submodule foreach --recursive '
set -eu
if [ -z "$REPO_ROOT" ] || [ ! -d "$REPO_ROOT" ] ; then
    echo "Could not find REPO_ROOT, aborting." >&2
    exit 211
fi

. "${REPO_ROOT}/scripts/lib/relpath.sh"
SUBMODULE_ROOT=$(pwd -P)
rel=$( relpath "$SUBMODULE_ROOT" "${REPO_ROOT}/.githooks" )
git config --local core.hooksPath "$rel"
'

chmod +x ./scripts/*.sh ./.githooks/*

ok "Submodule status"
git submodule status --recursive

echo

if command -v cargo >/dev/null 2>&1 && command -v rustc >/dev/null 2>&1; then
    rustc_version="$(rustc --version 2>/dev/null || echo 'unknown')"
    cargo_version="$(cargo --version 2>/dev/null || echo 'unknown')"
    ok "Rust toolchain found"
    printf '  rustc: %s\n' "$rustc_version"
    printf '  cargo: %s\n' "$cargo_version"
else
    warn "Rust toolchain not found on PATH."
    warn "Install rustup from https://rustup.rs/ and ensure"
    warn "\$HOME/.cargo/bin is on PATH before running"
    warn "\`cargo check --workspace\` or related commands."
    warn "(Setup itself succeeded; this is only a heads-up.)"
fi

# Install stable toolchain + nightly with miri. Stable is the primary
# build target; nightly+miri is used by scripts/miri-test.sh for routine
# UB checks (see docs/design/13-conventions.md §Testing — miri). Both
# rustup invocations are idempotent: `toolchain install` does nothing
# if the toolchain is present; `component add` does nothing if the
# component is already installed.
if command -v rustup >/dev/null 2>&1; then
    echo
    ok "Ensuring stable toolchain is installed"
    rustup toolchain install stable --profile minimal
    ok "Ensuring nightly toolchain is installed (for miri)"
    rustup toolchain install nightly --profile minimal
    ok "Ensuring miri is installed on nightly"
    rustup component add miri --toolchain nightly
else
    echo
    warn "rustup not on PATH — skipping toolchain install."
    warn "Install rustup from https://rustup.rs/; then rerun this"
    warn "script to pick up the stable + nightly + miri toolchains."
fi

echo
printf '%sSetup complete.%s Next steps:\n' "$BOLD" "$RESET"
printf '  scripts/status.sh     — see working-tree state\n'
printf '  scripts/pull-all.sh   — update submodules to tracked branches\n'
printf '  cargo check --workspace (once Rust is installed)\n'
printf '  scripts/miri-test.sh <crate> (requires nightly + miri)\n'
