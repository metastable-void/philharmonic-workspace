#!/bin/sh
# scripts/slack-webhook.sh — post a workspace progress summary
# (Japanese) to the configured Slack webhook.
#
# Usage:
#   ./scripts/slack-webhook.sh
#
# Builds the summary by running `./scripts/stats.sh --japanese`
# (volume / activity numbers) followed by an LLM-generated
# single-paragraph progress note from README.md + ROADMAP.md +
# the last 20 commits (via `xtask openai-chat`, model `gpt-5.5`).
# If `OPENAI_API_KEY` is unset, the LLM paragraph is skipped and
# only the raw stats are sent.
#
# Required env vars:
#   SLACK_WEBHOOK_URL  Slack incoming-webhook target. Unset →
#                      prints the summary to stdout and exits 0
#                      (useful for dry runs).
#   OPENAI_API_KEY     OpenAI API key. Unset → omits the LLM
#                      paragraph; stats still post.
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu

. "$(dirname -- "$0")/lib/script-help.sh"
script_help_handle "$@"

. "$(dirname -- "$0")/lib/colors.sh"
. "$(dirname -- "$0")/lib/workspace-cd.sh"
. "$(dirname -- "$0")/lib/cargo-target-dir.sh"

tmp=$(./scripts/mktemp.sh summary)
trap 'rm -f "$tmp"' EXIT INT HUP TERM

mk_prompt () {
	echo "Repo stats: $( ./scripts/stats.sh )"
	echo
	echo "===README.md==="
	echo
	cat README.md
	echo
	echo "===docs/ROADMAP.md==="
	echo
	cat docs/ROADMAP.md
	echo
	echo "===Latest Git commits==="
	echo
	./scripts/log.sh --history -n 20
}

./scripts/stats.sh --japanese > "$tmp"
echo >> "$tmp"

if [ -z "${OPENAI_API_KEY:-}" ]; then
	echo "(OPENAI_API_KEY not set; skipping LLM summary)" >> "$tmp"
else
	mk_prompt | ./scripts/xtask.sh openai-chat -- --model gpt-5.5 --system-prompt 'Write a single-paragraph summary of the progress on the project repo in plaintext Japanese (no Markdown). Focus on recent changes, no "published on GitHub/crates.io" wording, no OSS/FLOSS/etc. wording, to avoid confusions by decision makers.' | grep -v '^[[:space:]]*$' >> "$tmp"
fi

if [ -z "${SLACK_WEBHOOK_URL:-}" ]; then
	echo "Slack unavailable. Printing contents below:"
	cat "$tmp"
	exit 0
fi

echo '{"text":'"$(./scripts/xtask.sh encode-json-str < "$tmp")"'}' | ./scripts/xtask.sh web-post -- "${SLACK_WEBHOOK_URL}"


