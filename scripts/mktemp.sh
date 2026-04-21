#!/bin/sh
# scripts/mktemp.sh — create a temporary file and print its path.
#
# Workspace-canonical replacement for raw `mktemp(1)`. New code
# MUST call this wrapper; `mktemp` isn't universally present
# (older busybox builds, stripped containers), and the wrapper
# degrades gracefully where it isn't. See CLAUDE.md §Prefer
# scripts and docs/design/13-conventions.md §External tool
# wrappers for the rule.
#
# Usage:
#   tmp=$(./scripts/mktemp.sh [<slug>])
#   trap 'rm -f "$tmp"' EXIT INT HUP TERM
#
# The slug (default `tmp`) appears in the filename so files are
# distinguishable on a shared /tmp. The file is placed under
# $TMPDIR (fallback /tmp).
#
# Backends:
#   1. `mktemp` when on PATH — delegates with template
#      `${dir}/${slug}.XXXXXX`; mktemp's collision-avoidance and
#      default 0600 permissions apply.
#   2. Fallback: 10-char [A-Za-z0-9] suffix from /dev/urandom,
#      `touch`ed into existence and immediately `chmod 600`'d so
#      the fallback matches mktemp's own default-0600 contract.
#      Callers don't have to `chmod` after creation.
#
# **Cleanup is the caller's responsibility.** This script prints
# the path and exits; pair it with a `trap` in the caller so the
# temp file is removed on any exit path (normal, error, signal).
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu

dir=${TMPDIR:-/tmp}
slug=${1:-tmp}

if command -v mktemp >/dev/null 2>&1 ; then
    exec mktemp "${dir}/${slug}.XXXXXX"
fi

rand=$( LC_ALL=C tr -dc 'A-Za-z0-9' < /dev/urandom | dd bs=10 count=1 2>/dev/null )

temp="${dir}/${slug}.${rand}"
touch "$temp"
# Match mktemp(1)'s default 0600 — umask alone isn't a guarantee.
chmod 600 "$temp"
echo "$temp"
