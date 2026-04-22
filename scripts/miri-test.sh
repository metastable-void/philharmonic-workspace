#!/bin/sh
# scripts/miri-test.sh — run `cargo +nightly miri test` for routine
# undefined-behavior checking.
#
# Miri catches classes of bugs regular `cargo test` misses:
# uninitialized memory reads, out-of-bounds pointer arithmetic,
# invalid `mem::transmute` / `mem::uninitialized` usage, data
# races, type-layout confusion, and stacked-borrows violations in
# `unsafe` code. Even though this workspace bans `unsafe` in
# library code (docs/design/13-conventions.md §Panics and
# undefined behavior), miri is cheap insurance against UB smuggled
# in through dependencies or test harnesses.
#
# Usage:
#   ./scripts/miri-test.sh --workspace                 # cargo miri test --workspace
#   ./scripts/miri-test.sh <crate> [<test>...]         # per-crate, optionally
#                                                      # filtered to specific
#                                                      # test names (libtest
#                                                      # substring match)
#   ./scripts/miri-test.sh                             # usage error
#
# Single-crate + test-filter is the canonical form. Miri is slow
# (10–50× cargo test), so the realistic use pattern is "one crate,
# specific tests". The `--workspace` form is a rare escape hatch —
# it almost always fails because most workspace crates use FFI
# (testcontainers, sqlx, networking) that miri can't execute.
#
# Env:
#   MIRIFLAGS — forwarded to miri. Common flags:
#     -Zmiri-disable-isolation         allow filesystem / env access
#     -Zmiri-backtrace=full            verbose backtraces on UB
#   Example:
#     MIRIFLAGS="-Zmiri-disable-isolation" ./scripts/miri-test.sh mechanics-core
#
# Requires: nightly toolchain + miri component on nightly.
#   scripts/setup.sh installs both; scripts/check-toolchain.sh
#   verifies they're present. If miri isn't installed, this
#   script prints the rustup invocation to fix it and exits 2.
#
# Miri is slow (10–50x slowdown vs. cargo test) and cannot exercise
# FFI, inline asm, or most syscalls. Crates that depend on real
# sockets, DB drivers (sqlx), testcontainers, or other I/O won't
# run under miri — scope the invocation to in-memory crates
# (types, store traits, mechanics-config, philharmonic-policy's
# crypto paths) rather than running workspace-wide blindly.
#
# Not called from pre-landing.sh — too slow for per-commit runs.
# Run manually before publishing a crate and on a periodic
# schedule (weekly / before milestones).
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu
. "$(dirname -- "$0")/lib/workspace-cd.sh"

if [ $# -eq 0 ]; then
    cat <<EOF >&2
Usage: $0 --workspace | <crate> [<test>...]

Examples:
  $0 philharmonic-policy                               # all non-ignored tests in the crate
  $0 philharmonic-policy sck_decrypt                   # tests matching "sck_decrypt"
  $0 philharmonic-policy sck_ pht_                     # tests matching either substring
  $0 --workspace                                       # entire workspace (usually fails on FFI crates)

Test-name args are libtest substring filters, forwarded to
\`cargo test\` after \`--\`. Miri is slow; narrow the scope.

Env: MIRIFLAGS forwarded (e.g. -Zmiri-disable-isolation).
EOF
    exit 2
fi

# Verify nightly toolchain is installed.
if ! rustup toolchain list 2>/dev/null | grep -q '^nightly'; then
    cat <<EOF >&2
!!! nightly toolchain not installed.
    Fix with:
      rustup toolchain install nightly
    Or rerun scripts/setup.sh, which installs it for you.
EOF
    exit 2
fi

# Verify miri component is installed on nightly.
if ! rustup component list --toolchain nightly --installed 2>/dev/null | grep -q '^miri'; then
    cat <<EOF >&2
!!! miri component not installed on the nightly toolchain.
    Fix with:
      rustup +nightly component add miri
    Or rerun scripts/setup.sh, which installs it for you.
EOF
    exit 2
fi

# Ensure miri's sysroot is built. First run is slow (2–5 min); subsequent
# runs are a no-op. Stdout/stderr suppressed unless setup fails — miri
# setup is verbose on the happy path.
echo '=== cargo +nightly miri setup (idempotent) ==='
cargo +nightly miri setup >/dev/null

if [ "$1" = "--workspace" ]; then
    if [ $# -gt 1 ]; then
        echo "!!! --workspace takes no further args (got: $*)" >&2
        exit 2
    fi
    echo "=== cargo +nightly miri test --workspace ==="
    cargo +nightly miri test --workspace
else
    crate="$1"
    shift
    if [ $# -eq 0 ]; then
        printf '=== cargo +nightly miri test -p %s ===\n' "$crate"
        cargo +nightly miri test -p "$crate"
    else
        printf '=== cargo +nightly miri test -p %s -- %s ===\n' "$crate" "$*"
        cargo +nightly miri test -p "$crate" -- "$@"
    fi
fi
