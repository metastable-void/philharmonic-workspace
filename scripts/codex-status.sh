#!/usr/bin/env bash
# Find Codex processes spawned by Claude Code (via the codex-companion.mjs
# plugin shim), walk their descendant tree, and print a compact summary.
# Standalone `codex` invocations (e.g. the VSCode extension's app-server)
# are intentionally ignored.

set -uo pipefail

BOLD=$'\e[1m'
DIM=$'\e[2m'
RESET=$'\e[0m'

CODEX_PATTERN='codex-companion\.mjs'

mapfile -t ROOTS < <(pgrep -f "$CODEX_PATTERN" | sort -u)

if [ "${#ROOTS[@]}" -eq 0 ]; then
    echo "No Codex process running."
    exit 0
fi

collect_tree() {
    local pid=$1
    echo "$pid"
    local kids
    kids=$(pgrep -P "$pid" 2>/dev/null || true)
    for k in $kids; do
        collect_tree "$k"
    done
}

# Strip leading env-var assignments (VAR=value ...) that shells prepend to
# commands — they drown out the actual program name.
strip_env() {
    local cmd=$1
    while [[ "$cmd" =~ ^[A-Za-z_][A-Za-z0-9_]*=[^[:space:]]*[[:space:]]+(.*)$ ]]; do
        cmd="${BASH_REMATCH[1]}"
    done
    printf '%s' "$cmd"
}

fmt_tree() {
    local root=$1
    local pids
    pids=$(collect_tree "$root")
    {
        printf 'PID\t%%CPU\tRSS(MB)\tELAPSED\tCOMMAND\n'
        for p in $pids; do
            [ -d "/proc/$p" ] || continue
            local line pcpu rss etime cmd rss_mb
            line=$(ps -o pcpu=,rss=,etime=,command= -p "$p" 2>/dev/null | sed 's/^ *//') || continue
            [ -n "$line" ] || continue
            read -r pcpu rss etime cmd <<<"$line"
            rss_mb=$(( rss / 1024 ))
            cmd=$(strip_env "$cmd")
            if [ "${#cmd}" -gt 80 ]; then
                cmd="${cmd:0:77}..."
            fi
            printf '%s\t%s\t%s\t%s\t%s\n' \
                "$p" "$pcpu" "$rss_mb" "$etime" "$cmd"
        done
    } | column -t -s $'\t'
}

printf '%s=== Codex status ===%s\n' "$BOLD" "$RESET"
for root in "${ROOTS[@]}"; do
    etime_root=$(ps -o etime= -p "$root" 2>/dev/null | tr -d ' ')
    printf '\n%sRoot %s%s %s(elapsed %s)%s\n' \
        "$BOLD" "$root" "$RESET" "$DIM" "${etime_root:-?}" "$RESET"
    fmt_tree "$root" | sed 's/^/  /'
done
printf '\n'
