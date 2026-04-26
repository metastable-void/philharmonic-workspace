#!/bin/sh
# Fetch the parent and update every submodule to the tip of its
# tracked remote branch. Does NOT commit the bumped submodule
# pointers — run scripts/commit-all.sh when you're ready, then
# scripts/push-all.sh.
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu

. "$(dirname -- "$0")/lib/workspace-cd.sh"

# Integrate remote changes into the parent. Rebase keeps history
# linear — local parent commits (usually submodule-pointer bumps)
# get replayed on top of origin. Fails loudly on conflicts or a
# dirty working tree rather than auto-merging.
#
# History-modification exception: the workspace's append-only
# rule (docs/design/13-conventions.md §Git workflow, "No history
# modification") forbids rebase in general. This `pull --rebase`
# is exception #2 — script-enforced, only touches local
# not-yet-pushed commits, preserves the commit message (Audit-Info
# + Signed-off-by trailers) verbatim, and re-signs under
# commit.gpgsign=true. Alternatives (--ff-only, default merge,
# default submodule checkout) each violate other invariants.
# See the conventions doc for the full rationale.
git pull --rebase

# Fetch all tags on the parent. Default fetch only pulls tags that
# point at commits we're receiving; release tags for inactive
# releases wouldn't follow otherwise.
git fetch --tags --quiet

# Update each submodule's working tree to the tip of its tracked
# remote branch (branch = ... in .gitmodules). --rebase replays
# any local submodule commits on top of the remote branch instead
# of detaching HEAD at the remote SHA, which the default checkout
# mode would do when origin is ahead.
#
# Same history-modification exception as the parent pull above:
# covered by docs/design/13-conventions.md §Git workflow (exception
# #2). Same reasoning — only local not-yet-pushed submodule
# commits are ever replayed; avoiding rebase here would detach
# HEAD and break commit-all.sh's detached-HEAD guard.
git submodule update --remote --rebase --recursive

# Submodule `update --remote` fetches branches but not arbitrary
# tags. Pull tags explicitly so release tags (`vX.Y.Z` produced
# by `publish-crate.sh`) are available locally for `git log
# --tags`, `git describe`, GitHub release-notes drafting, and any
# tag-based tooling that may be added later. Note:
# `./scripts/check-api-breakage.sh` uses crates.io as its
# baseline (not a git tag), so it doesn't require these tags to
# be present — this fetch is defensive, not load-bearing for the
# semver-checks flow.
git submodule foreach --quiet 'git fetch --tags --quiet origin'

# `git submodule update --remote --rebase` silently degrades to a
# plain checkout when HEAD was already detached (e.g. after a fresh
# `setup.sh` that hit an off-branch case, or after a previous
# version of these scripts that didn't attach). Re-attach now so
# the next contributor edit doesn't trip `commit-all.sh`'s
# detached-HEAD guard. Helper is idempotent and only attaches when
# safe (no unique commits dropped); see lib/attach-submodule-branch.sh.
REPO_ROOT=$(pwd -P)
export REPO_ROOT
git submodule foreach --recursive '
set -eu
if [ -z "$REPO_ROOT" ] || [ ! -d "$REPO_ROOT" ] ; then
    echo "Could not find REPO_ROOT, aborting." >&2
    exit 211
fi
. "${REPO_ROOT}/scripts/lib/attach-submodule-branch.sh"
attach_submodule_branch
'

# Show resulting state. Parent may now be dirty due to bumped
# submodule pointers; that's the signal to commit. We're already
# at the workspace toplevel, so a relative path is unambiguous.
./scripts/status.sh

cat <<'EOF'
Submodule pointers may have moved. When ready, commit and push
through the helper scripts (never direct git):

  scripts/commit-all.sh 'bump submodules'
  scripts/push-all.sh
EOF
