# scripts/lib/workspace-cd.sh — sourced helper that resolves the
# philharmonic-workspace root and `cd`s to it. Sets `workspace_root`
# in the caller's shell.
#
# Expected usage (near the top of scripts/<name>.sh, after `set -eu`):
#
#     . "$(dirname -- "$0")/lib/workspace-cd.sh"
#
# Priority (first one that produces a workspace-shaped dir wins):
#
#   1. Inside a submodule: the superproject working tree. Handles
#      `cd mechanics-core && ../scripts/foo.sh`.
#   2. Inside the workspace itself (not a submodule): the current
#      git toplevel. Handles `./scripts/foo.sh` from the root.
#   3. Outside any git repo, OR inside an unrelated git repo:
#      derive from $0's path. `scripts/` lives at the workspace
#      root by convention, so $(dirname $0)/.. IS the root.
#      Handles `/absolute/path/to/philharmonic/scripts/foo.sh`
#      run from `/tmp` or from a different project entirely.
#
# The git-based routes (1 and 2) are sanity-checked against the
# presence of `scripts/test-scripts.sh` — if that marker is
# missing, we're in a different repo and fall through to route 3.
#
# Sourced — not executed directly. No shebang on purpose.
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

# `|| true` on both git invocations so `set -e` in the sourcing
# script doesn't abort when we're outside any git repo (git
# rev-parse returns 128). We detect that case via the empty
# result and fall through to the $0-based resolution below.
_wcd_root=$(git rev-parse --show-superproject-working-tree 2>/dev/null || true)
if [ -z "$_wcd_root" ]; then
    _wcd_root=$(git rev-parse --show-toplevel 2>/dev/null || true)
fi

if [ -z "$_wcd_root" ] || [ ! -f "$_wcd_root/scripts/test-scripts.sh" ]; then
    _wcd_dir=$(cd -- "$(dirname -- "$0")" && pwd)
    _wcd_root=$(cd -- "$_wcd_dir/.." && pwd)
    unset _wcd_dir
fi

workspace_root=$_wcd_root
unset _wcd_root

cd "$workspace_root"
