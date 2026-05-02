#!/bin/sh
# scripts/release-build.sh — build release-optimised statically-linked
# musl binaries for the philharmonic meta-crate's bin targets.
#
# Usage:
#   ./scripts/release-build.sh                      # all bins, gzip archive
#   ./scripts/release-build.sh --zstd               # all bins, zstd archive
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
. "$(dirname -- "$0")/lib/cargo-noise-filter.sh"

CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target-release}"
export CARGO_TARGET_DIR

TARGET="x86_64-unknown-linux-musl"
bin_args=""
archive_comp="gzip"

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
        --zstd)
            archive_comp="zstd"
            shift
            ;;
        --gzip)
            archive_comp="gzip"
            shift
            ;;
        --help|-h)
            cat <<EOF
Usage: $0 [--bin <name>]... [--gzip|--zstd]

Build release-optimised, statically-linked musl binaries.

Options:
  --bin <name>    Build only the named binary (repeatable).
                  Known bins: mechanics-worker,
                  philharmonic-connector, philharmonic-api
  --gzip          Archive with gzip (default).
  --zstd          Archive with zstd (parallel, faster on
                  large binaries like the 2.2 GB connector).

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
    bin_args="mechanics-worker philharmonic-connector-bin philharmonic-api-server"
fi

printf '%s=== release build (musl) ===%s\n' "$C_HEADER" "$C_RESET"
printf '%s    target: %s%s\n' "$C_NOTE" "$TARGET" "$C_RESET"

# Build each bin SEPARATELY so Cargo doesn't unify features
# across crates. Building together causes philharmonic-connector-bin's
# default features (connector-embed with 2.28 GB model weights) to
# bleed into the other bins via the shared philharmonic dependency.
for pkg in $bin_args; do
    printf '%s    building %s ...%s\n' "$C_NOTE" "$pkg" "$C_RESET"
    run_with_cargo_noise_filter cargo build --release --target "$TARGET" -p "$pkg" --bins
done

out_dir="$CARGO_TARGET_DIR/$TARGET/release"
printf '\n%s=== release build complete ===%s\n' "$C_OK" "$C_RESET"
printf '    Output: %s/\n' "$out_dir"

bins_found=""
for bin in mechanics-worker philharmonic-connector philharmonic-api; do
    if [ -f "$out_dir/$bin" ]; then
        size=$(ls -lh "$out_dir/$bin" | awk '{print $5}')
        printf '    %s (%s)\n' "$bin" "$size"
        bins_found="$bins_found $bin"
    fi
done

# Archive the built binaries using the in-tree Rust archiver
# (xtask tar-archive). Supports --gzip (default) and --zstd.
# The Rust archiver uses parallel zstd via zstdmt when --zstd
# is selected, which is significantly faster than single-
# threaded gzip on the 2.2 GB connector binary.
if [ -n "$bins_found" ]; then
    short_hash=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")

    case "$archive_comp" in
        gzip) archive_name="philharmonic-${short_hash}.tar.gz"; comp_flag="--gzip" ;;
        zstd) archive_name="philharmonic-${short_hash}.tar.zst"; comp_flag="--zstd" ;;
        *)    printf '%s!!! unknown archive_comp: %s%s\n' "$C_ERR" "$archive_comp" "$C_RESET" >&2; exit 2 ;;
    esac

    archive_path="$out_dir/$archive_name"

    # Build file-path arguments from the found bins.
    file_args=""
    for bin in $bins_found; do
        file_args="$file_args $out_dir/$bin"
    done

    printf '\n%s=== archive (%s) ===%s\n' "$C_HEADER" "$archive_comp" "$C_RESET"
    # shellcheck disable=SC2086
    ./scripts/xtask.sh tar-archive -- -o "$archive_path" $comp_flag $file_args
fi
