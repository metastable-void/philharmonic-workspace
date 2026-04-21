#!/usr/bin/env bash
set -euo pipefail

# Repo names to scaffold. Must match GitHub repo names under metastable-void/.
REPOS=(
    "philharmonic-policy"
    "philharmonic-workflow"
    "philharmonic-connector-common"
    "philharmonic-connector-client"
    "philharmonic-connector-router"
    "philharmonic-connector-service"
    "philharmonic-api"
    "philharmonic-connector-impl-http-forward"
    "philharmonic-connector-impl-llm-openai-compat"
    "philharmonic-connector-impl-llm-anthropic"
    "philharmonic-connector-impl-llm-gemini"
    "philharmonic-connector-impl-sql-postgres"
    "philharmonic-connector-impl-sql-mysql"
    "philharmonic-connector-impl-email-smtp"
    "philharmonic-connector-impl-embed"
    "philharmonic-connector-impl-vector-search"
)

TMPDIR=$(mktemp -d)
trap "rm -rf $TMPDIR" EXIT

for repo in "${REPOS[@]}"; do
    echo "=== Scaffolding $repo ==="
    cd "$TMPDIR"

    if [ -d "$repo" ]; then
        echo "  (already scaffolded locally, skipping)"
        continue
    fi

    git clone "https://github.com/metastable-void/$repo.git" 2>&1 | tail -3

    cd "$repo"

    # Only scaffold if empty
    if [ -z "$(git ls-files)" ] && [ -z "$(git log --oneline 2>/dev/null)" ]; then
        cat > README.md <<EOF
# $repo

Part of the Philharmonic workspace: https://github.com/metastable-void/philharmonic-workspace

SPDX-License-Identifier: Apache-2.0 OR MPL-2.0
EOF
        cat > .gitignore <<EOF
target/
**/*.rs.bk
Cargo.lock
.DS_Store
._*
.codex
.vscode/settings.json
EOF
        cat > Cargo.toml <<EOF
[package]
name = "$repo"
version = "0.0.0"
edition = "2024"
rust-version = "1.88"
license = "Apache-2.0 OR MPL-2.0"
repository = "https://github.com/metastable-void/$repo"
description = "Placeholder. Part of the Philharmonic workspace."

[dependencies]
EOF
        mkdir -p src
        echo "// $repo: placeholder" > src/lib.rs

        git add .
        git commit -m "Initial commit: placeholder scaffold" || true
        git branch -M main
        git push -u origin main
        echo "  pushed initial commit"
    else
        echo "  (not empty, skipping)"
    fi

    cd ..
done

echo
echo "All done. Now add each as a submodule in the parent:"
echo
for repo in "${REPOS[@]}"; do
    echo "git submodule add https://github.com/metastable-void/$repo.git $repo"
done
