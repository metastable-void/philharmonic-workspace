#!/bin/sh
# scripts/build-status.sh — show what cargo/rustc/lld is currently doing.
#
# Useful when a build appears stuck (no cargo output for minutes).
# Scans running processes for cargo, rustc, rust-lld, miri, clippy,
# rustfmt, rustdoc, and reports what each is compiling/testing.
#
# Usage:
#   ./scripts/build-status.sh          # one-shot snapshot
#   watch -n 2 ./scripts/build-status.sh   # poll every 2s
#
# POSIX sh — see CONTRIBUTING.md §6.

set -eu

. "$(dirname -- "$0")/lib/colors.sh"

printf '%s=== build status ===%s\n' "$C_HEADER" "$C_RESET"

found=0

# rustc: extract the crate name from --crate-name arg
ps -eo pid,etime,args 2>/dev/null | grep '[r]ustc' | grep -v 'grep' | while IFS= read -r line; do
    found=1
    pid=$(printf '%s' "$line" | awk '{print $1}')
    etime=$(printf '%s' "$line" | awk '{print $2}')
    crate=$(printf '%s' "$line" | sed -n 's/.*--crate-name \([^ ]*\).*/\1/p')
    if [ -n "$crate" ]; then
        printf '%s  rustc%s compiling %s%s%s (pid %s, elapsed %s)\n' \
            "$C_NOTE" "$C_RESET" "$C_BOLD" "$crate" "$C_RESET" "$pid" "$etime"
    fi
done

# rust-lld: linking phase
ps -eo pid,etime,args 2>/dev/null | grep '[r]ust-lld' | grep -v 'grep' | while IFS= read -r line; do
    found=1
    pid=$(printf '%s' "$line" | awk '{print $1}')
    etime=$(printf '%s' "$line" | awk '{print $2}')
    output=$(printf '%s' "$line" | sed -n 's/.*-o \([^ ]*\).*/\1/p' | xargs basename 2>/dev/null || true)
    printf '%s  rust-lld%s linking %s%s%s (pid %s, elapsed %s)\n' \
        "$C_WARN" "$C_RESET" "$C_BOLD" "${output:-unknown}" "$C_RESET" "$pid" "$etime"
done

# cargo test / miri
ps -eo pid,etime,args 2>/dev/null | grep -E '[c]argo.*(test|miri)' | grep -v 'grep' | while IFS= read -r line; do
    found=1
    pid=$(printf '%s' "$line" | awk '{print $1}')
    etime=$(printf '%s' "$line" | awk '{print $2}')
    crate=$(printf '%s' "$line" | sed -n 's/.*-p \([^ ]*\).*/\1/p')
    mode="testing"
    printf '%s' "$line" | grep -q 'miri' && mode="miri"
    printf '%s  cargo%s %s %s%s%s (pid %s, elapsed %s)\n' \
        "$C_OK" "$C_RESET" "$mode" "$C_BOLD" "${crate:-workspace}" "$C_RESET" "$pid" "$etime"
done

# clippy
ps -eo pid,etime,args 2>/dev/null | grep '[c]lippy-driver' | grep -v 'grep' | while IFS= read -r line; do
    found=1
    pid=$(printf '%s' "$line" | awk '{print $1}')
    etime=$(printf '%s' "$line" | awk '{print $2}')
    crate=$(printf '%s' "$line" | sed -n 's/.*--crate-name \([^ ]*\).*/\1/p')
    printf '%s  clippy%s checking %s%s%s (pid %s, elapsed %s)\n' \
        "$C_NOTE" "$C_RESET" "$C_BOLD" "${crate:-unknown}" "$C_RESET" "$pid" "$etime"
done

# rustfmt
ps -eo pid,etime,args 2>/dev/null | grep '[r]ustfmt' | grep -v 'grep' | while IFS= read -r line; do
    found=1
    pid=$(printf '%s' "$line" | awk '{print $1}')
    etime=$(printf '%s' "$line" | awk '{print $2}')
    printf '%s  rustfmt%s formatting (pid %s, elapsed %s)\n' \
        "$C_DIM" "$C_RESET" "$pid" "$etime"
done

# rustdoc
ps -eo pid,etime,args 2>/dev/null | grep '[r]ustdoc' | grep -v 'grep' | while IFS= read -r line; do
    found=1
    pid=$(printf '%s' "$line" | awk '{print $1}')
    etime=$(printf '%s' "$line" | awk '{print $2}')
    crate=$(printf '%s' "$line" | sed -n 's/.*--crate-name \([^ ]*\).*/\1/p')
    printf '%s  rustdoc%s documenting %s%s%s (pid %s, elapsed %s)\n' \
        "$C_DIM" "$C_RESET" "$C_BOLD" "${crate:-unknown}" "$C_RESET" "$pid" "$etime"
done

# If nothing found
if ! ps -eo args 2>/dev/null | grep -qE 'rustc|rust-lld|clippy-driver|rustfmt|rustdoc|cargo.*(test|miri)' 2>/dev/null; then
    printf '%s  (no active Rust build processes)%s\n' "$C_DIM" "$C_RESET"
fi
