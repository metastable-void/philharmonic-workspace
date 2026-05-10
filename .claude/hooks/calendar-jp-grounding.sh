#!/bin/sh
# .claude/hooks/calendar-jp-grounding.sh — emit a calendar-jp
# grounding-context message for Claude Code's PostToolUse hook
# `additionalContext` mechanism. Wired up as a single hook
# entry in `.claude/settings.json` that fires for every Bash
# tool call; this script then filters in-process to scope it to
# the three named state-change scripts CLAUDE.md flags as
# mandatory `calendar-jp` triggers (commit-all.sh, push-all.sh,
# publish-crate.sh).
#
# Why in-script filtering and not the hook schema's `if` field:
# empirically the `if` field is silently ignored by the
# Claude Code build this workspace targets — three `if`-scoped
# entries fired all-at-once for an unrelated `git check-ignore`
# in the session that introduced this hook. A single hook
# entry plus a case-match against the stdin JSON's
# `"command":"<prefix>"` substring is the workaround.
#
# Why no `jq`: forbidden in this workspace
# ([CONTRIBUTING.md §7](../../CONTRIBUTING.md#7-external-tool-wrappers)) —
# applies to `.claude/` infrastructure too. The substring
# case-match below intentionally treats the stdin JSON as an
# opaque text blob (no parsing) so we stay POSIX-shell-only.
# JSON encoding of the calendar-jp output uses the
# `encode-json-str` xtask bin instead.
#
# Output contract: a single-line JSON object on stdout matching
# Claude Code's PostToolUse hook output schema. The
# `additionalContext` field is injected back into the model's
# context, so the next reply sees the JST grid + holiday list +
# wall-clock timestamp without an explicit `calendar-jp`
# invocation.
#
# Failure modes are silent: if the command doesn't match, or
# `calendar-jp` errors out, we exit 0 with no JSON, and Claude
# Code treats the absence of output as "hook ran but had
# nothing to inject" rather than failing the underlying tool
# call. CLAUDE.md's prose grounding rule remains the
# authoritative obligation either way; this hook is a
# belt-and-braces reinforcement.
#
# POSIX sh — see CONTRIBUTING.md §6.

set -eu

# Substring-filter against the stdin JSON's `tool_input.command`
# field. Matches any invocation whose command starts with one
# of the three named state-change scripts; everything else
# exits 0 without emitting JSON.
json=$(cat)
case "$json" in
    *'"command":"./scripts/commit-all.sh'*) ;;
    *'"command":"./scripts/push-all.sh'*) ;;
    *'"command":"./scripts/publish-crate.sh'*) ;;
    *) exit 0 ;;
esac

out=$(./scripts/xtask.sh calendar-jp 2>&1) || exit 0

prefix='[auto-grounding] calendar-jp after state-changing git op (commit-all/push-all/publish-crate). Per CLAUDE.md the off-hours-timestamp case obligates the off-hours note on the next reply.'

ctx=$({
    printf '%s\n\n' "$prefix"
    printf '%s\n' "$out"
} | ./scripts/xtask.sh encode-json-str --)

printf '{"hookSpecificOutput":{"hookEventName":"PostToolUse","additionalContext":%s}}\n' "$ctx"
