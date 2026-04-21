#!/bin/sh
# scripts/web-fetch.sh — HTTP(S) GET a URL, write the body to
# stdout or to the given output file.
#
# Workspace-canonical replacement for raw `curl`/`wget`. New code
# MUST call this wrapper; the wrapper picks whichever of
# curl / wget / fetch / ftp is available, which keeps our scripts
# portable across Alpine (busybox), Debian/Ubuntu, FreeBSD,
# OpenBSD, macOS, WSL. See CLAUDE.md §Prefer scripts and
# docs/design/13-conventions.md §External tool wrappers for the
# rule.
#
# Usage:
#   ./scripts/web-fetch.sh <URL> [<outfile>]
#   body=$(./scripts/web-fetch.sh <URL>)
#
# Without <outfile>, the body is written to stdout. With
# <outfile>, the body is written to that path (atomically-ish,
# via the underlying tool's redirection semantics).
#
# Backend search order (first found wins):
#   1. curl   — GNU/Apache curl. The common case.
#   2. wget   — GNU wget, wget2, and busybox wget.
#   3. fetch  — FreeBSD's fetch(1).
#   4. ftp    — OpenBSD's ftp(1) in HTTP mode.
# Exits 1 with "No Web fetch available!" on stderr if none are
# present.
#
# User-Agent: override via `WEB_FETCH_UA` env. Default is
# `philharmonic-dev-agent/1.0`. OpenBSD's ftp(1) is the one
# backend that can't set a UA — it falls through to its own
# default.
#
# **HTTP errors fail the fetch.** All four backends are invoked
# so that a 4xx/5xx response causes a non-zero exit and an empty
# output (or nothing written on stdout). curl gets `-f`; wget,
# FreeBSD fetch, and OpenBSD ftp already default to failing on
# server errors. Callers that want to proceed regardless of HTTP
# status should use `./scripts/web-fetch.sh ... || :` at the
# call site, which is the idiom in `print-audit-info.sh` (an
# empty `4=`/`6=` field in the audit line is acceptable; a
# commit that fails because 1.1.1.1 was briefly unreachable is
# not).
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu

if [ $# -lt 1 ] ; then
    echo "Usage: $0 URL [OUTFILE]"
    exit
fi

UA=${WEB_FETCH_UA:-philharmonic-dev-agent/1.0}
URL=${1}
OUT=${2:-}

if [ -z "$URL" ] ; then
    echo "URL cannot be empty"
    exit 1
fi

if command -v curl > /dev/null 2>&1 ; then
    if [ -n "$OUT" ] ; then
        exec curl -fs -A "$UA" "$URL" > "$OUT" 2>/dev/null
    fi
    exec curl -fs -A "$UA" "$URL" 2>/dev/null
elif command -v wget > /dev/null 2>&1 ; then
    # baseline: GNU Wget, wget2, and Busybox's wget supported.
    exec wget -q -U "$UA" -O "${OUT:--}" "$URL" 2>/dev/null
elif command -v fetch > /dev/null 2>&1 ; then
    # FreeBSD fetch(1)
    exec fetch -qo "${OUT:--}" --user-agent="$UA" "$URL" 2>/dev/null
elif command -v ftp >/dev/null 2>&1 && ftp -h 2>&1 | grep -qi http ; then
    # OpenBSD ftp(1): no UA flag. Falls through to its default.
    exec ftp -Vo "${OUT:--}" "$URL"
fi

echo "No Web fetch available!" >&2
exit 1
