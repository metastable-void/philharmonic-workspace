#!/bin/sh
# scripts/update-stats-graph.sh — regenerate `docs/stats.svg` from
# the workspace's commit-stats history.
#
# Pipeline:
#   ./scripts/stats-log.sh --no-color
#       | ./scripts/xtask.sh stats-graph
#       > ./docs/stats.svg
#
# `stats-log.sh --no-color` emits one ANSI-free line per commit
# carrying the `Code-stats:` trailer; `stats-graph` (xtask bin)
# parses those lines and writes an SVG line chart of total / code
# / docs lines over time using the `poloto` crate. Commits without
# a parseable trailer are excluded (never interpolated). The SVG
# is embedded as an image in `docs/README.md`, so the mdBook
# output carries an up-to-date growth chart.
#
# **Run automatically by `commit-all.sh`** before each parent
# commit so HEAD always carries a fresh `docs/stats.svg`. Manual
# invocation is fine too (e.g. inspecting the chart between
# commits) but isn't required for normal workflow — every parent
# commit refreshes it.
#
# The SVG always lags by one commit: the new commit's
# `Code-stats:` trailer doesn't exist until the commit lands, so
# the chart up to commit N is regenerated just before commit N+1.
# That gap closes one commit at a time and isn't worth a
# post-commit re-write loop.
#
# Usage:
#   ./scripts/update-stats-graph.sh
#
# POSIX sh only — see CONTRIBUTING.md §6.

set -eu
. "$(dirname -- "$0")/lib/colors.sh"
. "$(dirname -- "$0")/lib/workspace-cd.sh"

out_path=docs/stats.svg

if [ ! -d docs ]; then
    printf '%s!!! update-stats-graph.sh: docs/ directory missing.%s\n' \
        "$C_ERR" "$C_RESET" >&2
    exit 1
fi

# Pipefail isn't POSIX, so check both stages explicitly. Render to
# a temp file first; only move into place on success so a failed
# render doesn't truncate the committed SVG.
tmp=$("$(dirname "$0")"/mktemp.sh stats-graph)
trap 'rm -f "$tmp"' EXIT INT HUP TERM

if ! ./scripts/stats-log.sh --no-color > "$tmp.log"; then
    echo "update-stats-graph.sh: stats-log.sh failed" >&2
    exit 1
fi

if ! ./scripts/xtask.sh stats-graph < "$tmp.log" > "$tmp.svg"; then
    echo "update-stats-graph.sh: stats-graph (xtask) failed" >&2
    exit 1
fi

mv -- "$tmp.svg" "$out_path"
rm -f -- "$tmp.log"

bytes=$(wc -c < "$out_path" | tr -d ' ')
printf '%s=== wrote %s (%s bytes) ===%s\n' "$C_OK" "$out_path" "$bytes" "$C_RESET"
