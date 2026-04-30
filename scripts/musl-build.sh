#!/bin/sh
# scripts/musl-build.sh — build statically-linked musl binaries
# for all (or selected) bin targets in the philharmonic meta-crate.
#
# Usage:
#   ./scripts/musl-build.sh                          # all bins, debug
#   ./scripts/musl-build.sh --release                # all bins, release
#   ./scripts/musl-build.sh --bin mechanics-worker   # one bin
#   ./scripts/musl-build.sh --release --bin philharmonic-api
#
# Prerequisites:
#   - rustup target x86_64-unknown-linux-musl (setup.sh adds it)
#   - musl-tools (Debian/Ubuntu: apt install musl-tools)
#     Provides x86_64-linux-musl-gcc, needed by aws-lc-rs's
#     vendored C compilation via cc-rs.
#
# The .cargo/config.toml already sets the linker and CC env var
# for the musl target, so no manual env overrides are needed.
#
# Output lands in target-main/x86_64-unknown-linux-musl/{debug,release}/.
#
# POSIX sh — see CONTRIBUTING.md §6.

set -eu

. "$(dirname -- "$0")/lib/colors.sh"
. "$(dirname -- "$0")/lib/workspace-cd.sh"
. "$(dirname -- "$0")/lib/cargo-target-dir.sh"

TARGET="x86_64-unknown-linux-musl"
release_flag=""
profile="debug"
bin_args=""

while [ $# -gt 0 ]; do
    case "$1" in
        --release)
            release_flag="--release"
            profile="release"
            shift
            ;;
        --bin)
            if [ $# -lt 2 ]; then
                printf '%s!!! --bin requires a value%s\n' "$C_ERR" "$C_RESET" >&2
                exit 2
            fi
            bin_args="$bin_args --bin $2"
            shift 2
            ;;
        --help|-h)
            cat <<EOF
Usage: $0 [--release] [--bin <name>]...

Build statically-linked musl binaries for the philharmonic meta-crate.

Options:
  --release       Build in release mode (optimized, stripped)
  --bin <name>    Build only the named binary (repeatable)
                  Known bins: mechanics-worker, philharmonic-connector,
                  philharmonic-api

Without --bin, all three bins are built.

Prerequisites:
  apt install musl-tools   (provides x86_64-linux-musl-gcc)
  rustup target add x86_64-unknown-linux-musl
EOF
            exit 0
            ;;
        *)
            printf '%sunknown argument: %s%s\n' "$C_ERR" "$1" "$C_RESET" >&2
            exit 2
            ;;
    esac
done

# Guard: musl-gcc must be available
if ! command -v x86_64-linux-musl-gcc >/dev/null 2>&1; then
    printf '%s!!! x86_64-linux-musl-gcc not found.%s\n' "$C_ERR" "$C_RESET" >&2
    printf '    Install musl-tools: apt install musl-tools\n' >&2
    exit 1
fi

# Default: all three bins
if [ -z "$bin_args" ]; then
    bin_args="--bin mechanics-worker --bin philharmonic-connector --bin philharmonic-api"
fi

printf '%s=== musl build (%s) ===%s\n' "$C_HEADER" "$profile" "$C_RESET"
# shellcheck disable=SC2086
printf '%s    target: %s%s\n' "$C_NOTE" "$TARGET" "$C_RESET"
printf '%s    bins:   %s%s\n' "$C_NOTE" "$bin_args" "$C_RESET"

# shellcheck disable=SC2086
cargo build --target "$TARGET" -p philharmonic $release_flag $bin_args

out_dir="$CARGO_TARGET_DIR/$TARGET/$profile"
printf '\n%s=== musl build complete ===%s\n' "$C_OK" "$C_RESET"
printf '    Output: %s/\n' "$out_dir"

for bin in mechanics-worker philharmonic-connector philharmonic-api; do
    if [ -f "$out_dir/$bin" ]; then
        size=$(ls -lh "$out_dir/$bin" | awk '{print $5}')
        printf '    %s (%s)\n' "$bin" "$size"
    fi
done
