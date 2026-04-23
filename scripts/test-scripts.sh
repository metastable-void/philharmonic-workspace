#!/bin/sh
# scripts/test-scripts.sh — POSIX-compliance check for every shell
# script under ./scripts/. Runs `dash -n` (strict POSIX parser) on
# each .sh file, falling back to `sh -n` if dash isn't installed.
# Any parse error fails the script — catches bashisms and POSIX
# deviations before they land.
#
# Mandatory after any change under scripts/. See
# docs/design/13-conventions.md §Shell scripts.
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu
. "$(dirname -- "$0")/lib/colors.sh"
. "$(dirname -- "$0")/lib/workspace-cd.sh"

if command -v dash >/dev/null 2>&1; then
    checker=dash
elif command -v sh >/dev/null 2>&1; then
    printf 'note: dash not found; falling back to %s\n' \
        "$(command -v sh)" >&2
    checker=sh
else
    printf '%s!!! no POSIX shell found (need dash or sh)%s\n' "$C_ERR" "$C_RESET" >&2
    exit 1
fi

printf '%s=== parse-checking scripts/*.sh + scripts/lib/*.sh with %s ===%s\n' \
    "$C_HEADER" "$checker" "$C_RESET"

fail=0
# POSIX sh has no recursive glob; enumerate the subdirectories we
# care about explicitly. `scripts/lib/*.sh` covers sourced helpers
# like workspace-cd.sh; a parse error there breaks every script
# that sources them.
for f in scripts/*.sh scripts/lib/*.sh; do
    [ -f "$f" ] || continue  # glob expands literally if no match
    if "$checker" -n "$f" 2>/dev/null; then
        printf '%sok%s   %s\n' "$C_OK" "$C_RESET" "$f"
    else
        printf '%sFAIL%s %s\n' "$C_ERR" "$C_RESET" "$f" >&2
        # Re-run without stderr suppression so the user sees the error.
        "$checker" -n "$f" || true
        fail=1
    fi
done

if [ "$fail" -eq 0 ]; then
    printf '%s=== all scripts POSIX-clean (%s) ===%s\n' "$C_OK" "$checker" "$C_RESET"
else
    printf '%s=== FAILURES — fix before landing ===%s\n' "$C_ERR" "$C_RESET" >&2
    exit 1
fi
