# Instructions/Notes for human developers

**For coding agents**: You can freely read the contents of
this file, but never edit this file.

**The following is notes for humans and not for coding agents**

---

## Reminders

- make sure we always make docs/roadmaps up-to-date.
- not blocking for this project: we should refactor all
  the best practices for AI coding brewed inside this
  workspace repo into a reusable template/Rust crate/etc.
  this can happen after the MVP work is done.

## Priority: Maintainability sweep

Sweep through the whole workspace (spawning subagents is
preferred) for maintainability issues, dirty/spaghetti codes,
and quality issues (e.g. memory leaks, deadlocks, races, etc.).

Refactor codes to make the code structured, small, de-duplicated.
Fixing the actual bugs mid-run is okay; don't change the bahavior
otherwise.

This is done by Yuka's direct Codex dispatch.

After it is fully completed, Claude Code continues to Tier-2
connectors (DNS + SMTP).

## HTTP/3 support notes

- Make sure HTTP/3 server side is done.

## SMTP connector

### Requirements

1. In any circumstances, refuse to connect to port 25.
2. Refuse to operate without username/password.
3. If port is given, use it.
4. If a port is given and the port is `587`, assume STARTTLS.
  If `465`, assume SMTPS (submission over TLS; RFC8314). 
  Otherwise, assume STARTTLS.
5. If no port is given, try 587/STARTTLS, then 465/SMTPS.
6. About TLS strictness (applicable to both modes):
  - `strict` (default) - full TLS enforcement, server verified.
  - `sloppy` - server verification skipped, encryption required.
    Document it as vulnerable to active MITM.
  - `opportunistic` - use TLS when available, server verified
    when on TLS.
  - `opportunistic_sloppy` - use TLS when available, server
    verification skipped.

### API shape

MIME body as a string, plus `MAIL FROM` (`mail_from` field)
value, and `RCPT TO` (`recipients` array field) values. It
should validate MIME validness and fix sloppy MIME envelopes
(like adding `MIME-Version: 1.0`, adding `Date:`, adding
`Message-Id`, `Content-Type`, formatting, etc.), without
introducing security holes, if it is really necessary for
mails to accepted by submission servers. If it is usually
handled correctly by SMTP servers, we can skip that.

## WebUI

Note: Keep WebUI up-to-date with any API features added
in the future.

## New connector: DNS (Tier 2)

**`philharmonic-connector-impl-dns`**:

Arbitrary DNS querying connector; supports any standard
RRs; `IN` only; uses system's resolver (consults
`/etc/resolv.conf`). Endpoint config can limit queries
to certain RRs only, or blocklist or allowlist (when both
exists, allowlisted non-blocklisted ones pass, applying
the both) zones.

## Keep the workflow authoring guide up-to-date

Re-read the docs/codex of everything related, and re-write
workflow authoring guides in en/jp to reflect the facts
on any surface changes.
