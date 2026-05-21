#!/bin/sh
# scripts/chat-release-build.sh — build a release-optimised
# statically-linked musl binary for the philharmonic-chat bin.
#
# Usage:
#   ./scripts/chat-release-build.sh                  # gzip archive (default)
#   ./scripts/chat-release-build.sh --zstd           # zstd archive
#   ./scripts/chat-release-build.sh --no-https       # plaintext-only build
#
# Thin wrapper around `cargo build --release --target
# x86_64-unknown-linux-musl -p philharmonic-chat`. The
# .cargo/config.toml already sets the linker + CC for the musl
# target. The frontend bundle embedded into the bin comes from
# the committed `bins/philharmonic-chat/dist/` — the dirty-tree
# gate below ensures that the embedded bundle corresponds to the
# commit named in the archive's short HEAD SHA.
#
# Prerequisites:
#   - rustup target x86_64-unknown-linux-musl (setup.sh adds it)
#   - musl-tools (Debian/Ubuntu: apt install musl-tools)
#
# Output:
#   target-release/x86_64-unknown-linux-musl/release/philharmonic-chat
#   target-release/x86_64-unknown-linux-musl/release/philharmonic-chat-<sha>.tar.{gz|zst}
#
# POSIX sh — see CONTRIBUTING.md §6.

set -eu

. "$(dirname -- "$0")/lib/script-help.sh"
script_help_handle "$@"


. "$(dirname -- "$0")/lib/colors.sh"
. "$(dirname -- "$0")/lib/workspace-cd.sh"
. "$(dirname -- "$0")/lib/cargo-noise-filter.sh"

CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target-release}"
export CARGO_TARGET_DIR

TARGET="x86_64-unknown-linux-musl"
PKG="philharmonic-chat"
BIN="philharmonic-chat"
archive_comp="gzip"
with_https=1

# Abort if the workspace has any uncommitted changes — parent
# file edits, parent untracked files, submodule pointer
# movement, or submodule content dirtiness (modified / staged /
# untracked inside any submodule). `git status --porcelain
# --ignore-submodules=none` summarises all of these from the
# parent in one pass; non-empty output means dirty.
#
# Called at two points: start (so we never build a release off
# a dirty tree) and archive (so the archive name's embedded
# `git rev-parse --short HEAD` actually identifies the binary's
# source — a dirty tree between build and archive would emit a
# `philharmonic-chat-<sha>.tar.gz` that doesn't correspond to
# <sha>).
#
# No escape hatch: release archives must be reproducible from a
# named commit. If you want a debug build off a dirty tree, use
# `cargo build` directly, not this wrapper.
check_clean_or_die() {
    cd_label=$1
    cd_dirty=$(git -c diff.ignoreSubmodules=none status \
        --porcelain --ignore-submodules=none 2>/dev/null)
    if [ -n "$cd_dirty" ]; then
        printf '\n%s!!! chat-release-build aborted (%s gate): %s%s\n' \
            "$C_ERR" "$cd_label" "working tree is dirty" "$C_RESET" >&2
        printf '\n%s\n\n' "$cd_dirty" >&2
        printf '    Run ./scripts/status.sh for the full picture\n' >&2
        printf '    across parent + every submodule. Commit or\n' >&2
        printf '    stash every change, then re-run chat-release-build.\n' >&2
        exit 1
    fi
}

while [ $# -gt 0 ]; do
    case "$1" in
        --zstd)
            archive_comp="zstd"
            shift
            ;;
        --gzip)
            archive_comp="gzip"
            shift
            ;;
        --https)
            with_https=1
            shift
            ;;
        --no-https)
            with_https=0
            shift
            ;;
        --help|-h)
            cat <<EOF
Usage: $0 [--gzip|--zstd] [--https|--no-https]

Build a release-optimised, statically-linked musl binary for
the philharmonic-chat bin and archive it alongside.

