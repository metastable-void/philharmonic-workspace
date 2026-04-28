#!/bin/sh
# scripts/stats.sh — print workspace code statistics summary.
#
# Usage:
#   ./scripts/stats.sh                # English (default)
#   ./scripts/stats.sh --japanese     # Japanese
#   ./scripts/stats.sh --help         # help
#
# Parses the Total line from `./scripts/tokei.sh` and formats
# a one-line summary. The `--japanese` output is designed for
# Slack webhook messages.
#
# POSIX sh only — see CONTRIBUTING.md §6.

set -eu

. "$(dirname -- "$0")/lib/colors.sh"
. "$(dirname -- "$0")/lib/workspace-cd.sh"
. "$(dirname -- "$0")/lib/cargo-target-dir.sh"

lang=en

usage() {
    cat <<'EOF'
Usage: stats.sh [--japanese] [--help]

Print workspace code statistics summary.

Options:
  --japanese, -j   Output in Japanese (for Slack webhook etc.)
  --help, -h       Show this help and exit.

Output fields:
  files       Total source files
  lines       Total lines (code + docs + blanks)
  code        Code lines
  docs        Documentation/comment lines
EOF
}

while [ $# -gt 0 ]; do
    case "$1" in
        -h|--help)     usage; exit 0 ;;
        -j|--japanese) lang=ja; shift ;;
        --)            shift; break ;;
        -*)            printf 'stats.sh: unknown flag: %s\n' "$1" >&2; exit 2 ;;
        *)             printf 'stats.sh: unexpected argument: %s\n' "$1" >&2; exit 2 ;;
    esac
done

total=$( ./scripts/tokei.sh | grep '^\s*Total' )

files=$( printf %s "$total" | awk '{print $2}' )
lines=$( printf %s "$total" | awk '{print $3}' )
codes=$( printf %s "$total" | awk '{print $4}' )
docs=$(  printf %s "$total" | awk '{print $5}' )

case "$lang" in
    ja)
        printf '%dファイル、合計%d行（コード%d行、ドキュメント%d行）\n' \
            "$files" "$lines" "$codes" "$docs"
        ;;
    *)
        printf '%d files, %d total lines (%d code lines, %d docs lines)\n' \
            "$files" "$lines" "$codes" "$docs"
        ;;
esac
