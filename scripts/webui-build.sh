#!/bin/sh
# scripts/webui-build.sh — build the WebUI for embedding into the
# philharmonic-api binary.
#
# Usage:
#   ./scripts/webui-build.sh [--production]
#
# EXCEPTION TO THE NO-NODE.JS RULE (CONTRIBUTING.md §7):
# This is the ONLY script in the workspace that invokes Node.js
# (via npx/webpack). Node.js is used here solely to produce the
# four committed build artifacts (index.html, main.js, main.css,
# icon.svg) that the Rust binary embeds at compile time. General
# Node.js usage remains forbidden in workspace tooling — see §7.
#
# Reproducibility:
# - The Webpack build cache is removed before every run so that
#   builds are fully deterministic from source. Stale cache is
#   the #1 cause of "it built differently on my machine."
# - Output goes to a fixed directory (philharmonic/webui/dist/)
#   that is committed to Git. The Rust binary includes these
#   files via include_bytes! or rust-embed at compile time, so
#   no Node.js is needed to build the Rust crate.
#
# Prerequisites:
# - Node.js (LTS) on PATH.
# - npm on PATH (ships with Node.js).
# - Run `npm ci` in philharmonic/webui/ at least once to
#   install deps.
#
# POSIX sh — see CONTRIBUTING.md §6.

set -eu

. "$(dirname -- "$0")/lib/colors.sh"
. "$(dirname -- "$0")/lib/workspace-cd.sh"

webui_dir="$workspace_root/philharmonic/webui"
dist_dir="$webui_dir/dist"

# ── Guard: Node.js must be available ─────────────────────────
if ! command -v node >/dev/null 2>&1; then
    printf '%s!!! node not found on PATH.%s\n' "$C_ERR" "$C_RESET" >&2
    printf '    Install Node.js (LTS) to build the WebUI.\n' >&2
    printf '    This is the ONLY workspace operation that needs Node.js.\n' >&2
    exit 1
fi

if ! command -v npm >/dev/null 2>&1; then
    printf '%s!!! npm not found on PATH.%s\n' "$C_ERR" "$C_RESET" >&2
    exit 1
fi

# ── Guard: webui/ must exist ─────────────────────────────────
if [ ! -d "$webui_dir" ]; then
    printf '%s!!! webui/ directory not found at %s%s\n' \
        "$C_ERR" "$webui_dir" "$C_RESET" >&2
    printf '    The WebUI source tree has not been created yet.\n' >&2
    exit 1
fi

# ── Parse flags ──────────────────────────────────────────────
mode="development"
while [ $# -gt 0 ]; do
    case "$1" in
        --production) mode="production"; shift ;;
        --) shift; break ;;
        -*) printf 'unknown flag: %s\n' "$1" >&2; exit 2 ;;
        *) break ;;
    esac
done

printf '%s=== WebUI build (mode: %s) ===%s\n' "$C_HEADER" "$mode" "$C_RESET"

# ── Install deps if node_modules is missing ──────────────────
if [ ! -d "$webui_dir/node_modules" ]; then
    printf '%s=== npm ci ===%s\n' "$C_HEADER" "$C_RESET"
    (cd "$webui_dir" && npm ci)
fi

# ── Remove build cache for reproducibility ───────────────────
# Webpack's filesystem cache (and any other caching layer) is
# wiped before every build. This guarantees identical source →
# identical output, regardless of prior build state.
cache_dir="$webui_dir/.cache"
if [ -d "$cache_dir" ]; then
    rm -rf "$cache_dir"
    printf '%s    removed %s%s\n' "$C_DIM" "$cache_dir" "$C_RESET"
fi
# Also remove webpack's default cache location inside node_modules
nm_cache="$webui_dir/node_modules/.cache"
if [ -d "$nm_cache" ]; then
    rm -rf "$nm_cache"
    printf '%s    removed %s%s\n' "$C_DIM" "$nm_cache" "$C_RESET"
fi

# ── Build ────────────────────────────────────────────────────
printf '%s=== npx webpack --mode %s ===%s\n' "$C_HEADER" "$mode" "$C_RESET"
(cd "$webui_dir" && NODE_ENV="$mode" npx webpack --mode "$mode")

# ── Verify expected artifacts ────────────────────────────────
ok=1
for f in index.html main.js main.css icon.svg; do
    if [ ! -f "$dist_dir/$f" ]; then
        printf '%s!!! missing expected artifact: %s/dist/%s%s\n' \
            "$C_ERR" "philharmonic/webui" "$f" "$C_RESET" >&2
        ok=0
    fi
done

if [ "$ok" -eq 0 ]; then
    printf '%s!!! Build produced incomplete output.%s\n' "$C_ERR" "$C_RESET" >&2
    exit 1
fi

printf '%s=== WebUI build complete ===%s\n' "$C_OK" "$C_RESET"
printf '    Artifacts in philharmonic/webui/dist/:\n'
# shellcheck disable=SC2012
ls -lh "$dist_dir/" | tail -n +2

echo
printf '%sCommit the artifacts with ./scripts/commit-all.sh when ready.%s\n' \
    "$C_NOTE" "$C_RESET"
