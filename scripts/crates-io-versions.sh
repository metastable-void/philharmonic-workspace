#!/bin/sh
# scripts/crates-io-versions.sh — list the published (non-yanked)
# versions of a crate on crates.io, one per line, in the order the
# registry stores them (effectively oldest first).
#
# Usage:
#   ./scripts/crates-io-versions.sh <crate-name>
#
# Example:
#   ./scripts/crates-io-versions.sh mechanics-core
#   0.1.0
#   0.2.0
#   0.2.1
#   0.2.2
#
# Queries the crates.io **sparse index** (index.crates.io) directly
# rather than the JSON API (`crates.io/api/v1/...`). The sparse
# index is what cargo itself uses; it's faster, lighter, and
# doesn't require a custom User-Agent header. Each line of the
# response is one JSON object describing a single version.
#
# Complements `crate-version.sh`:
#   - `crate-version.sh <crate>`        — local version parsed from
#                                         <crate>/Cargo.toml.
#   - `crates-io-versions.sh <crate>`   — all published versions
#                                         currently on crates.io.
#
# Useful when preparing a release to sanity-check what's already
# published (e.g. "is 0.2.3 free?", "was 0.2.1 yanked?"). Yanked
# versions are filtered out — they're still in the index but
# unavailable for fresh resolution.
#
# Requires `jq`. Fetching is delegated to `./scripts/web-fetch.sh`,
# which picks whichever of curl/wget/fetch/ftp is available and
# fails on HTTP 4xx/5xx — exactly the behavior we need so that a
# missing crate (sparse-index 404) surfaces as a hard error rather
# than returning an empty body that jq would silently treat as
# "zero non-yanked versions."
#
# jq isn't part of the workspace's baseline toolchain (no other
# script in scripts/ depends on it). This script probes for jq up
# front and fails fast with a clear message if it's missing,
# rather than emitting an opaque "command not found". Install per
# your OS (e.g. `apt install jq`, `brew install jq`,
# `snap install jq`) before relying on this helper.
#
# Exit codes:
#   0         — crate found, zero or more non-yanked versions
#               printed. Empty output is possible (all yanked).
#   non-zero  — crate not found on crates.io (HTTP 404 from the
#               sparse-index fetch), network error, fetch tool /
#               jq missing, or malformed argument.
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.
# No bashisms. In particular, substring parameter expansions like
# `${var:0:2}` are a bash/ksh extension that `dash -n` parses
# without complaint but fails at runtime with "Bad substitution".
# `cut -c1-2` is the POSIX equivalent used below.

set -eu

if [ $# -ne 1 ] || [ -z "$1" ]; then
    echo "Usage: $0 <crate-name>" >&2
    exit 2
fi

# External-dep check for jq. (Web fetching is delegated to
# `./scripts/web-fetch.sh`, which handles its own tool probing.)
if ! command -v jq >/dev/null 2>&1; then
    printf '!!! crates-io-versions.sh: `jq` not found on PATH. Install it and retry.\n' >&2
    exit 1
fi

# crates.io normalizes crate names to lowercase for index lookup.
crate=$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')
n=${#crate}

# Sparse-index layout (https://doc.rust-lang.org/cargo/reference/registry-index.html):
#   length 1:  1/<crate>
#   length 2:  2/<crate>
#   length 3:  3/<first-char>/<crate>
#   length 4+: <first-2-chars>/<chars-3-4>/<crate>
#
# Bucketed this way so no directory ever holds more than a few
# thousand entries.
case "$n" in
    1)
        path="1/$crate"
        ;;
    2)
        path="2/$crate"
        ;;
    3)
        # One-char prefix. `cut -c1` = first character (POSIX).
        first=$(printf '%s' "$crate" | cut -c1)
        path="3/$first/$crate"
        ;;
    *)
        # Two two-char segments. `cut -c1-2` / `cut -c3-4` are the
        # POSIX-portable substitute for `${crate:0:2}`/`${crate:2:2}`.
        first2=$(printf '%s' "$crate" | cut -c1-2)
        next2=$(printf '%s' "$crate" | cut -c3-4)
        path="$first2/$next2/$crate"
        ;;
esac

# Capture the fetched body into a variable before piping, so that
# a fetch failure (e.g. HTTP 404 for a missing crate) aborts the
# script under `set -e`. A naive `web-fetch.sh ... | jq ...` would
# swallow the fetch exit status because POSIX sh has no
# `pipefail`; jq on empty input returns 0 silently, and we'd
# report "success with no versions" for a crate that doesn't
# exist.
body=$("$(dirname "$0")"/web-fetch.sh "https://index.crates.io/$path")

# `-r` emits raw strings (unquoted). `select(.yanked | not)` drops
# yanked versions; `.vers` prints the SemVer string for what's left.
printf '%s\n' "$body" | jq -r 'select(.yanked | not) | .vers'
