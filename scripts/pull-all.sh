#!/bin/sh
# Fetch the parent and update every submodule to the tip of its
# tracked remote branch. Does NOT commit the bumped submodule
# pointers — run scripts/commit-all.sh when you're ready, then
# scripts/push-all.sh.
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu

cd "$(git rev-parse --show-toplevel)"

# Integrate remote changes into the parent. Rebase keeps history
# linear — local parent commits (usually submodule-pointer bumps)
# get replayed on top of origin. Fails loudly on conflicts or a
# dirty working tree rather than auto-merging.
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
git submodule update --remote --rebase --recursive

# Submodule `update --remote` fetches branches but not arbitrary
# tags. Pull tags explicitly so `cargo-semver-checks --baseline-rev
# vX.Y.Z` and similar tag-based tooling see every release.
git submodule foreach --quiet 'git fetch --tags --quiet origin'

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
