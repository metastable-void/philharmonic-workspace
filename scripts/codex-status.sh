#!/bin/sh
# Find Codex processes spawned by Claude Code (via the codex-companion.mjs
# plugin shim), walk their descendant tree, and print a compact summary.
# Standalone `codex` invocations (e.g. the VSCode extension's app-server)
# are intentionally ignored.
#
# POSIX sh + POSIX utilities — works on Linux (glibc + procps, busybox /
# Alpine), FreeBSD, macOS. A single `ps -A -o` snapshot drives
# everything; no pgrep, no /proc, no `column`.
#
# Field choices:
# - `time` (cumulative CPU time), not `pcpu`/`%CPU`. `pcpu` is not in
#   busybox ps (Alpine 1.37's supported list omits it); `time` is POSIX-
#   mandated and supported everywhere. Format differs slightly by
#   platform (HH:MM:SS on Linux/busybox, MM:SS.s on macOS/BSD) but
#   parses the same for our purposes.
# - `rss` (not strict POSIX — POSIX mandates only `vsz`) kept because
#   it's supported identically on Linux procps, FreeBSD, macOS, and
#   Alpine busybox, and matches what the user expects for memory.
#
# See docs/design/13-conventions.md §Shell scripts.

set -eu

BOLD=$(printf '\033[1m')
DIM=$(printf '\033[2m')
RESET=$(printf '\033[0m')

# Snapshot all processes once. Columns in order: pid ppid time rss etime
# args. `args` must be last because it can contain whitespace. No `-w`
# because busybox ps rejects it; on macOS/BSD ps may truncate args to
# terminal width, but we truncate to 80 chars downstream anyway.
SNAPSHOT=$(ps -A -o pid=,ppid=,time=,rss=,etime=,args=)

# Roots: processes whose args match the codex-companion shim. POSIX awk.
ROOTS=$(printf '%s\n' "$SNAPSHOT" | awk '
{
    args = ""
    for (i = 6; i <= NF; i++) args = args (i == 6 ? "" : " ") $i
    if (args ~ /codex-companion\.mjs/) print $1
}
' | sort -u)

if [ -z "$ROOTS" ]; then
    echo "No Codex process running."
    exit 0
fi

# All descendants (including self) of a pid, via snapshot walk.
collect_tree() {
    _ct_pid=$1
    echo "$_ct_pid"
    _ct_kids=$(printf '%s\n' "$SNAPSHOT" | awk -v p="$_ct_pid" '$2 == p { print $1 }')
    for _ct_k in $_ct_kids; do
        collect_tree "$_ct_k"
    done
}

# Strip one or more leading `VAR=value ` env assignments from a command
# line. POSIX BRE with [[:space:]] character class.
strip_env() {
    printf '%s' "$1" | sed 's/^\([A-Za-z_][A-Za-z0-9_]*=[^[:space:]]*[[:space:]]\{1,\}\)\{1,\}//'
}

# Look up a row in the snapshot. Sets ROW_TIME, ROW_RSS_KB, ROW_ETIME,
# ROW_CMD. Returns 1 if the pid is not in the snapshot.
row_for() {
    _rf_pid=$1
    _rf_row=$(printf '%s\n' "$SNAPSHOT" | awk -v p="$_rf_pid" '$1 == p { print; exit }')
    [ -n "$_rf_row" ] || return 1
    ROW_TIME=$(printf '%s' "$_rf_row" | awk '{ print $3 }')
    ROW_RSS_KB=$(printf '%s' "$_rf_row" | awk '{ print $4 }')
    ROW_ETIME=$(printf '%s' "$_rf_row" | awk '{ print $5 }')
    ROW_CMD=$(printf '%s' "$_rf_row" | awk '{ $1=$2=$3=$4=$5=""; sub(/^ +/, ""); print }')
    return 0
}

fmt_tree() {
    _ft_root=$1
    _ft_pids=$(collect_tree "$_ft_root")
    printf '%-8s %-10s %-8s %-12s %s\n' 'PID' 'CPU-TIME' 'RSS(MB)' 'ELAPSED' 'COMMAND'
    for _ft_p in $_ft_pids; do
        row_for "$_ft_p" || continue
        _ft_rss_mb=$((ROW_RSS_KB / 1024))
        _ft_cmd=$(strip_env "$ROW_CMD")
        if [ "${#_ft_cmd}" -gt 80 ]; then
            _ft_cmd=$(printf '%.77s...' "$_ft_cmd")
        fi
        printf '%-8s %-10s %-8s %-12s %s\n' \
            "$_ft_p" "$ROW_TIME" "$_ft_rss_mb" "$ROW_ETIME" "$_ft_cmd"
    done
}

printf '%s=== Codex status ===%s\n' "$BOLD" "$RESET"
for root in $ROOTS; do
    if row_for "$root"; then
        _etime_root=$ROW_ETIME
    else
        _etime_root='?'
    fi
    printf '\n%sRoot %s%s %s(elapsed %s)%s\n' \
        "$BOLD" "$root" "$RESET" "$DIM" "$_etime_root" "$RESET"
    fmt_tree "$root" | sed 's/^/  /'
done
printf '\n'
