#!/bin/sh
# scripts/heads.sh — show the current HEAD commit for the parent
# and every submodule, with short SHA, signature indicator, and
# subject. Canonical for verifying every committed/pushed change
# carries a cryptographic signature.
#
# The signature indicator comes from `git log --format=%G?`:
#   G — good signature (any trusted key)
#   U — good signature, untrusted key
#   X — good signature with expired key
#   Y — good signature with expired key (alternate)
#   R — good signature with revoked key
#   E — can't be verified (missing key, etc.)
#   B — bad signature
#   N — no signature at all
#
# Anything other than `G` (or `U` if you trust untrusted keys) on a
# pushed commit is a problem — `commit-all.sh` won't produce an `N`
# commit on its own, so if you see one it's either a commit that
# bypassed the script or a local verification issue to investigate.
#
# Prefer this over raw `git log -n 1` for HEAD-state queries; see
# docs/design/13-conventions.md §Git workflow.
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu

. "$(dirname -- "$0")/lib/colors.sh"
. "$(dirname -- "$0")/lib/workspace-cd.sh"

# Width of the longest submodule name
# (philharmonic-connector-impl-llm-openai-compat = 45), plus a
# little padding. Keeps columns aligned without needing `column`.
fmt='%-48s %s\n'

# Buffer all rows via `$()` and emit in one go at the end. If the
# script's stdout is piped to a truncating consumer (e.g.
# `./scripts/heads.sh | head -3`), SIGPIPE on closure affects only
# the final `printf` below — not the submodule walk. Otherwise
# `git submodule foreach` would abort mid-walk with a confusing
# "fatal: run_command returned non-zero status for <name>" as soon
# as the first row hits a closed pipe.
output=$(
    printf "$fmt" 'parent' "$(git log -n 1 --format='%h %G? %s' HEAD)"
    git submodule foreach --quiet '
    printf "%-48s %s\n" "$name" "$(git log -n 1 --format="%h %G? %s" HEAD)"
    '
)

# Colorize the signature character (field 3 after short SHA):
# G (good) green, U (untrusted-but-valid) and ? (unknown) yellow,
# everything else (N/E/B/X/Y/R) red. Awk rebuilds the line via
# the explicit fmt we already used above so multi-space padding
# from `%-48s` is preserved. No-op when colors are off (C_* empty).
printf '%s\n' "$output" | awk \
    -v ok="$C_OK" -v warn="$C_WARN" -v err="$C_ERR" -v reset="$C_RESET" '
{
    if (NF < 4) { print; next }
    sig = $3
    if (sig == "G")                c = ok
    else if (sig == "U" || sig == "?") c = warn
    else                             c = err
    # Subject = fields 4..NF rejoined with single spaces.
    subj = $4
    for (i = 5; i <= NF; i++) subj = subj " " $i
    printf "%-48s %s %s%s%s %s\n", $1, $2, c, sig, reset, subj
}'
