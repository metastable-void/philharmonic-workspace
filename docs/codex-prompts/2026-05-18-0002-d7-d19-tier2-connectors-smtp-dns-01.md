# D7 + D19 — Tier-2 connector implementations: SMTP + DNS (batched)

**Date:** 2026-05-18 (JST)
**Slug:** `d7-d19-tier2-connectors-smtp-dns`
**Round:** 01 — initial dispatch covering both crates in a single
round. Both are independent; either order is fine within the round.
**Subagent:** `codex:rescue`

## Motivation

§3.K Audit & refactor closed 2026-05-18 ([`docs/ROADMAP.md`](../ROADMAP.md)).
That clears the gate on §3.B Tier-2 connector work. This dispatch
batches D7 (`philharmonic-connector-impl-email-smtp`) and D19
(`philharmonic-connector-impl-dns`) — both are Tier 2, both go from
`0.0.x` placeholder to `0.1.0` substantive release, and neither
touches the crypto path. The connector-service framework already
verifies tokens, decrypts payloads, and dispatches; each
implementation only has to fill in the
[`philharmonic_connector_impl_api::Implementation`] trait.

Tier 3 (D8 Anthropic + D9 Gemini) is a separate, later batch — out
of scope here.

**No crypto-review-protocol gate.** Both implementations operate on
the already-decrypted `config` and `request` JSON values that the
framework hands them. No SCK, no COSE, no payload-hash, no `pht_`
tokens.

## References (read in this order; authoritative if anything in
this prompt contradicts them)

