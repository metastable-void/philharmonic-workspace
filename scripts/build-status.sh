#!/bin/sh
# scripts/build-status.sh — show what cargo/rustc/lld is currently doing.
#
# Useful when a build appears stuck (no cargo output for minutes).
# Scans running processes for cargo, rustc, rust-lld, miri, clippy,
# rustfmt, rustdoc, build-script compilation, and running build scripts.
#
# Usage:
#   ./scripts/build-status.sh          # one-shot snapshot
#   watch -n 2 ./scripts/build-status.sh   # poll every 2s
#
# POSIX sh — see CONTRIBUTING.md §6.

set -eu

. "$(dirname -- "$0")/lib/script-help.sh"
script_help_handle "$@"


. "$(dirname -- "$0")/lib/colors.sh"

printf '%s=== build status ===%s\n' "$C_HEADER" "$C_RESET"

found=0

cargo_subcommand() {
    awk '
        {
            for (i = 3; i <= NF; i++) {
                if ($i ~ /(^|\/)cargo$/) {
                    j = i + 1
                    if ($j ~ /^\+/) {
                        j++
                    }
                    print $j
                    exit
                }
            }
        }
    '
}

cargo_target() {
    line=$1
    crate=$(printf '%s' "$line" | sed -n 's/.*-p \([^ ]*\).*/\1/p')
    if [ -n "$crate" ]; then
        printf '%s' "$crate"
    elif printf '%s' "$line" | grep -q -- '--workspace'; then
        printf '%s' 'workspace'
    else
        printf '%s' 'default target'
    fi
}

build_script_crate_from_hash_dir() {
    raw=$1
    printf '%s' "$raw" | sed 's/-[0-9a-f][0-9a-f]*$//'
}

# cargo driver: show the subcommand currently controlling the build.
ps -eo pid,etime,args 2>/dev/null | grep '[c]argo' | grep -v 'grep' | while IFS= read -r line; do
    subcmd=$(printf '%s' "$line" | cargo_subcommand)
    case "$subcmd" in
        build | check | clippy | doc | fmt | miri | test)
            ;;
        *)
            continue
            ;;
    esac

    found=1
    pid=$(printf '%s' "$line" | awk '{print $1}')
    etime=$(printf '%s' "$line" | awk '{print $2}')
    target=$(cargo_target "$line")
    mode=$subcmd
    if [ "$subcmd" = "miri" ]; then
        mode=$(printf '%s' "$line" | awk '
            {
                for (i = 3; i <= NF; i++) {
                    if ($i == "miri") {
                        print "miri " $(i + 1)
                        exit
                    }
                }
            }
        ')
        mode=${mode:-miri}
    fi
    if [ "$subcmd" = "test" ] && printf '%s' "$line" | grep -q -- '-- --ignored'; then
        mode='test --ignored'
    fi
    printf '%s  cargo%s %s %s%s%s (pid %s, elapsed %s)\n' \
        "$C_OK" "$C_RESET" "$mode" "$C_BOLD" "$target" "$C_RESET" "$pid" "$etime"
done

# rustc: extract the crate name from --crate-name arg
ps -eo pid,etime,args 2>/dev/null | grep '[r]ustc' | grep -v 'grep' | while IFS= read -r line; do
    found=1
    pid=$(printf '%s' "$line" | awk '{print $1}')
    etime=$(printf '%s' "$line" | awk '{print $2}')
    crate=$(printf '%s' "$line" | sed -n 's/.*--crate-name \([^ ]*\).*/\1/p')
    if [ "$crate" = "build_script_build" ] || [ "$crate" = "build_script_main" ]; then
        raw=$(printf '%s' "$line" | sed -n 's|.*--out-dir [^ ]*/build/\([^/]*\).*|\1|p')
        build_crate=$(build_script_crate_from_hash_dir "$raw")
        printf '%s  rustc%s compiling build script %s%s%s (pid %s, elapsed %s)\n' \
            "$C_WARN" "$C_RESET" "$C_BOLD" "${build_crate:-unknown}" "$C_RESET" "$pid" "$etime"
        continue
    fi
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

# build-script-build / build-script-main: a running build.rs executable (e.g. aws-lc-sys's
# C build can stall for minutes with no other Rust process active).
# Cargo invokes it from target/<dir>/build/<crate>-<hash>/build-script-*.
ps -eo pid,etime,args 2>/dev/null | grep -E '[b]uild-script-(build|main)' | grep -v 'grep' | while IFS= read -r line; do
    found=1
    pid=$(printf '%s' "$line" | awk '{print $1}')
    etime=$(printf '%s' "$line" | awk '{print $2}')
    raw=$(printf '%s' "$line" | sed -n 's|.*/build/\([^/]*\)/build-script-[^ ]*.*|\1|p')
    crate=$(build_script_crate_from_hash_dir "$raw")
    printf '%s  build-script%s running %s%s%s (pid %s, elapsed %s)\n' \
        "$C_WARN" "$C_RESET" "$C_BOLD" "${crate:-unknown}" "$C_RESET" "$pid" "$etime"
done

# If nothing found
if ! ps -eo args 2>/dev/null | grep -qE '[r]ustc|[r]ust-lld|[c]lippy-driver|[r]ustfmt|[r]ustdoc|[c]argo .*(build|check|clippy|doc|fmt|miri|test)|[b]uild-script-(build|main)' 2>/dev/null; then
    printf '%s  (no active Rust build processes)%s\n' "$C_DIM" "$C_RESET"
fi
