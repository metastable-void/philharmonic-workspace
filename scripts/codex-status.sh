#!/usr/bin/env bash
# Find a running Codex process, walk its descendant tree, and print
# a pretty summary. Useful while a Codex task is in flight: you can
# see whether it's still doing real work, how long it's been running,
# and what its child processes are up to (cargo check, clippy, etc.).

set -uo pipefail

BOLD=$'\e[1m'
DIM=$'\e[2m'
CYAN=$'\e[36m'
RESET=$'\e[0m'

# Anchors we consider "a Codex root":
# - `codex-companion.mjs` — the plugin shim Claude Code spawns.
# - `codex` binary invoked directly from CLI.
CODEX_PATTERN='codex-companion\.mjs|(^|/)codex( |$)'

mapfile -t ROOTS < <(pgrep -f "$CODEX_PATTERN" | sort -u)

if [ "${#ROOTS[@]}" -eq 0 ]; then
    echo "No Codex process running."
    exit 0
fi

# Collect pid plus every descendant, in a rough pre-order.
collect_tree() {
    local pid=$1
    echo "$pid"
    local kids
    kids=$(pgrep -P "$pid" 2>/dev/null || true)
    for k in $kids; do
        collect_tree "$k"
    done
}

fmt_tree() {
    local root=$1
    local pids
    pids=$(collect_tree "$root")
    {
        printf 'PID\tPPID\tSTAT\t%%CPU\tRSS(MB)\tELAPSED\tCOMMAND\n'
        for p in $pids; do
            [ -d "/proc/$p" ] || continue
            local line ppid stat pcpu rss etime cmd rss_mb
            line=$(ps -o ppid=,stat=,pcpu=,rss=,etime=,command= -p "$p" 2>/dev/null | sed 's/^ *//') || continue
            [ -n "$line" ] || continue
            read -r ppid stat pcpu rss etime cmd <<<"$line"
            rss_mb=$(( rss / 1024 ))
            # Keep the command column readable but not grotesquely long.
            if [ "${#cmd}" -gt 90 ]; then
                cmd="${cmd:0:87}..."
            fi
            printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
                "$p" "$ppid" "$stat" "$pcpu" "$rss_mb" "$etime" "$cmd"
        done
    } | column -t -s $'\t'
}

total_procs=0
printf '%s=== Codex status ===%s\n' "$BOLD" "$RESET"
for root in "${ROOTS[@]}"; do
    pids=$(collect_tree "$root")
    count=$(echo "$pids" | wc -l)
    total_procs=$((total_procs + count))
    etime_root=$(ps -o etime= -p "$root" 2>/dev/null | tr -d ' ')
    cmd_root=$(ps -o command= -p "$root" 2>/dev/null | sed 's/^ *//')
    printf '\n%sRoot PID %s%s  %s(elapsed: %s)%s\n' \
        "$BOLD" "$root" "$RESET" "$DIM" "${etime_root:-?}" "$RESET"
    printf '  %s%s%s\n\n' "$CYAN" "$cmd_root" "$RESET"
    fmt_tree "$root" | sed 's/^/  /'
done

printf '\n%sTotal: %d process(es) across %d root(s).%s\n' \
    "$BOLD" "$total_procs" "${#ROOTS[@]}" "$RESET"
