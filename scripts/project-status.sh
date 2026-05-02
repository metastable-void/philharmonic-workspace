#!/bin/sh
# scripts/project-status.sh — generate an LLM-written summary of
# the workspace's development history and current status, save it
# to the committed archive, and print the output path.
#
# Assembles a single prompt payload from
#   - top-level README.md         (executive summary)
#   - top-level ROADMAP.md        (plans)
#   - `./scripts/git-log.sh -n N` (recent parent-repo history)
# and pipes it through the `openai-chat` xtask bin. The model's
# reply is written to
#   docs/project-status-reports/YYYY-MM-DD-hh-mm-ss.md
# and the resulting path is printed to stdout. The reply is
# **not** streamed to stdout: reports are committed for reuse, so
# re-reading a prior snapshot is a plain `cat` away and doesn't
# cost another OpenAI round-trip. See
# `docs/project-status-reports/README.md` for the archive's role
# and editorial policy.
#
# The OpenAI API key is read by `openai-chat` itself — either
# from `$OPENAI_API_KEY` or from `./.env` at the workspace root.
# See xtask/src/bin/openai-chat.rs.
#
# Usage:
#   ./scripts/project-status.sh [-n <log-lines>] [--model <model>]
#
# Options:
#   -n <N>          Lines of `git-log.sh` output to include
#                   (default 500, same as git-log.sh's default).
#   --model <M>     Override the model forwarded to OpenAI
#                   (default: whatever openai-chat's default is).
#   -h, --help      Show this message.
#
# POSIX sh — see CONTRIBUTING.md §6.

set -eu

. "$(dirname -- "$0")/lib/workspace-cd.sh"

log_lines=500
model=
reports_dir="docs/project-status-reports"

usage() {
    cat >&2 <<EOF
Usage: $0 [-n <log-lines>] [--model <model>]

  -n <N>          Lines of git-log.sh output to include (default 500).
  --model <M>     Override OpenAI model identifier.
  -h, --help      Show this message.

Output file: $reports_dir/YYYY-MM-DD-hh-mm-ss.md
EOF
}

is_positive_int() {
    case "$1" in
        ''|*[!0-9]*) return 1 ;;
        0) return 1 ;;
        *) return 0 ;;
    esac
}

while [ $# -gt 0 ]; do
    case "$1" in
        -n)
            [ $# -ge 2 ] || {
                printf '!!! project-status: -n needs a value\n' >&2
                exit 2
            }
            if ! is_positive_int "$2"; then
                printf '!!! project-status: -n expects a positive integer (got: %s)\n' "$2" >&2
                exit 2
            fi
            log_lines=$2
            shift 2
            ;;
        --model)
            [ $# -ge 2 ] || {
                printf '!!! project-status: --model needs a value\n' >&2
                exit 2
            }
            model=$2
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            printf '!!! project-status: unknown argument: %s\n' "$1" >&2
            usage
            exit 2
            ;;
    esac
done

[ -f ./README.md ]  || { printf '!!! project-status: README.md not found at workspace root\n' >&2;  exit 1; }
[ -f ./docs/ROADMAP.md ] || { printf '!!! project-status: ROADMAP.md not found at docs directory\n' >&2; exit 1; }
[ -d "$reports_dir" ] || {
    printf '!!! project-status: report directory %s does not exist\n' "$reports_dir" >&2
    exit 1
}

timestamp=$(TZ=Asia/Tokyo date +%Y-%m-%d-%H-%M-%S)
report_path="$reports_dir/$timestamp.md"

if [ -e "$report_path" ]; then
    printf '!!! project-status: %s already exists — refusing to overwrite\n' "$report_path" >&2
    exit 1
fi

prompt_tmp=$(./scripts/mktemp.sh project-status-in)
output_tmp=$(./scripts/mktemp.sh project-status-out)
trap 'rm -f "$prompt_tmp" "$output_tmp"' EXIT INT HUP TERM

{
    printf '=== README.md ===\n\n'
    cat ./README.md
    printf '\n\n=== ROADMAP.md ===\n\n'
    cat ./docs/ROADMAP.md
    printf '\n\n=== ./scripts/git-log.sh -n %s (parent workspace) ===\n\n' "$log_lines"
    ./scripts/git-log.sh -n "$log_lines"
} > "$prompt_tmp"

system_prompt='You are summarising the development history and current status of a Rust workspace project named Philharmonic, given its README.md (executive summary), ROADMAP.md (plans), and recent parent-repo git log output. The output will be committed as a Markdown file in the project'\''s `docs/project-status-reports/` archive. Produce a Markdown report using headings (`##` level) with these sections, in order: Overall status — one short paragraph; Recent work — bulleted list drawn from the git log, citing short commit SHAs where useful; What is next — short paragraph grounded in ROADMAP.md; Notable concerns or blockers — omit this section entirely if none are evident. No code fences around the whole document; be specific and concise; do not speculate beyond what the inputs support.'

if [ -n "$model" ]; then
    ./scripts/xtask.sh openai-chat -- \
        --system-prompt "$system_prompt" \
        --model "$model" \
        < "$prompt_tmp" > "$output_tmp"
else
    ./scripts/xtask.sh openai-chat -- \
        --system-prompt "$system_prompt" \
        < "$prompt_tmp" > "$output_tmp"
fi

# Promote to the committed archive only after the model call
# succeeded — otherwise a network/HTTP failure would leave a
# partial/empty file at the archive path.
mv "$output_tmp" "$report_path"

printf '%s\n' "$report_path"

echo 'Please update docs/SUMMARY.md'