1. [`docs/ROADMAP.md` §3.B](../ROADMAP.md#b-phase-7-tier-23-connector-implementations-4-dispatches)
   — the canonical spec for both dispatches (hard requirements
   locked 2026-05-12 via HUMANS.md, repeated below).
2. [`docs/design/08-connector-architecture.md` §SMTP](../design/08-connector-architecture.md#smtp)
   and [`§DNS`](../design/08-connector-architecture.md#dns) — wire
   shapes, config shapes, error cases.
3. [`docs/design/08-connector-architecture.md` §"Security
   boundary"](../design/08-connector-architecture.md#security-boundary)
   and [`§"v1 implementation set"`](../design/08-connector-architecture.md#v1-implementation-set)
   — what the framework does vs. what the impl does.
4. [`CONTRIBUTING.md`](../../CONTRIBUTING.md):
   - **§4** Git workflow — `./scripts/commit-all.sh` only.
   - **§5** Script wrappers — every cargo call routes via the
     wrappers (which set `CARGO_TARGET_DIR=target-main`).
   - **§10.3** No panics in library `src/` — no `.unwrap()` /
     `.expect()` on `Result`/`Option`, no `panic!` /
     `unreachable!` / `todo!` / `unimplemented!` on reachable
     paths. Tests are exempt.
   - **§10.4** Library crates take bytes, not file paths — file
     I/O / env-var lookup / config-file parsing belong in bins.
     Both impls are libraries; they receive `config` and `request`
     as decrypted `JsonValue`s and never read the filesystem
     themselves.
   - **§10.9** HTTP-client split — runtime crates use `reqwest` +
     rustls-tls + tokio, or hyper + rustls. D7 uses `lettre` (SMTP,
     not HTTP); D19 uses `mechanics-dns` (already disciplined). No
     `reqwest` needed in either.
   - **§10.15** No implicit host-file deps in runtime libraries
     (no direct `/etc/resolv.conf` reads, no direct `/etc/hosts`
     reads). DNS lookups go through `mechanics-dns`.
   - **§11** Pre-landing checks — `./scripts/pre-landing.sh` is
     mandatory before the final commit.
   - **§14.6** English as the default for prose.
   - **§Banned-dep posture** (see `deny.toml` and §3.J archive):
     `pyo3`, `maturin`, `openssl-sys`, `native-tls`,
     `rustls-platform-verifier`, `rustls-native-certs`, and `ring`
     are no-wrapper full bans on the workspace's ship targets
     (`x86_64-unknown-linux-{gnu,musl}`). `ring` exception list
     restricted to `quinn-proto` only. If a feature trim would
     introduce a new banned-dep path, pick a different feature
     combination.
5. [`philharmonic-connector-impl-llm-openai-compat/src/lib.rs`](../../philharmonic-connector-impl-llm-openai-compat/src/lib.rs)
   — working `Implementation` trait impl exemplar. Mirror its
   shape: `mod` decomposition (`config`, `request`, `response`,
   `error`, plus impl-specific modules), `pub use`s at the top of
   `lib.rs`, `NAME` constant, `Self::new()` constructor,
   `#[async_trait] impl Implementation` block delegating to private
   helpers. Don't reinvent.
6. [`philharmonic-connector-impl-api/src/lib.rs`](../../philharmonic-connector-impl-api/src/lib.rs)
   — the `Implementation` trait, `ImplementationError` variants,
   `ConnectorCallContext`. Re-imports
   `ImplementationError::{InvalidConfig, InvalidRequest,
   UpstreamError, UpstreamUnreachable, UpstreamTimeout,
   SchemaValidationFailed, ResponseTooLarge, Internal}`.
   `is_retryable()` covers the upstream-unreachable / -timeout /
   internal cases.
7. [`bins/philharmonic-connector/src/main.rs`](../../bins/philharmonic-connector/src/main.rs)
   (`build_implementation_registry` at line ~313) — where new
   impls register with the per-realm bin. Both new impls must be
   added here.
8. [`philharmonic/Cargo.toml`](../../philharmonic/Cargo.toml) and
   [`philharmonic/src/lib.rs`](../../philharmonic/src/lib.rs) —
   meta-crate features + re-exports. SMTP re-export already
   exists at lib.rs:73 (`connector_impl_email_smtp`) and feature
   `connector-email-smtp` already exists at Cargo.toml:148
   (currently optional, off-by-default until `0.1.0` lands). DNS
   has **no** re-export and **no** feature yet — both must be
   added.

## Context files pointed at

**D7 (SMTP) crate** — `philharmonic-connector-impl-email-smtp/`
(submodule, currently `0.0.1` placeholder with empty deps and a
two-line `src/lib.rs` comment):

- `Cargo.toml` — replace with real deps + bump to `0.1.0`.
- `src/lib.rs` — replace the placeholder with the impl.
- new `src/{config,request,validate,mime,connect,error}.rs`
  modules (or similar — module split is your call, mirroring
  `llm-openai-compat`).
- new `tests/` directory with unit + integration test files.
- `CHANGELOG.md` — add the `0.1.0` entry.
- `README.md` — keep the existing shape; expand the one-line
  description to a short paragraph describing the realm /
  capability / config / request shape (still ≤ 30 lines, mirror
  `llm-openai-compat/README.md`).

**D19 (DNS) crate** — `philharmonic-connector-impl-dns/`
(submodule, currently `0.0.0` placeholder with empty deps and a
two-line `src/lib.rs` comment):

- Same shape as above.
- `Cargo.toml` — real deps + bump to `0.1.0`.
- `src/lib.rs` + modules (`config`, `request`, `policy`, `error`,
  etc. — your call).
- `tests/` with unit + integration test files.
- `CHANGELOG.md` — `0.1.0` entry.
- `README.md` — short paragraph mirroring the SMTP/openai-compat
  shape.

**Wiring (parent repo):**

- `bins/philharmonic-connector/src/main.rs` —
  `build_implementation_registry`: add two new
  `register_implementation(..., EmailSmtp::new()?)` and
  `register_implementation(..., DnsQuery::new()?)` calls (names
  follow the actual constructor shape each impl exposes; see
  `LlmOpenaiCompat::new()` for the exemplar).
- `philharmonic/Cargo.toml` —
  - bump the `philharmonic-connector-impl-email-smtp` dep
    version pin from `0.0.1` to `0.1.0`; keep `optional = true`.
  - add a new `philharmonic-connector-impl-dns = { version =
    "0.1.0", optional = true, default-features = false }` dep.
  - add a new `connector-dns = ["dep:philharmonic-connector-impl-dns"]`
    feature.
  - add `connector-email-smtp` AND `connector-dns` to the
    `default = [...]` array (both now ship; both default-on, like
    every other ship-eligible connector).
  - update the meta-crate-level comment in the `[features]`
    block that calls out "`connector-llm-anthropic` /
    `-llm-gemini` / `-email-smtp` stay off-by-default until
    their `0.1.0` lands" — remove `-email-smtp` from that list,
    leaving anthropic + gemini as the still-stubbed ones.
- `philharmonic/src/lib.rs` — add a `pub use
  philharmonic_connector_impl_dns as connector_impl_dns;`
  re-export under the `connector-dns` cfg-gate, mirroring the
  existing impl re-exports (lines 48, 51, 54, 57, 60, 63, 73).
  The SMTP re-export already exists at line 73 — verify and
  leave alone unless its cfg-gate also needs adjustment now that
  the feature moves to default-on.
- Workspace root `Cargo.toml` — the
  `[patch.crates-io]` overrides already cover both crates
  (line 72 SMTP, line 78 DNS) and both are already workspace
  members (line 28 SMTP, line 27 DNS). No edit there expected;
  verify.
- `philharmonic/CHANGELOG.md` — add a meta-crate patch-bump entry
  noting the SMTP/DNS connector default-on additions.
  Patch-bump the meta-crate version per
  `./scripts/crate-version.sh`.

## D7 (SMTP) — implementation spec

Identifiers (locked):

- Crate: `philharmonic-connector-impl-email-smtp` `0.1.0`.
- Realm: `email`.
- Implementation name (returned by `Implementation::name()`):
  `email_smtp`.

Transport: **`lettre`** over rustls, async (`tokio` runtime).
Lettre's TLS feature surface is non-trivial — pick the feature
combination that gives you:

- `rustls` (no `native-tls`).
- aws-lc-rs as the rustls crypto provider (no `ring`).
- webpki-roots as the trust store (no `rustls-native-certs`, no
  `rustls-platform-verifier`).
- tokio-async (no blocking client).
- Smtp transport, no file/sendmail transport.

A reasonable starting point is `lettre = { version = "0.11",
default-features = false, features = ["smtp-transport",
"tokio1", "tokio1-rustls-tls", "builder", "webpki-roots"] }` —
**verify against `lettre`'s actual `Cargo.toml` at its current
released version that this combination produces a clean
aws-lc-rs + webpki-roots tree** (`cargo tree -e features -p
lettre` and `cargo tree --invert ring --target all`). If
`lettre`'s rustls feature pulls `ring` transitively, switch to
whichever feature name selects aws-lc-rs (rustls 0.23+ uses
`tls12 + aws_lc_rs` provider features; lettre may surface this
as `rustls-tls`, `rustls-tls-aws-lc`, or similar — pick what
exists in the published version). If you cannot get a
ring-free tree with the published lettre, **STOP and report**
— do not vendor or fork lettre as part of this dispatch.

### Hard requirements

These are locked. **No flexibility.**

- **Port 25 rejected unconditionally** at config-validation time
  (not at runtime — fail fast in `InvalidConfig`).
- **Username and password required.** Anonymous submission
  refused at config-validation time (`InvalidConfig`).
- **Explicit `connection_mode` knob** in the endpoint config —
  `starttls` | `smtps` | `auto` (default). When set explicitly,
  wins over port-driven inference.
- **Port-driven defaults** (apply only when `connection_mode` is
  `auto`):
  - `587` → STARTTLS.
  - `465` → SMTPS (implicit TLS).
  - any other port → STARTTLS by default.
- **Auto-discovery** (`connection_mode = "auto"` AND no port
  configured): try `587/STARTTLS` first, then `465/SMTPS`. First
  successful TLS handshake wins. Auto-discovery is per-call
  (lazy) — no connection pooling needed for v1.
- **Four-valued `tls_strictness` enum**:
  - `strict` (default) — full TLS verification (certificate +
    hostname).
  - `sloppy` — encryption required, server identity skipped.
  - `opportunistic` — TLS attempted; full verification when
    negotiated; plaintext otherwise.
  - `opportunistic_sloppy` — TLS attempted; verification skipped
    when negotiated; plaintext otherwise.
  Independent of `connection_mode` — applies whichever transport
  is used.
- **Request shape** `{mail_from, recipients[], body}` with
  **minimal MIME envelope fixing**:
  - Insert `MIME-Version: 1.0` if absent.
  - Insert `Date:` if absent (current wall-clock; UTC fine).
  - Insert `Message-Id:` if absent (RFC 5322 §3.6.4
    conventions; use a hostname-stable suffix like
    `<uuid>@philharmonic.local` — `Uuid::new_v4` from
    `philharmonic-connector-common::Uuid`).
  - Insert `Content-Type: text/plain; charset=utf-8` if absent.
  - Normalise line endings to CRLF.
  - **Never inject security-relevant headers** — no
    `From:` rewriting, no DKIM/SPF/DMARC-relevant additions,
    no `Authentication-Results:` injection, no `Received:`
    munging.
  - When the body already has the header, pass through
    verbatim — don't replace, don't reformat.

### Config shape (canonical)

```json
{
  "host": "smtp.example.com",
  "port": 587,
  "connection_mode": "starttls",
  "username": "alerts@example.com",
  "password": "...",
  "tls_strictness": "strict"
}
```

- `host`: required string. Submission server hostname.
- `port`: optional u16. **Validation: reject `25` with
  `InvalidConfig`.** Omitting the port enables auto-discovery
  only when `connection_mode` is `auto`.
- `connection_mode`: optional enum `{starttls, smtps, auto}`,
  default `auto`. Use serde rename to lowercase.
- `username` / `password`: both required, both non-empty
  strings. Reject empty/missing with `InvalidConfig`.
- `tls_strictness`: optional enum `{strict, sloppy,
  opportunistic, opportunistic_sloppy}`, default `strict`.
  Lowercase via serde rename.

### Request shape (canonical)

```json
{
  "mail_from": "alerts@example.com",
  "recipients": ["ops@example.com", "oncall@example.com"],
  "body": "<full MIME message, CRLF or LF, ≥ 1 line>"
}
```

- `mail_from`: required, non-empty string. Used for SMTP `MAIL
  FROM`. **No address-syntax validation beyond non-emptiness**
  — lettre will reject what the SMTP server would.
- `recipients`: required non-empty array of strings. Used for
  SMTP `RCPT TO`.
- `body`: required, non-empty string. Full MIME message, headers
  + blank line + body. The implementation applies the MIME
  envelope fixing above before handing to lettre.

### Response shape

On success:

```json
{
  "accepted": true,
  "message_id": "<the Message-Id used, including angle brackets>"
}
```

- `accepted`: always `true` when the implementation returns
  `Ok(...)`. The framework converts errors to
  `ImplementationError`; success implies the SMTP submission
  server accepted the message.
- `message_id`: the `Message-Id:` header value the implementation
  used — either the one already in the body or the one it
  inserted. Useful for log correlation.

### Error mapping

Use `philharmonic_connector_common::ImplementationError`
variants:

- Invalid config → `InvalidConfig { detail }`. Examples: port
  25, missing username, missing password, malformed
  `connection_mode`, unknown `tls_strictness`.
- Invalid request → `InvalidRequest { detail }`. Examples:
  missing `mail_from`, empty `recipients`, empty `body`,
  body that fails MIME parsing in a non-recoverable way.
- TLS handshake failure (strict / sloppy modes) →
  `UpstreamUnreachable { detail }`.
- SMTP authentication failure → `UpstreamError { status: 535,
  body: "<server response or 'authentication failed'>" }`.
  The status field is overloaded for SMTP — 535 is the
  RFC 4954 / RFC 5321 auth-failure code.
- SMTP server rejection (`5xx`) → `UpstreamError { status:
  <smtp-code-as-u16>, body: "<server response>" }`. If the
  server returns 4xx (transient), surface as `UpstreamError`
  too — the framework's retry logic uses `is_retryable()`,
  and `UpstreamError` is non-retryable by current policy. v1
  does not auto-retry SMTP submissions.
- SMTP connection timeout / connect failure →
  `UpstreamUnreachable { detail: "<context>" }`.
- Hostname resolution failure → `UpstreamUnreachable { detail }`.
  lettre handles DNS internally; surface the error as
  unreachable.
- Internal serialisation / unexpected lettre error →
  `Internal { detail }`.

### Connection-mode test matrix (unit / integration)

Cover at minimum:

1. `connection_mode = "starttls"`, port 587 → STARTTLS.
2. `connection_mode = "smtps"`, port 465 → SMTPS.
3. `connection_mode = "auto"`, port 587 → STARTTLS (port-driven).
4. `connection_mode = "auto"`, port 465 → SMTPS (port-driven).
5. `connection_mode = "auto"`, port 2525 → STARTTLS (other-port
   default).
6. `connection_mode = "auto"`, no port → auto-discovery (587
   first; if that connects with TLS, win).
7. `connection_mode = "starttls"`, port 465 → STARTTLS on 465
   (operator override wins over port inference; documented
   in design §Connection policy).
8. `connection_mode = "smtps"`, port 587 → SMTPS on 587
   (operator override wins).
9. Port 25 with any `connection_mode` → `InvalidConfig`.

### MIME envelope fixing tests

Cover at minimum:

1. Body already has all four headers → pass through verbatim
   (CRLF normalisation may still apply).
2. Body has none of the four headers → all four inserted.
3. Body has `MIME-Version:` but lowercase `mime-version:` —
   case-insensitive detection, don't double-insert.
4. Body has `Date:` with an obsolete-but-valid value → leave
   alone (don't reformat to current).
5. Body uses LF-only line endings → normalised to CRLF.
6. Body uses mixed LF/CRLF → normalised to CRLF throughout.
7. Body has a `From:` header — pass through verbatim (no
   rewriting, no enrichment).
8. Empty body → `InvalidRequest`.

### TLS strictness tests

- Verify `strict` produces a default rustls config that uses
  webpki-roots + aws-lc-rs.
- Verify `sloppy` selects a permissive verifier (skips
  hostname + certificate validation but keeps encryption).
- Verify `opportunistic` accepts plaintext when the server
  does not offer STARTTLS / refuses TLS — exact mock-server
  shape is up to you; an integration test via a `lettre`-
  compatible mock SMTP container under `dockerlet` is the
  preferred shape if feasible, otherwise unit-level coverage
  of the config-to-lettre-transport translation is acceptable.
- `opportunistic_sloppy` — similar; verify it picks the
  permissive verifier when TLS negotiates.

If you can't get a real-server integration test to pass
reliably in the workspace's CI shape (host firewall, sandbox
constraints), prefer unit-level coverage of the
config → transport-builder translation. Record the call in
the codex-report. Do not block the dispatch on a flaky
integration test.

### Out of scope (D7)

- **No connection pooling** in v1. Each `execute` call opens a
  fresh SMTP connection. Pooling is a later optimisation.
- **No DKIM signing.** The framework does not sign messages;
  operators that need DKIM put it on the submission server.
- **No SPF / DMARC enforcement.** Receiving infrastructure
  problem, not a connector concern.
- **No SMTP AUTH mechanism negotiation override.** lettre's
  default mechanism selection is what ships.
- **No SOCKS / HTTP proxy support.** Future feature.
- **No mailbox listing, IMAP, POP3.** Submission-only.
- **No transactional-email-provider HTTP APIs** (SendGrid,
  Postmark, Mailgun). Those are served by `http_forward` with
  per-provider configs.
- **No MIME composition.** Workflow authors hand-write the
  body, or use the `mechanics:mime` realm module (mechanics-
  core 0.6.0). D7 only fixes envelopes; it never assembles
  bodies.

## D19 (DNS) — implementation spec

Identifiers (locked):

- Crate: `philharmonic-connector-impl-dns` `0.1.0`.
- Realm: `dns`.
- Implementation name (returned by `Implementation::name()`):
  `dns_query`.

Resolver backend: **`mechanics-dns`** (in-tree crate at
`mechanics-dns/`, `0.1.0`, already published — see
[`mechanics-dns/src/lib.rs`](../../mechanics-dns/src/lib.rs)).
**NO direct `hickory-resolver`, `hickory-proto`, or any other
`hickory-*` dependency in D19's `Cargo.toml`.** If you need a
hickory type or function (e.g. `RecordType`, `ResponseCode`),
add a `pub use` re-export to `mechanics-dns` and consume it
from there. The user's explicit instruction (2026-05-18):

> no direct hickory deps; re-export things from mechanics-dns
> if needed.

`mechanics-dns` already re-exports `RecordType` and
`ResponseCode` at the top of its `lib.rs` (lines 27–28); plus
`parse_record_type` (line 210), `DnsRecord` (line 219), and
`HttpsRecord` (line 248). If you need additional surface, add
the `pub use` re-export in `mechanics-dns/src/lib.rs`,
patch-bump `mechanics-dns` to `0.1.1`, and add a CHANGELOG
entry there too. Keep the surface minimal.

`mechanics-dns::Resolver` is the entry point:
- `Resolver::new()` — loads system config, falls back to
  Cloudflare on `/etc/resolv.conf` ENOENT.
- `Resolver::query(name, RecordType)` — generic IN-class
  query returning `Vec<DnsRecord>`.
- `Resolver::lookup_a` / `lookup_aaaa` / `lookup_ip` /
  `lookup_socket_addrs` / `lookup_https` — typed helpers.

For D19, use `query(...)` for the generic shape and let the
caller's `type` field drive the `RecordType`.

### Hard requirements

These are locked.

- **Arbitrary DNS querying via the system's stub resolver.**
  `mechanics-dns::Resolver::new()` handles this; no further
  resolver-config logic in D19.
- **Resolv.conf fallback on ENOENT.** Already handled inside
  `mechanics-dns`; D19 does nothing extra. Other read errors
  (permission denied, malformed file) surface as
  `Resolver::new()` errors → mapped to
  `ImplementationError::Internal` at construction.
- **`IN` class only.** `mechanics-dns` enforces this; D19
  inherits.
- **Endpoint config carries optional `allowed_types`,
  `allowlist_zones`, `blocklist_zones`** — see config shape
  below. Policy gates apply **before** any DNS packet leaves
  the process (no observable network side-effect from a
  blocked query).
- **Both-list semantics**: when both `allowlist_zones` and
  `blocklist_zones` are set, a query passes if the name
  matches at least one allowlist entry AND does not match any
  blocklist entry. Blocklist is a strict overlay-deny.
- **Zone matching is suffix-based, case-insensitive.**
  `example.com` matches `example.com`, `foo.example.com`,
  `bar.foo.example.com`, but not `notexample.com`. Implement
  as exact-match-on-final-labels (split on `.`, compare from
  right), not substring `contains` — `notexample.com`
  contains `example.com` lexically.
- **Per-call timeout.** `request.timeout_ms` overrides
  `config.default_timeout_ms`; if neither is set, use a
  conservative default (5000 ms). Apply via
  `tokio::time::timeout` around the `Resolver::query` call —
  `mechanics-dns` doesn't take a per-call timeout currently.

### Config shape (canonical)

```json
{
  "allowed_types": ["A", "AAAA", "MX", "TXT"],
  "allowlist_zones": ["example.com", "example.org"],
  "blocklist_zones": ["secret.example.com"],
  "default_timeout_ms": 5000
}
```

- `allowed_types`: optional `Vec<String>`. When set, only
  queries whose `type` parses to one of these RR types pass.
  When omitted, all standard RR types are allowed. Type
  strings match canonical IANA names (`A`, `AAAA`, `CNAME`,
  `MX`, `NS`, `PTR`, `SOA`, `SRV`, `TXT`, `CAA`, `HTTPS`,
  …). Use `mechanics-dns::parse_record_type` to canonicalize
  and validate.
- `allowlist_zones`: optional `Vec<String>`. Domain suffixes.
- `blocklist_zones`: optional `Vec<String>`. Domain suffixes.
- `default_timeout_ms`: optional `u64`. Per-query timeout
  default; clamp to a sane range
  (e.g. ≥ 100 ms, ≤ 60_000 ms).

### Request shape (canonical)

```json
{
  "name": "www.example.com",
  "type": "MX",
  "timeout_ms": 3000
}
```

- `name`: required string. Domain name to query (caller is
  responsible for IDN encoding if non-ASCII).
- `type`: required string. RR type as canonical IANA name.
- `timeout_ms`: optional `u64`. Per-query timeout; falls back
  to `default_timeout_ms`.

### Response shape (canonical)

```json
{
  "records": [
    {"type": "A", "name": "www.example.com", "ttl": 300,
     "data": "93.184.216.34"},
    {"type": "MX", "name": "example.com", "ttl": 3600,
     "data": "10 mail.example.com"}
  ]
}
```

- `records`: array of objects with `type` (string, IANA
  name), `name` (string, DNS presentation form), `ttl`
  (`u32`, seconds), `data` (string, **rdata in
  presentation-form by default**).
- **v1 sub-shape decision: presentation-format strings,
  not per-type structured objects.** `mechanics-dns::DnsRecord`
  already emits this shape via `record.data` (the
  `RData::to_string()` form). Per-type structured objects
  (e.g. `{priority, exchange}` for MX) are a later
  enhancement if workflow authors need them — out of scope
  for D19.

### Error mapping

- Unknown RR type in `request.type` →
  `InvalidRequest { detail: "unknown record type '<t>'" }`.
- Empty / missing `name` →
  `InvalidRequest { detail: "name required" }`.
- Zone outside `allowlist_zones` (when set) →
  `InvalidRequest { detail: "zone '<name>' not in allowlist" }`.
  **Connector-level deny — return before any DNS packet leaves
  the process.**
- Zone matching `blocklist_zones` →
  `InvalidRequest { detail: "zone '<name>' blocklisted" }`.
  Same — pre-network deny.
- RR type not in `allowed_types` (when set) →
  `InvalidRequest { detail: "type '<t>' not in allowed_types" }`.
- Per-call timeout exceeded → `UpstreamTimeout`.
- Resolver error with `RCODE NXDOMAIN` / `SERVFAIL` /
  `NOTIMP` / `REFUSED` → `UpstreamError { status:
  <rcode-as-u16>, body: "<rcode-name>: <name> <type>" }`. Use
  the numeric DNS RCODE (3 = NXDOMAIN, 2 = SERVFAIL, 4 =
  NOTIMP, 5 = REFUSED) as the status overload.
- `Resolver::new()` failure (e.g. `/etc/resolv.conf`
  permission denied; the ENOENT path is already handled
  inside `mechanics-dns`) → at construction, propagate as
  `ImplementationError::Internal { detail }` from
  `DnsQuery::new()?`. If the constructor lazily defers to
  first-call, propagate the same error then.
- Other resolver errors → `Internal { detail }` carrying the
  underlying `mechanics-dns::Error` `Display` form.

### Test matrix (unit)

Cover at minimum:

**Policy gate:**

1. No allowlist, no blocklist, no allowed_types → all queries
   pass policy.
2. Allowlist `["example.com"]` → `www.example.com` passes,
   `notexample.com` denies, `EXAMPLE.com` passes (case
   insensitive), `bar.foo.example.com` passes.
3. Blocklist `["secret.example.com"]` → `secret.example.com`
   denies, `child.secret.example.com` denies,
   `public.example.com` passes.
4. Both lists: allowlist `["example.com"]` + blocklist
   `["secret.example.com"]` →
   `public.example.com` passes,
   `secret.example.com` denies (overlay-deny),
   `notexample.com` denies (not in allowlist).
5. `allowed_types = ["A", "AAAA"]` with `type = "MX"` →
   `InvalidRequest`.
6. `allowed_types = ["A", "AAAA"]` with `type = "A"` → pass.
7. Empty `name` → `InvalidRequest`.
8. Unknown RR type string → `InvalidRequest`.

**Timeout:**

9. `request.timeout_ms` overrides `config.default_timeout_ms`.
10. Neither set → uses the 5000 ms default.

**RCODE mapping** (integration, via mechanics-dns against a
controlled mock if you can wire one; otherwise unit-level
coverage of the error-mapping helper):

11. `NXDOMAIN` → `UpstreamError { status: 3, body: "..." }`.
12. `SERVFAIL` → `UpstreamError { status: 2, body: "..." }`.

If you cannot reliably wire a controlled DNS server inside
the test environment, skip (11) + (12) and cover the
error-mapping helper at unit level with synthetic
`mechanics-dns::Error::Lookup{...}` instances. Document in
the codex-report.

### Out of scope (D19)

- **No DoH / DoT / DoQ.** v1 is stub-resolver only. If the host
  OS is configured for DoT/DoH (e.g. systemd-resolved), the
  stub-resolver path picks that up transparently.
- **No per-query resolver override** (the config can't pick a
  resolver; that's the host's job).
- **No caching beyond the OS / hickory's built-in caches.**
- **No reverse DNS (PTR) special-casing.** PTR is just another
  RR type from D19's perspective — the caller assembles the
  `<reverse>.in-addr.arpa` name.
- **No per-type structured response objects** in v1
  (presentation-form strings only).
- **No DNSSEC validation** explicitly turned on (whatever
  `mechanics-dns` exposes by default is what ships).
- **No EDNS Client Subnet (ECS) plumbing.**

## Wiring sequence

1. Land D7's code + tests + CHANGELOG inside the
   `philharmonic-connector-impl-email-smtp` submodule.
2. Land D19's code + tests + CHANGELOG inside the
   `philharmonic-connector-impl-dns` submodule.
3. Update `philharmonic/Cargo.toml`:
   - Bump SMTP dep pin to `0.1.0`.
   - Add DNS dep entry at `0.1.0`.
   - Add `connector-dns` feature.
   - Add `connector-email-smtp` and `connector-dns` to
     `default = [...]`.
   - Remove `-email-smtp` from the "stays off-by-default"
     comment.
4. Update `philharmonic/src/lib.rs`:
   - Add `pub use philharmonic_connector_impl_dns as
     connector_impl_dns;` under the `connector-dns`
     feature-gate (mirror the existing cfg-gate shape).
   - Verify the SMTP re-export at line 73 is still correct
     given the feature is now default-on.
5. Update `bins/philharmonic-connector/src/main.rs`:
   - Add two new `register_implementation(&mut registry,
     ...)` calls in `build_implementation_registry` for the
     SMTP and DNS impls.
   - Verify the implementations are reachable via
     `philharmonic::connector_impl_email_smtp::...` and
     `philharmonic::connector_impl_dns::...`.
6. Patch-bump `philharmonic` (meta-crate) for the
   features-and-deps change; add a CHANGELOG entry.
7. Patch-bump `bins/philharmonic-connector` if its
   `Cargo.toml` version field is tracked (it's
   `publish = false`, so no version bump needed).
8. Verify with `./scripts/pre-landing.sh`.

## Per-crate version policy

| Crate | Old | New | CHANGELOG |
|---|---|---|---|
| `philharmonic-connector-impl-email-smtp` | `0.0.1` | `0.1.0` | New `0.1.0` entry — first substantive release; capability `email_smtp`; lettre over rustls-aws-lc-rs + webpki-roots; per spec above. |
| `philharmonic-connector-impl-dns` | `0.0.0` | `0.1.0` | New `0.1.0` entry — first substantive release; capability `dns_query`; mechanics-dns-backed; per spec above. |
| `philharmonic` (meta-crate) | look up via `./scripts/crate-version.sh` | patch-bump | Note: SMTP + DNS connector default-on additions; `connector-dns` feature added. |
| `mechanics-dns` | `0.1.0` | `0.1.1` ONLY IF re-exports were added | Note: re-exports added for D19. |

**Do NOT bump** any crate not listed above. **Do NOT publish.**
Yuka publishes via `./scripts/publish-crate.sh` after review.

## Verification (mandatory before declaring done)

Run (once, at the end):

```sh
./scripts/pre-landing.sh
```

Must print `=== pre-landing: all checks passed ===` at the end.

Run, after pre-landing is green:

```sh
CARGO_TARGET_DIR=target-main cargo tree --workspace --invert ring --target x86_64-unknown-linux-musl 2>&1 | head -60
CARGO_TARGET_DIR=target-main cargo tree --workspace --invert ring --target x86_64-unknown-linux-gnu 2>&1 | head -60
```

`ring` must only appear via the `quinn-proto` wrapper exception
already documented in `deny.toml`. If either D7 or D19 introduces
a new `ring` path, that's a feature-flag mistake — fix at the
dep declaration, not by widening `deny.toml`.

```sh
CARGO_TARGET_DIR=target-main cargo tree --workspace --invert native-tls --target all 2>&1 | head -20
CARGO_TARGET_DIR=target-main cargo tree --workspace --invert rustls-native-certs --target all 2>&1 | head -20
CARGO_TARGET_DIR=target-main cargo tree --workspace --invert rustls-platform-verifier --target all 2>&1 | head -20
CARGO_TARGET_DIR=target-main cargo tree --workspace --invert hickory-resolver -p philharmonic-connector-impl-dns 2>&1 | head -20
```

The four banned-dep checks must print `package not found in the
dependency graph` (or equivalent). The hickory check **for the
DNS impl crate** must show hickory reachable ONLY via
`mechanics-dns` — never as a direct dependency of
`philharmonic-connector-impl-dns`.

```sh
CARGO_TARGET_DIR=target-main cargo deny check bans
```

Must be clean.

If the box becomes contended mid-pre-landing, check with
`./scripts/xtask.sh resource-pressure` and back off if `load1/cpus`
climbs well above 1.0.

Do not run raw `cargo fmt` / `cargo clippy` / `cargo test` —
`pre-landing.sh` covers them and uses the right
`CARGO_TARGET_DIR`.

## Hand-off shape: Codex does not commit

**Leave the working tree dirty.** Claude commits via
`./scripts/commit-all.sh` after reviewing the diff. The script
has a `codex-guard` (`scripts/lib/codex-guard.sh`) that walks
the ancestor process chain and aborts if any process is named
`*codex*`; calling `commit-all.sh` from inside a Codex run
will hard-fail. Do not work around the guard.

Specifically:

- Do **not** run `./scripts/commit-all.sh` (any flags,
  including `--dry-run` and `--parent-only` and `--exclude`).
- Do **not** run raw `git commit` / `git push` / `git add`.
  The pre-commit hooks enforce signoff + signature +
  `Audit-Info:` trailer; the codex-guard fires from those hooks
  too.
- Do **not** run `git commit --no-verify` / `--no-gpg-sign`.
- Do **not** run `git reset` / `git rebase` / `git amend`.
  History is append-only.
- Do **not** run `./scripts/push-all.sh`. Claude pushes after
  reviewing.
- Do **not** run `./scripts/publish-crate.sh`. Yuka publishes.
- Do **not** edit `HUMANS.md`. Agent-readable,
  agent-writable forbidden.

Edits land in the working tree across the parent repo and the
touched submodules:

- `philharmonic-connector-impl-email-smtp/` submodule.
- `philharmonic-connector-impl-dns/` submodule.
- `mechanics-dns/` (in-tree, not a submodule) iff re-exports
  added.
- Parent repo: `philharmonic/` (submodule), `bins/`, `Cargo.lock`
  (regenerated automatically by `cargo build`).

Claude inspects the per-submodule + parent dirty state, runs
`./scripts/commit-all.sh --dry-run` to confirm scope, then
runs the real commit-all that handles submodule-first ordering
and writes signoff/signature/audit-trailer per commit.

Codex's session summary should mention which submodules have
dirty trees so Claude knows where to look.

## Codex report (encouraged)

If anything non-obvious surfaced during this round — a design
call you had to make on the fly (lettre feature combination
that landed, MIME-fixing edge case, mock-server choice for
SMTP integration coverage), a blocker you worked around, a
residual concern for Yuka — write a short report to
`docs/codex-reports/2026-05-18-0002-d7-d19-tier2-smtp-dns.md`
per [`docs/codex-reports/README.md`](../codex-reports/README.md).
Routine specified-and-shipped work doesn't need one; the
session summary covers it. Leave the report **dirty** in the
working tree; Claude commits it alongside the implementation
diff.

If you skip the report, say so in the session summary.

## Outcome

D7 SMTP and D19 DNS landed as Tier-2 connector implementations with
`philharmonic-connector-impl-email-smtp` and
`philharmonic-connector-impl-dns` both bumped to `0.1.0`, and the
`philharmonic` meta-crate bumped to `0.3.5`; `mechanics-dns` did not
need surface changes or a version bump. Files touched: parent
`Cargo.lock`, `bins/philharmonic-connector/Cargo.toml`,
`bins/philharmonic-connector/src/main.rs`, this prompt archive, and
`docs/codex-reports/2026-05-18-0002-d7-d19-tier2-smtp-dns.md`;
`philharmonic/{Cargo.toml,CHANGELOG.md,src/lib.rs}`;
SMTP crate `Cargo.toml`, `CHANGELOG.md`, `README.md`, `src/lib.rs`,
`src/{config,connect,error,mime,request,response}.rs`, and
`tests/{config,mime}.rs`; DNS crate `Cargo.toml`, `CHANGELOG.md`,
`README.md`, `src/lib.rs`, `src/{config,error,policy,request,response}.rs`,
and `tests/{policy,timeout}.rs`. The SMTP crate uses lettre with
`features = ["aws-lc-rs", "builder", "smtp-transport", "tokio1",
"tokio1-rustls", "webpki-roots"]` to keep rustls + aws-lc-rs while
avoiding `native-tls`; `cargo tree --target all --invert ring` shows
the existing lockfile `ring` path is still `quinn-proto` only, and
the GNU/Linux musl/gnu target-specific ring checks print nothing.
Blockers worked around: the sandbox could not write the default Cargo
or rustup homes, so verification used `CARGO_HOME=/tmp/codex-cargo-home`
and carried a non-fatal rustup temp-dir warning; no implementation
blockers remain. Residual risks: coverage is policy/MIME/config/error
focused, with no live SMTP server or live DNS server integration in this
round; that tradeoff is documented in the Codex report. Base SHAs for
the dirty hand-off are parent `0bc2e1ca0e9c9930fef89f19ddf2a52267e6c86b`,
`philharmonic` `707c86c4bb31609a5ae1d0e4306c8f522e3ef069`, SMTP
`2eb62e5ac1ce8546c4645bc3bacae1ba6b8e5eae`, and DNS
`c7517c96ae9345c6ae4e6f906cda84eac42d68d1`; Codex created no commits.

---

<task>
Implement two Tier-2 connector crates per ROADMAP §3.B and
design 08 §SMTP / §DNS — `philharmonic-connector-impl-email-smtp`
(`0.0.1` → `0.1.0`, capability `email_smtp`, lettre over
rustls-aws-lc-rs + webpki-roots) and `philharmonic-connector-
impl-dns` (`0.0.0` → `0.1.0`, capability `dns_query`,
mechanics-dns backend with no direct hickory deps). Both
implement the shared `philharmonic_connector_impl_api::Implementation`
trait. Neither touches the crypto path — the framework
verifies tokens and decrypts payloads before `execute` is
called.

**Reference docs (authoritative if they contradict this prompt):**

1. `docs/ROADMAP.md` §3.B — the canonical spec for both
   dispatches.
2. `docs/design/08-connector-architecture.md` §SMTP and §DNS —
   wire shapes, config shapes, error cases.
3. `CONTRIBUTING.md` §§4, 5, 10.3, 10.4, 10.9, 10.15, 11, 14.6,
   and the banned-dep posture documented in `deny.toml`.
4. `philharmonic-connector-impl-llm-openai-compat/src/lib.rs` —
   working `Implementation` impl exemplar; mirror its module
   decomposition.
5. The full preamble above (this prompt's `## …` sections,
   especially the per-crate "implementation spec",
   "test matrix", "wiring sequence", and "verification" blocks).

**Hard constraints (locked):**

- **D19 has no direct `hickory-*` deps.** Backend is
  `mechanics-dns` only. If you need a hickory type, re-export
  it from `mechanics-dns` and consume it from there. User's
  explicit instruction 2026-05-18.
- **D7 uses lettre over rustls with aws-lc-rs +
  webpki-roots.** No `native-tls`, no `ring` (beyond the
  `quinn-proto` wrapper exception in `deny.toml`), no
  `rustls-platform-verifier`, no `rustls-native-certs`.
- **Port 25 rejected unconditionally at SMTP config
  validation.** No runtime check needed — fail fast in
  `InvalidConfig`.
- **DNS policy gates fire before network I/O.** A blocked
  query has no observable network side-effect.
- **All connector libraries take bytes, not paths.** Decrypted
  `config` and `request` come in as `JsonValue`; no file
  reads, no env-var lookups inside `src/`. Per
  CONTRIBUTING.md §10.4.
- **No panics in `src/`.** Per CONTRIBUTING.md §10.3. Tests
  exempt.

**Per-crate version bumps + CHANGELOG entries:** SMTP `0.0.1`
→ `0.1.0`; DNS `0.0.0` → `0.1.0`; `philharmonic` meta-crate
patch-bump; `mechanics-dns` `0.1.0` → `0.1.1` ONLY IF you
added re-exports for D19. **Do not publish.**

**Wiring (parent repo):** add both impls to the meta-crate
features + re-exports + dep pins (`connector-dns` is new;
`connector-email-smtp` exists but moves to default-on); add
both to the connector bin's
`build_implementation_registry`; remove `-email-smtp` from
the meta-crate's "stays off-by-default until 0.1.0 lands"
comment.

**Verification (must run + pass before declaring done):**

- `./scripts/pre-landing.sh` — clean.
- `cargo tree --workspace --invert ring --target
  x86_64-unknown-linux-{gnu,musl}` — `ring` only via
  `quinn-proto`.
- `cargo tree --workspace --invert native-tls
  --invert rustls-native-certs --invert
  rustls-platform-verifier` (each separately) — empty.
- `cargo tree --invert hickory-resolver -p
  philharmonic-connector-impl-dns` — hickory reachable ONLY
  via `mechanics-dns`, not as a direct D19 dep.
- `cargo deny check bans` — clean.

**Non-goals (explicit):** no connection pooling, no DKIM /
SPF / DMARC for SMTP, no DoH / DoT / DoQ for DNS, no per-type
structured DNS response objects, no D8 / D9 (Tier 3 — out of
scope), no crypto-path changes, no `[profile.release]` edits,
no banned-dep widening.

<default_follow_through_policy>
Codex is expected to land **both crates plus the wiring** in
this single round. "D7 done, D19 pending" is **not** a
complete result — keep going. Both crates are independent of
one another, so order within the round is your call.

If one crate hits a hard blocker (e.g. lettre's feature
matrix doesn't expose an aws-lc-rs path in the published
version), **STOP and report the blocker before touching the
other crate or any wiring**. Don't half-land. A blocker on
D7 doesn't justify landing D19 + partial D7 — surface the
blocker first so Claude can scope the fix.

If both crates land cleanly but `pre-landing.sh` fails on
something orthogonal (a pre-existing flake, a transient build
issue), **fix forward** if the fix is mechanical
(e.g. a clippy lint regression you can address narrowly).
If the failure is structural (a workspace-wide refactor
needed), **STOP and report**.
</default_follow_through_policy>

<completeness_contract>
"Complete" means:

1. `philharmonic-connector-impl-email-smtp/src/` has a real
   `Implementation` impl matching the locked spec.
2. `philharmonic-connector-impl-email-smtp/tests/` covers the
   connection-mode matrix, MIME envelope fixing, TLS
   strictness (at unit level minimum), and config-validation
   edge cases (port 25, missing username/password).
3. `philharmonic-connector-impl-dns/src/` has a real
   `Implementation` impl matching the locked spec, backed by
   `mechanics-dns` and **with no direct `hickory-*` deps**.
4. `philharmonic-connector-impl-dns/tests/` covers the policy
   gate matrix (allowlist / blocklist / both-list / type-gate)
   and timeout selection.
5. Both crates' `Cargo.toml` use
   `default-features = false` on every direct dep with
   explicit feature lists.
6. Both crates' `CHANGELOG.md` carry the new `0.1.0` entry.
7. `philharmonic/Cargo.toml` features + dep pins updated; meta-
   crate patch-bumped; meta-crate `CHANGELOG.md` entry added.
8. `philharmonic/src/lib.rs` re-exports D19; D7 re-export
   verified.
9. `bins/philharmonic-connector/src/main.rs` registry includes
   both new impls.
10. `mechanics-dns` re-exports added (if needed) +
    patch-bumped + CHANGELOG entry (if needed).
11. `./scripts/pre-landing.sh` passes.
12. Banned-dep tree checks pass per Verification block above.
13. Working tree left dirty across parent + touched submodules
    (per "Hand-off shape" above). **No commits, no pushes** —
    Claude commits and pushes after reviewing the diff.
14. Session summary lists which submodules + the parent have
    dirty trees so Claude can scope the `commit-all.sh` run.
15. Outcome section of this prompt file updated with: (a)
    list of files touched per crate, (b) version bumps issued,
    (c) lettre feature combination chosen + why, (d) any
    blockers encountered, (e) residual risks, (f) commit SHAs
    per submodule + parent.

If any of (1)–(14) is incomplete, the dispatch is INCOMPLETE.
Report INCOMPLETE clearly with what's done and what's left,
and STOP — don't synthesize a half-result.
</completeness_contract>

<verification_loop>
Per-crate (during implementation, between rounds of edits in
each crate):

  CARGO_TARGET_DIR=target-main cargo check -p philharmonic-connector-impl-email-smtp --all-targets
  CARGO_TARGET_DIR=target-main cargo check -p philharmonic-connector-impl-dns --all-targets

Per-crate tests:

  CARGO_TARGET_DIR=target-main cargo test -p philharmonic-connector-impl-email-smtp --all-targets
  CARGO_TARGET_DIR=target-main cargo test -p philharmonic-connector-impl-dns --all-targets

After wiring lands (meta-crate + bin):

  CARGO_TARGET_DIR=target-main cargo check -p philharmonic --all-features
  CARGO_TARGET_DIR=target-main cargo check -p philharmonic-connector --all-targets

Final, single run:

  ./scripts/pre-landing.sh

If `pre-landing.sh` fails, read the failure carefully:

1. If a single crate's clippy / doctest / test caused it,
   that's a local fix — make the fix, re-run pre-landing.
2. If it's a workspace-wide failure (e.g. type-mismatch in a
   crate you didn't touch), the wiring is wrong — likely a
   missing feature gate or a stale `pub use`. Fix the wiring,
   re-run.
3. If you're tight-looping pre-landing.sh on a slow box, run
   `./scripts/xtask.sh resource-pressure` first to confirm
   the host has headroom; back off if it doesn't.

Do not run raw `cargo fmt` / `cargo clippy` / `cargo test` —
`pre-landing.sh` covers them with the right `CARGO_TARGET_DIR`
and feature flags.

Do not run `cargo build --workspace` standalone as a "check"
— the per-crate `cargo check -p` covers it without the full
link-time cost.
</verification_loop>

<missing_context_gating>
Before you start editing, the workspace state must match the
prompt's claims:

  ./scripts/status.sh

Should print `(clean)` for the parent repo and all submodules.
If it doesn't, **STOP and report**. The prompt assumes a clean
starting tree — uncommitted changes in unrelated submodules
mean someone else is mid-edit; don't conflict.

If `lettre`'s current published version does not expose a
feature combination that gives you a ring-free aws-lc-rs +
webpki-roots tree, **STOP and report**. Do not:

- Vendor or fork lettre.
- Add `ring` to `deny.toml`'s wrapper exception list.
- Pull `rustls-native-certs` or `rustls-platform-verifier` as
  a "temporary" workaround.

Surface the lettre constraint and let Claude scope an
alternative (e.g. wait for upstream, swap to a different SMTP
library, vendor as a separate dispatch).

If `mechanics-dns`'s current public surface is missing a type
or function D19 needs (e.g. you need `RecordType::from_str`
behaviour different from `parse_record_type`), **add the
re-export to `mechanics-dns`** (`pub use ...`) — that's an
expected outcome and triggers the `mechanics-dns 0.1.1` bump.
Don't reach for `hickory-resolver` directly.
</missing_context_gating>

<action_safety>
- **You do not commit.** Leave the working tree dirty across
  parent + touched submodules. `./scripts/commit-all.sh` (any
  flags) and raw `git commit` / `git push` / `git add` /
  `git reset` / `git rebase` / `git amend` are all forbidden.
  The script's `codex-guard` will hard-abort if you try; the
  same guard fires from the pre-commit hooks. Claude commits +
  pushes after reviewing the diff.
- **Never** invoke `./scripts/push-all.sh`. Claude pushes.
- **Never** invoke `./scripts/publish-crate.sh`. Yuka publishes.
- **Never** edit `HUMANS.md`. Agent-readable, agent-writable
  forbidden.
- Every `cargo` invocation needs `CARGO_TARGET_DIR=target-main`
  (the wrappers in `scripts/` set this; if you call cargo
  directly, set it yourself).
- POSIX-ish host: no `bash`-only constructs in any shell you
  invoke. The wrappers are POSIX `#!/bin/sh`.
- The workspace's authoritative timezone is JST (Asia/Tokyo).
  Any wall-clock value you generate for the CHANGELOG, the
  CHANGELOG date, or the codex-report belongs in JST; today
  is 2026-05-18 (Mon).
- Resource pressure: at session start the host is idle
  (cpu 0.3%, load1/cpus 0.06, mem ~25% avail, swap ~4.9%).
  Run `./scripts/xtask.sh resource-pressure` before
  pre-landing if you want to confirm the box is still idle.
</action_safety>

<structured_output_contract>
At the end of the dispatch, return:

1. **Summary** (2-3 sentences): what landed; both crates' new
   versions; key numbers (lines of new code per crate, test
   counts, lettre feature combo chosen).
2. **Touched files**: full list, grouped by submodule + parent.
3. **Version bumps issued**: `crate@old → crate@new` for each
   (SMTP, DNS, philharmonic meta-crate, mechanics-dns iff
   touched). CHANGELOG entry confirmed for each.
4. **Lettre feature combination**: exact `features = [...]`
   array used; verification that the resulting tree is
   ring-free + native-tls-free + aws-lc-rs.
5. **mechanics-dns surface changes** (if any): re-exports
   added, why each was needed, version bump confirmation.
6. **Test coverage**: number of unit tests per crate, number
   of integration tests, anything skipped (e.g. flaky SMTP
   integration) with reasoning.
7. **Verification results**:
   - `pre-landing.sh`: PASS / FAIL (with one-line summary if
     FAIL).
   - `cargo tree --invert ring --target
     x86_64-unknown-linux-musl`: `quinn-proto`-only / FAIL.
   - `cargo tree --invert ring --target
     x86_64-unknown-linux-gnu`: `quinn-proto`-only / FAIL.
   - `cargo tree --invert native-tls --target all`: empty /
     FAIL.
   - `cargo tree --invert rustls-native-certs --target all`:
     empty / FAIL.
   - `cargo tree --invert rustls-platform-verifier --target
     all`: empty / FAIL.
   - `cargo tree --invert hickory-resolver -p
     philharmonic-connector-impl-dns`: via mechanics-dns
     only / FAIL.
   - `cargo deny check bans`: PASS / FAIL.
8. **Working-tree state at hand-off**:
   - List which submodules + the parent have dirty trees.
   - No commits expected from you. Claude will commit + push
     after reviewing the diff.
9. **Codex report**: if you wrote
   `docs/codex-reports/2026-05-18-0002-d7-d19-tier2-smtp-dns.md`,
   note its presence (dirty in working tree; Claude commits
   it). If you skipped, say so.
10. **Residual risks**: anything you'd flag for Claude or
    Yuka before publish.
11. **Outcome paragraph** for the prompt-archive file: 4–6
    sentences summarising the round for posterity, ready to
    drop into `## Outcome` of this file.
</structured_output_contract>
</task>
