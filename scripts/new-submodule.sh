#!/bin/sh
# scripts/new-submodule.sh — scaffold a new workspace submodule
# crate, correctly and repeatably; or adopt an existing remote
# crate as a submodule via `--adopt-existing`.
#
# Thin wrapper around the `new-submodule` xtask bin. See
# `xtask/src/bin/new-submodule.rs` for the authoritative
# documentation (arguments, preflight checks, exit codes, what
# it does, what it does NOT do).
#
# Scaffold a fresh placeholder submodule:
#
#   ./scripts/new-submodule.sh \
#       --name philharmonic-connector-impl-api \
#       --description "Trait-only API crate between the connector-service framework and per-implementation crates." \
#       --remote-url https://github.com/metastable-void/philharmonic-connector-impl-api.git \
#       --before philharmonic-connector-impl-http-forward
#
# Adopt an existing remote crate (skip scaffolding so the
# remote's `Cargo.toml`, `src/`, etc. survive intact):
#
#   ./scripts/new-submodule.sh \
#       --name inline-blob \
#       --remote-url https://github.com/metastable-void/inline-blob.git \
#       --adopt-existing
#
# Dry-run (print plan without changing anything):
#
#   ./scripts/new-submodule.sh --name ... --remote-url ... --dry-run
#
# The remote must already exist and have at least one commit.
# Create it via `gh repo create <name> --add-readme` or the
# GitHub web UI "Initialize this repository with a README".
#
# After a successful run, nothing has been committed yet —
# finalise with:
#
#   ./scripts/commit-all.sh "add <name> submodule — placeholder scaffolding"
#   ./scripts/push-all.sh
#
# (For adoption, the commit message will say "adopted existing
# crate" instead — the bin prints the recommended message.)
#
# POSIX sh only — see CONTRIBUTING.md §6.

set -eu
exec "$(dirname -- "$0")/xtask.sh" new-submodule -- "$@"
