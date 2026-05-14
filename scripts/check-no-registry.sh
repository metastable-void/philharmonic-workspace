#!/bin/sh
# scripts/check-no-registry.sh — refuse `registry = "..."` directives
# inside any workspace Cargo.toml.
#
# Why: the workspace publishes to `crates-io` only. Pinning a dep or
# package to an alternate registry inside a tracked Cargo.toml would
#   (a) make downstream crates.io consumers unable to resolve the
#       dep (they don't have the alt-registry configured),
#   (b) leak the Menhera-cooldown / pub-fresh proxy plumbing into
#       the published manifest, and
#   (c) silently disable the workspace's `[registry] default =
#       "menhera-cooldown"` cooldown-mirror posture for that
#       specific dep — Cargo.toml's per-dep `registry =` overrides
#       the global default.
#
# Registry-routing belongs in `.cargo/config.toml` (workspace-level)
# or `~/.cargo/config.toml` (developer-level), never in a tracked
# Cargo.toml.
#
# Usage:
#   ./scripts/check-no-registry.sh
#
# Exit codes:
#   0  — no `registry = ...` directive found in any Cargo.toml.
#   1  — at least one violation found; lines printed.
#
# Wired into `./scripts/pre-landing.sh` (and CI by extension) so any
# accidental introduction surfaces at landing-check time.
#
# POSIX sh only — see CONTRIBUTING.md §6.

set -eu

. "$(dirname -- "$0")/lib/colors.sh"
. "$(dirname -- "$0")/lib/workspace-cd.sh"

# Find every tracked Cargo.toml in the workspace. `git ls-files`
# excludes target/, vendored .crate tarballs, and anything else
# that isn't part of the workspace's source tree.
matches=$(git ls-files -- '*Cargo.toml' \
    | xargs grep -n -E 'registry[[:space:]]*=' 2>/dev/null \
    | grep -vE '^[^:]*:[0-9]+:[[:space:]]*#' \
    | grep -vE '\bregistry-(index|default|alternate|protocol|access|token)\b' \
    || true)

if [ -n "$matches" ]; then
    printf '%s!!! `registry = "..."` directive(s) found in tracked Cargo.toml files:%s\n' \
        "$C_ERR" "$C_RESET" >&2
    printf '%s\n' "$matches" >&2
    printf '\n' >&2
    printf '%sRegistry-routing belongs in .cargo/config.toml, not Cargo.toml.%s\n' \
        "$C_WARN" "$C_RESET" >&2
    printf '%sSee scripts/check-no-registry.sh header for the full rationale.%s\n' \
        "$C_WARN" "$C_RESET" >&2
    exit 1
fi

printf '%s=== check-no-registry: no `registry = ...` in tracked Cargo.toml files ===%s\n' \
    "$C_OK" "$C_RESET"
