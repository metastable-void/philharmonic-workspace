#!/bin/sh
# scripts/verify-tag.sh — verify that a crate's release tag is locally
# present, cryptographically signed, and pushed to origin at the same
# commit.
#
# Usage:
#   ./scripts/verify-tag.sh <crate>           # tag = "v<version>" from the
#                                             # crate's own Cargo.toml
#   ./scripts/verify-tag.sh <crate> <tag>     # explicit tag (e.g. v0.3.4)
#
# What it checks (all three must hold for exit 0):
#   1. Tag exists locally in the submodule.
#   2. Tag is a signed annotated tag and the signature verifies
#      (`git tag -v`). Relies on the signer's GPG key being in the
#      local keyring, or the SSH allowed-signers file being
#      configured — a signature that's legitimate but unverifiable
#      here fails this check (which is the correct "I can't trust
#      this" outcome for the machine running the check).
#   3. Origin has the same tag pointing to the same commit SHA that
#      the local tag's annotated ref dereferences to.
#
# Output shape: one `ok` / `FAIL: <reason>` line per check plus a
# final verdict. Exit 0 only when every check passes.
#
# Complements `heads.sh` (which shows HEAD signatures across all
# submodules) — this one targets a specific tag on a specific crate.
# Run after `./scripts/publish-crate.sh` + `./scripts/push-all.sh` to
# confirm the release landed cleanly end-to-end.
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu
. "$(dirname -- "$0")/lib/colors.sh"
. "$(dirname -- "$0")/lib/workspace-cd.sh"

script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"

if [ $# -lt 1 ] || [ $# -gt 2 ]; then
    cat <<EOF >&2
Usage: $0 <crate> [<tag>]

With one arg, tag is derived as "v<version>" from the crate's
local Cargo.toml via ./scripts/crate-version.sh.

Examples:
  $0 philharmonic-types
  $0 philharmonic-policy v0.1.0
EOF
    exit 2
fi

crate="$1"
crate_dir="$crate"

if [ ! -d "$crate_dir" ]; then
    printf '%s!!! crate directory not found: %s%s\n' "$C_ERR" "$crate_dir" "$C_RESET" >&2
    exit 2
fi

if [ ! -d "$crate_dir/.git" ] && [ ! -f "$crate_dir/.git" ]; then
    printf '%s!!! %s is not a git submodule (no .git entry)%s\n' "$C_ERR" "$crate_dir" "$C_RESET" >&2
    exit 2
fi

if [ $# -ge 2 ]; then
    tag="$2"
else
    version="$("$script_dir/crate-version.sh" "$crate")"
    tag="v${version}"
fi

printf '%s=== verifying tag %s in %s ===%s\n' "$C_HEADER" "$tag" "$crate" "$C_RESET"

# (1) Local tag exists.
if ! git -C "$crate_dir" rev-parse --verify --quiet "refs/tags/${tag}" >/dev/null; then
    printf '  local tag:  %sMISSING%s\n' "$C_ERR" "$C_RESET" >&2
    printf '    fix: ./scripts/publish-crate.sh %s (if you meant to release it) or locate the missing tag\n' "$crate" >&2
    exit 1
fi

# Dereference the annotated tag to its target commit.
local_sha="$(git -C "$crate_dir" rev-parse "${tag}^{commit}")"
printf '  local tag:  %sok%s (%s)\n' "$C_OK" "$C_RESET" "$local_sha"

# (2) Signature verifies. `git tag -v` is chatty — swallow stdout/stderr.
if git -C "$crate_dir" tag -v "$tag" >/dev/null 2>&1; then
    printf '  signed:     %sok%s\n' "$C_OK" "$C_RESET"
else
    printf '  signed:     %sFAIL%s — signature invalid, missing, or verifier key not in local keyring\n' "$C_ERR" "$C_RESET" >&2
    printf '    fix: ensure GPG key or SSH allowed-signers entry for the tag signer is available locally\n' >&2
    exit 1
fi

# (3) Origin has the tag at the same commit.
# Annotated tags show up twice in `git ls-remote --tags origin`:
# `refs/tags/TAG` (the tag object) and `refs/tags/TAG^{}` (the
# dereferenced commit). Passing a *pattern* filter to ls-remote
# silently hides the `^{}` peel line, so we list all tag refs and
# filter with awk — costs one full ref listing per invocation, but
# reliable regardless of tag shape. The `^{}` line is preferred; if
# it's missing the tag is lightweight (our workflow uses annotated
# + signed, so that'd be unexpected), and we fall back to the bare
# ref as a last resort.
all_remote_tags="$(git -C "$crate_dir" ls-remote --tags origin 2>/dev/null || :)"
origin_sha="$(
    printf '%s\n' "$all_remote_tags" \
        | awk -v t="refs/tags/${tag}^{}" '$2 == t {print $1}'
)"
if [ -z "$origin_sha" ]; then
    origin_sha="$(
        printf '%s\n' "$all_remote_tags" \
            | awk -v t="refs/tags/${tag}" '$2 == t {print $1}'
    )"
    if [ -z "$origin_sha" ]; then
        printf '  origin:     %sMISSING%s — tag not pushed\n' "$C_ERR" "$C_RESET" >&2
        printf '    fix: ./scripts/push-all.sh (uses --follow-tags)\n' >&2
        exit 1
    fi
fi

if [ "$origin_sha" != "$local_sha" ]; then
    printf '  origin:     %sMISMATCH%s — local %s vs origin %s\n' "$C_ERR" "$C_RESET" "$local_sha" "$origin_sha" >&2
    exit 1
fi

printf '  origin:     %sok%s (%s)\n' "$C_OK" "$C_RESET" "$origin_sha"
printf '%s=== verified: %s %s signed and pushed ===%s\n' "$C_OK" "$crate" "$tag" "$C_RESET"
