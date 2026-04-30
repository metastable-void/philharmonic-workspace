#!/bin/sh
# scripts/release-build.sh — build release-optimised statically-linked
# musl binaries for the philharmonic meta-crate's bin targets.
#
# Usage:
#   ./scripts/release-build.sh                      # all bins
#   ./scripts/release-build.sh --bin mechanics-worker
#   ./scripts/release-build.sh --bin philharmonic-api --bin philharmonic-connector
#
# This is a thin wrapper around `cargo build --release --target
# x86_64-unknown-linux-musl -p philharmonic`. The .cargo/config.toml
# already sets the linker + CC for the musl target.
#
# Prerequisites:
#   - rustup target x86_64-unknown-linux-musl (setup.sh adds it)
#   - musl-tools (Debian/Ubuntu: apt install musl-tools)
#
# Output: target-release/x86_64-unknown-linux-musl/release/
#
# POSIX sh — see CONTRIBUTING.md §6.

set -eu

. "$(dirname -- "$0")/lib/colors.sh"
. "$(dirname -- "$0")/lib/workspace-cd.sh"

CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target-release}"
export CARGO_TARGET_DIR

TARGET="x86_64-unknown-linux-musl"
bin_args=""

while [ $# -gt 0 ]; do
    case "$1" in
        --bin)
            if [ $# -lt 2 ]; then
                printf '%s!!! --bin requires a value%s\n' "$C_ERR" "$C_RESET" >&2
                exit 2
            fi
            case "$2" in
                mechanics-worker)       bin_args="$bin_args -p mechanics-worker" ;;
                philharmonic-connector) bin_args="$bin_args -p philharmonic-connector-bin" ;;
                philharmonic-api)       bin_args="$bin_args -p philharmonic-api-server" ;;
                *) printf '%sunknown bin: %s%s\n' "$C_ERR" "$2" "$C_RESET" >&2; exit 2 ;;
            esac
            shift 2
            ;;
        --help|-h)
            cat <<EOF
Usage: $0 [--bin <name>]...

Build release-optimised, statically-linked musl binaries.

Options:
  --bin <name>    Build only the named binary (repeatable).
                  Known bins: mechanics-worker,
                  philharmonic-connector, philharmonic-api

Without --bin, all three bins are built.

Prerequisites:
  apt install musl-tools
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

if ! command -v x86_64-linux-musl-gcc >/dev/null 2>&1; then
    printf '%s!!! x86_64-linux-musl-gcc not found.%s\n' "$C_ERR" "$C_RESET" >&2
    printf '    Install musl-tools: apt install musl-tools\n' >&2
    exit 1
fi

if [ -z "$bin_args" ]; then
    bin_args="-p mechanics-worker -p philharmonic-connector-bin -p philharmonic-api-server"
fi

printf '%s=== release build (musl) ===%s\n' "$C_HEADER" "$C_RESET"
printf '%s    target: %s%s\n' "$C_NOTE" "$TARGET" "$C_RESET"

# shellcheck disable=SC2086
cargo build --release --target "$TARGET" $bin_args

out_dir="$CARGO_TARGET_DIR/$TARGET/release"
printf '\n%s=== release build complete ===%s\n' "$C_OK" "$C_RESET"
printf '    Output: %s/\n' "$out_dir"

for bin in mechanics-worker philharmonic-connector philharmonic-api; do
    if [ -f "$out_dir/$bin" ]; then
        size=$(ls -lh "$out_dir/$bin" | awk '{print $5}')
        printf '    %s (%s)\n' "$bin" "$size"
    fi
done
