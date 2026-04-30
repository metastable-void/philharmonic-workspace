#!/bin/sh
# scripts/show-dirty.sh — print the names of workspace member
# crates whose contents are dirty, one name per line.
#
# Covers both:
#   - Submodule-backed members (the majority) — dirtiness is
#     checked via `git diff` / `git ls-files` run *inside* the
#     submodule, because file-level changes don't appear in the
#     parent's working tree (the parent only sees the submodule-
#     pointer change, if any).
#   - In-tree (non-submodule) members (e.g. `xtask`) — dirtiness
#     is checked against the parent repo's working tree, scoped
#     to the member's path.
#
# Classifier: `-f <path>/.git` — submodules carry a `.git`
# pointer file at their root; in-tree directories have no `.git`.
#
# Machine-readable — `pre-landing.sh` consumes this to compute
# modified crates for the `--ignored` phase. Also usable standalone
# to inspect dirty members without the decoration of `status.sh`.
#
# Usage:
#   ./scripts/show-dirty.sh
#
# Output: one crate name per line (extracted from Cargo.toml,
# not the filesystem path — so `bins/philharmonic-api-server`
# emits `philharmonic-api-server`, compatible with `cargo -p`).
# Empty output if nothing is dirty. Always exits 0 (the absence
# of dirty members isn't a failure).
#
# Does not report the parent's own file-level dirtiness (docs,
# scripts, workspace-Cargo.toml edits) — use `status.sh` for that
# full picture. A parent-only commit lands via
# `commit-all.sh --parent-only`.
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu
. "$(dirname -- "$0")/lib/workspace-cd.sh"
. "$(dirname -- "$0")/lib/workspace-members.sh"

# Extract the crate name from a member's Cargo.toml.
# Falls back to the directory basename if parsing fails.
crate_name() {
    if [ -f "$1/Cargo.toml" ]; then
        _cn=$(sed -n 's/^name *= *"\([^"]*\)"/\1/p' "$1/Cargo.toml" | head -1)
        if [ -n "$_cn" ]; then
            echo "$_cn"
            return
        fi
    fi
    basename "$1"
}

# Buffer via `$()` so SIGPIPE from a truncating consumer
# (`./scripts/show-dirty.sh | head -3`) doesn't abort the walk
# mid-way.
output=$(
    for member in $workspace_members; do
        if [ -f "$member/.git" ]; then
            # Submodule-backed: check file-level dirtiness inside.
            if (
                cd -- "$member"
                ! git diff --quiet \
                    || ! git diff --cached --quiet \
                    || [ -n "$(git ls-files --others --exclude-standard)" ]
            ); then
                crate_name "$member"
            fi
        else
            # In-tree: parent repo sees file-level changes directly.
            if ! git diff --quiet -- "$member" \
                || ! git diff --cached --quiet -- "$member" \
                || [ -n "$(git ls-files --others --exclude-standard -- "$member")" ]; then
                crate_name "$member"
            fi
        fi
    done
)

if [ -n "$output" ]; then
    printf '%s\n' "$output"
fi
