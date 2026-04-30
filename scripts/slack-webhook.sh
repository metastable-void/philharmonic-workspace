#!/bin/sh

set -eu

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
	echo "===ROADMAP.md==="
	echo
	cat ROADMAP.md
	echo
	echo "===Latest Git commits==="
	echo
	./scripts/git-log.sh -n 50
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


