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
    cargo build --release --target "$TARGET" -p "$pkg"
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

# Archive the built binaries into a gzip-compressed tarball.
# gzip is a deviation from strict POSIX (SUSv4 specifies
# `compress`, not `gzip`), but gzip is more widely available
# on real systems than `compress -m gzip`. Fall back to
# `compress -m gzip` if gzip is absent.
if [ -n "$bins_found" ]; then
    short_hash=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
    archive_name="philharmonic-${short_hash}.tar.gz"

    if command -v gzip >/dev/null 2>&1; then
        gz_cmd="gzip"
    elif compress -m gzip </dev/null >/dev/null 2>&1; then
        gz_cmd="compress -m gzip"
    else
        printf '%s!!! neither gzip nor compress -m gzip found; skipping archive%s\n' \
            "$C_WARN" "$C_RESET" >&2
        bins_found=""
    fi

    if [ -n "$bins_found" ]; then
        # shellcheck disable=SC2086
        (cd "$out_dir" && tar cf - -- $bins_found | $gz_cmd > "$archive_name")
        archive_path=$(cd "$out_dir" && pwd -P)/"$archive_name"
        archive_size=$(ls -lh "$archive_path" | awk '{print $5}')
        printf '\n%s=== archive ===%s\n' "$C_HEADER" "$C_RESET"
        printf '    %s (%s)\n' "$archive_path" "$archive_size"
    fi
fi