Options:
  --gzip          Archive with gzip (default).
  --zstd          Archive with zstd (parallel, faster on
                  large binaries).
  --https         Build with the \`https\` feature enabled —
                  the chat bin terminates TLS itself.
                  Default ON for release builds.
  --no-https      Build without the \`https\` feature.
                  Release binary serves plaintext only; a
                  reverse proxy must terminate TLS upstream.

Dirty-tree gates (no escape hatch):
  - Start gate aborts the build if the workspace has any
    uncommitted changes (parent file edits, parent untracked
    files, submodule pointer movement, or content dirtiness
    inside any submodule).
  - Archive gate re-checks after the build completes so the
    archive name's embedded short HEAD SHA actually
    identifies the binary's source — a tree that goes dirty
    mid-build would otherwise emit a misleading
    philharmonic-chat-<sha>.tar(.gz|.zst).
  Run ./scripts/status.sh to see what's dirty; commit or
  stash before re-running.

Prerequisites:
  apt install musl-tools
  rustup target add x86_64-unknown-linux-musl

The frontend bundle baked into the binary is the committed
\`bins/philharmonic-chat/dist/\` content as of HEAD. Rebuild
the frontend with ./scripts/philharmonic-chat-build.sh
--production and commit the artifacts before tagging a
release if the source has changed since the last dist
commit.
EOF
            exit 0
            ;;
        *)
            printf '%sunknown argument: %s%s\n' "$C_ERR" "$1" "$C_RESET" >&2
            exit 2
            ;;
    esac
done

check_clean_or_die "start"

if [ "$with_https" = "1" ]; then
    features_arg="--features https"
else
    features_arg=""
fi

if ! command -v x86_64-linux-musl-gcc >/dev/null 2>&1; then
    printf '%s!!! x86_64-linux-musl-gcc not found.%s\n' "$C_ERR" "$C_RESET" >&2
    printf '    Install musl-tools: apt install musl-tools\n' >&2
    exit 1
fi

printf '%s=== chat release build (musl) ===%s\n' "$C_HEADER" "$C_RESET"
printf '%s    target: %s%s\n' "$C_NOTE" "$TARGET" "$C_RESET"
if [ "$with_https" = "1" ]; then
    printf '%s    https: enabled (--features https)%s\n' \
        "$C_NOTE" "$C_RESET"
else
    printf '%s    https: disabled (--no-https; TLS must be terminated upstream)%s\n' \
        "$C_NOTE" "$C_RESET"
fi

printf '%s    building %s ...%s\n' "$C_NOTE" "$PKG" "$C_RESET"
# shellcheck disable=SC2086
run_with_cargo_noise_filter cargo build --release --target "$TARGET" -p "$PKG" --bins $features_arg

out_dir="$CARGO_TARGET_DIR/$TARGET/release"
printf '\n%s=== chat release build complete ===%s\n' "$C_OK" "$C_RESET"
printf '    Output: %s/\n' "$out_dir"

if [ ! -f "$out_dir/$BIN" ]; then
    printf '%s!!! expected output missing: %s/%s%s\n' \
        "$C_ERR" "$out_dir" "$BIN" "$C_RESET" >&2
    exit 1
fi

size=$(ls -lh "$out_dir/$BIN" | awk '{print $5}')
printf '    %s (%s)\n' "$BIN" "$size"

# Archive the built binary using the in-tree Rust archiver
# (xtask tar-archive). Supports --gzip (default) and --zstd.
check_clean_or_die "archive"

short_hash=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")

case "$archive_comp" in
    gzip) archive_name="philharmonic-chat-${short_hash}.tar.gz"; comp_flag="--gzip" ;;
    zstd) archive_name="philharmonic-chat-${short_hash}.tar.zst"; comp_flag="--zstd" ;;
    *)    printf '%s!!! unknown archive_comp: %s%s\n' "$C_ERR" "$archive_comp" "$C_RESET" >&2; exit 2 ;;
esac

archive_path="$out_dir/$archive_name"

printf '\n%s=== archive (%s) ===%s\n' "$C_HEADER" "$archive_comp" "$C_RESET"
./scripts/xtask.sh tar-archive -- -o "$archive_path" "$comp_flag" "$out_dir/$BIN"
