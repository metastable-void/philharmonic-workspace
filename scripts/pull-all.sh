#!/usr/bin/env bash
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

# Fetch latest commits for all submodules
git submodule update --remote --recursive

# Then update the parent's pointer
git status
echo
echo "Submodule pointers updated. Review and commit if appropriate:"
echo "  git add <submodule-dirs>"
echo "  git commit -m 'bump submodules'"
