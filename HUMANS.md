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

## HTTP/3 support notes

- HTTP/3 is enabled for a server whenever it is configured
  with the HTTP/3 UDP bind port (the convention would be the top-level `bind_h3: Option<SockAddr>`).
- HTTP/3 is auto-discovered for any remote services (with HTTPS RRs, and
  alt-svc headers) by the client. Forgetting HTTP/3 support statuses across
  statelessness boundaries is fine, but static LazyLock/Mutex states can be kept
  by the lib crate.

```
pub static MUTEX: Mutex<MyType> = Mutex::new(MyType::new());
// or
pub static MUTEX: LazyLock<Arc<Mutex<MyType>>> =
  LazyLock::new(|| Arc::new(Mutex::new(MyType::new())));
```

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

### MIME module at `mechanics-core`

If it is not too offtopic, add a structured MIME composer
(it doesn't need to know about HTML, etc., just formats)
to `mechanics-core`: `mechanics:mime`. It should handle
Base64, multipart messages, etc, cleanly, emitting standard
compiant MIME messages.

```js
import { compose, parse } from `mechanics:mime`;
```

Let's make every non-endpoint module feature-gated:

- Pre-existing modules are enabled by default.
- features:
  - `rand` (default): enables `mechanics:rand` and
   `mechanics:uuid`.
    without it, `Math.random()` is seeded with zero.
  - `encoding` (default): form-urlencoded, base64, base32,
    hex.
  - `url` (default): a new WHATWG URL API-compliant API.
  - `console` (default): a minimal WHATWG-compliant console
    API.
  - `mime` (non-default): see the above.
  - `html` (default): the new `html` (`htmlize::escape_text()`
    to `escapeText()`, `htmlize::escape_all_quotes()` to
    `escapeAttribute()`, `htmlize::unescape()` to 
    `unescapeText()`, `htmlize::unescape_attribute()`
    to `unescapeAttribute()`) module.

Please note that `jsdom` would not work with Mechanics,
which on purpose doesn't have any non-ES globals.

```js
import URL, { URLSearchParams } from `mechanics:url`;
import console from `mechanics:console`;
```

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
