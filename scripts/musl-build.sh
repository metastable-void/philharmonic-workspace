#!/bin/sh
# scripts/musl-build.sh — build statically-linked musl binaries
# (DEBUG profile) for all (or selected) bin targets in the
# philharmonic meta-crate.
#
# This is for debugging and quick verification only. For release
# builds use ./scripts/release-build.sh instead.
#
# Usage:
#   ./scripts/musl-build.sh                          # all bins
#   ./scripts/musl-build.sh --bin mechanics-worker   # one bin
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
# Output lands in target-main/x86_64-unknown-linux-musl/debug/.
#
# POSIX sh — see CONTRIBUTING.md §6.

set -eu

. "$(dirname -- "$0")/lib/colors.sh"
. "$(dirname -- "$0")/lib/workspace-cd.sh"
. "$(dirname -- "$0")/lib/cargo-target-dir.sh"

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
                mechanics-worker)       bin_args="$bin_args mechanics-worker" ;;
                philharmonic-connector) bin_args="$bin_args philharmonic-connector-bin" ;;
                philharmonic-api)       bin_args="$bin_args philharmonic-api-server" ;;
                *) printf '%sunknown bin: %s%s\n' "$C_ERR" "$2" "$C_RESET" >&2; exit 2 ;;
            esac
            shift 2
            ;;
        --help|-h)
            cat <<EOF
Usage: $0 [--bin <name>]...

Build debug musl binaries. For release builds use
./scripts/release-build.sh instead.

Options:
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
    bin_args="mechanics-worker philharmonic-connector-bin philharmonic-api-server"
fi

printf '%s=== musl build (debug) ===%s\n' "$C_HEADER" "$C_RESET"
printf '%s    target: %s%s\n' "$C_NOTE" "$TARGET" "$C_RESET"

# Build each bin SEPARATELY — see release-build.sh for rationale.
for pkg in $bin_args; do
    printf '%s    building %s ...%s\n' "$C_NOTE" "$pkg" "$C_RESET"
    cargo build --target "$TARGET" -p "$pkg"
done

out_dir="$CARGO_TARGET_DIR/$TARGET/debug"
printf '\n%s=== musl build complete ===%s\n' "$C_OK" "$C_RESET"
printf '    Output: %s/\n' "$out_dir"

for bin in mechanics-worker philharmonic-connector philharmonic-api; do
    if [ -f "$out_dir/$bin" ]; then
        size=$(ls -lh "$out_dir/$bin" | awk '{print $5}')
        printf '    %s (%s)\n' "$bin" "$size"
    fi
done
