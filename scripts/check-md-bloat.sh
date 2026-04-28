#!/bin/sh
# Detect bloated Markdown files in the workspace. Prints line
# counts for every `.md` / `.MD` file reachable from the workspace
# root, excluding any `target/` build-output trees. Output ends
# with a `total` line (standard `wc -l` behaviour with multiple
# files).
#
# Pipe through `sort -n` to surface the biggest files:
#
#   ./scripts/check-md-bloat.sh | sort -n | tail -20
#
# Detector, not a rule — a file dramatically larger than its peers
# is worth inspecting against its intended contract (see
# CONTRIBUTING.md §18 for each doc home's role). Some docs
# legitimately need to be large (CONTRIBUTING.md, ROADMAP.md, the
# per-phase design docs); the script surfaces candidates so the
# human can decide.
#
# POSIX sh — see CONTRIBUTING.md §6.

set -eu

. "$(dirname -- "$0")/lib/workspace-cd.sh"

find . -type d -name target -prune -o -type f -name '*.[mM][dD]' -exec wc -l {} + | sort -nr
