#!/bin/sh
# Build the philharmonic-chat frontend artifacts served by the Rust bin.

set -eu

. "$(dirname -- "$0")/lib/script-help.sh"
script_help_handle "$@"

. "$(dirname -- "$0")/lib/colors.sh"
. "$(dirname -- "$0")/lib/workspace-cd.sh"

frontend_dir="$workspace_root/bins/philharmonic-chat/frontend"
dist_dir="$workspace_root/bins/philharmonic-chat/dist"

if ! command -v node >/dev/null 2>&1; then
    printf '%s!!! node not found on PATH.%s\n' "$C_ERR" "$C_RESET" >&2
    exit 1
fi

if ! command -v npm >/dev/null 2>&1; then
    printf '%s!!! npm not found on PATH.%s\n' "$C_ERR" "$C_RESET" >&2
    exit 1
fi

mode=""
while [ $# -gt 0 ]; do
    case "$1" in
        --production) mode="production"; shift ;;
        --) shift; break ;;
        -*) printf 'unknown flag: %s\n' "$1" >&2; exit 2 ;;
        *) break ;;
    esac
done

if [ -z "$mode" ]; then
    printf '%s!!! --production flag is required.%s\n' "$C_ERR" "$C_RESET" >&2
    printf '    Usage: ./scripts/philharmonic-chat-build.sh --production\n' >&2
    exit 2
fi

printf '%s=== Philharmonic Chat frontend build (mode: %s) ===%s\n' \
    "$C_HEADER" "$mode" "$C_RESET"

if [ ! -d "$frontend_dir" ]; then
    printf '%s!!! chat frontend directory not found: %s%s\n' \
        "$C_ERR" "$frontend_dir" "$C_RESET" >&2
    exit 1
fi

if [ ! -d "$frontend_dir/node_modules" ]; then
    printf '%s=== npm ci ===%s\n' "$C_HEADER" "$C_RESET"
    (cd "$frontend_dir" && npm ci)
fi

cache_dir="$frontend_dir/.cache"
if [ -d "$cache_dir" ]; then
    rm -rf "$cache_dir"
    printf '%s    removed %s%s\n' "$C_DIM" "$cache_dir" "$C_RESET"
fi

nm_cache="$frontend_dir/node_modules/.cache"
if [ -d "$nm_cache" ]; then
    rm -rf "$nm_cache"
    printf '%s    removed %s%s\n' "$C_DIM" "$nm_cache" "$C_RESET"
fi

printf '%s=== npx webpack --mode %s ===%s\n' "$C_HEADER" "$mode" "$C_RESET"
(cd "$frontend_dir" && NODE_ENV="$mode" npx webpack --mode "$mode")

ok=1
for f in index.html main.js main.css icon.svg main.js.map main.css.map; do
    if [ ! -f "$dist_dir/$f" ]; then
        printf '%s!!! missing expected artifact: bins/philharmonic-chat/dist/%s%s\n' \
            "$C_ERR" "$f" "$C_RESET" >&2
        ok=0
    fi
done

if [ "$ok" -eq 0 ]; then
    printf '%s!!! Build produced incomplete output.%s\n' "$C_ERR" "$C_RESET" >&2
    exit 1
fi

printf '%s=== Philharmonic Chat frontend build complete ===%s\n' "$C_OK" "$C_RESET"
printf '    Artifacts in bins/philharmonic-chat/dist/:\n'
ls -lh "$dist_dir/"
