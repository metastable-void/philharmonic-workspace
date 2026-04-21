#!/usr/bin/env bash
# Fetch the parent and update every submodule to the tip of its
# tracked remote branch. Does NOT commit the bumped submodule
# pointers — run scripts/commit-all.sh when you're ready, then
# scripts/push-all.sh.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

# Integrate remote changes into the parent. Rebase keeps history
# linear — local parent commits (usually submodule-pointer bumps)
# get replayed on top of origin. Fails loudly on conflicts or a
# dirty working tree rather than auto-merging.
git pull --rebase

# Update each submodule's working tree to the tip of its tracked
# remote branch. (.gitmodules can specify per-submodule branch;
# otherwise git uses each submodule's origin HEAD.)
git submodule update --remote --recursive

# Show resulting state. Parent may now be dirty due to bumped
# submodule pointers; that's the signal to commit.
bash "$(dirname "$0")/status.sh"

cat <<'EOF'
Submodule pointers may have moved. When ready, commit and push
through the helper scripts (never direct git):

  scripts/commit-all.sh 'bump submodules'
  scripts/push-all.sh
EOF
