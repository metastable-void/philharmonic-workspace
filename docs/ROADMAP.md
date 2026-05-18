# Philharmonic Implementation Roadmap

**Audience**: Claude Code, working in the `philharmonic-workspace`
repo.

**Purpose**: The authoritative implementation plan for Philharmonic.
V1 is complete; active work lives in the post-v1 dispatch plan
(§3 below). This file describes the **current state and what is
needed next** — past dispatches and closed arcs are no longer
enumerated here. Per-arc done-state snapshots live under
[`docs/archive/`](archive/) and per-crate release notes live in
each crate's `CHANGELOG.md`.

## Current status

- **v1 path: complete through Phase 9.** Reference deployment
  operational since 2026-05-02; the RAG-grounded chat use-case
  (embedding-dataset + `embed` + `vector_search` + `sql_postgres`
  + `llm_openai_compat`) is verified end-to-end against it.
- **Post-v1 internal work closed.** Embedding-datasets, WebUI
  infrastructure, connector enhancements, runtime semantics,
  module-surface refactor, HTTP-client transport + TLS posture,
  workspace tooling, HTTP/3 client + server, production-security
  dep cleanup, `mechanics-dns` extraction, and the §3.K Audit
  & refactor sweep have all landed.
- **Open**: §3.B Tier 2/3 connector implementations —
  D7 SMTP, D19 DNS (Tier 2, dispatched as a batch), then
  D8 Anthropic, D9 Gemini (Tier 3, dispatched separately).
  No gate blocks dispatch; specs in §3.B below.
- **Deferred (not yet scoped)**: a crypto-review-aware slice
  for the `lowerer.rs` / `embed_job.rs` extraction that was
  originally considered under §3.K but moved out because it
  touches SCK encrypt/decrypt and endpoint-payload handling.
  Will be sequenced once its Gate-1 proposal is ready.

Authoritative sources for things this file cross-references:

- **Conventions / dev environment / git workflow / pre-landing
  / scripts / publishing**: [`CONTRIBUTING.md`](../CONTRIBUTING.md)
- **Architecture / cross-cutting design** (observability, error
  envelope, permission atoms, API token format, canonical JSON,
  statelessness, etc.): [`docs/design/`](design/)
- **Operating principles for Claude / Codex**:
  [`CLAUDE.md`](../CLAUDE.md), [`AGENTS.md`](../AGENTS.md)
- **Two-gate crypto review protocol**:
  [`.claude/skills/crypto-review-protocol/SKILL.md`](../.claude/skills/crypto-review-protocol/SKILL.md)

If a design doc is wrong or incomplete, update the doc first,
then implement — **do not invent architectural decisions**.

---

## 1. v1 milestone archive (pointer)

The full pre-trim plans, definition-of-done, and completed-
crate inventory for Phases 0–9 (workspace setup through
reference deployment) are preserved verbatim at
[`docs/archive/2026-05-10-readme-roadmap-trim.md`](archive/2026-05-10-readme-roadmap-trim.md)
(under "Pre-trim `docs/ROADMAP.md`" → §4 "Completed v1 Milestone
Archive" and §8 "Definition of done for v1"). Historical
implementation detail also lives in dated
`docs/codex-prompts/`, `docs/codex-reports/`,
`docs/notes-to-humans/`, and `docs/crypto/{proposals,approvals}/`.

---

## 2. Crypto review protocol (pointer)

The two-gate cryptographic-review protocol (Gate 1 = approach
pre-approval; Gate 2 = post-implementation code review before
publish) lives in
[`.claude/skills/crypto-review-protocol/SKILL.md`](../.claude/skills/crypto-review-protocol/SKILL.md).
That file is the authoritative spec for what triggers the gates,
what each gate requires, and the test-vector discipline.

The Phase-5 / Phase-9 / 2026-05-04 Gate-1 records and approvals
are committed under `docs/crypto/{proposals,approvals}/`.

---

## 3. Post-v1 dispatch plan

Each numbered item below is one Codex dispatch with its own
archived prompt under `docs/codex-prompts/` (see
[`.claude/skills/codex-prompt-archive/SKILL.md`](../.claude/skills/codex-prompt-archive/SKILL.md)).

**Closed arcs** (§3.A, C–L): per-arc done-state snapshots live
under [`docs/archive/`](archive/) — per-day archives span
2026-05-10 through 2026-05-18. They are not enumerated here.

### B. Phase 7 Tier 2/3 connector implementations (4 dispatches)

Three of these (D7 / D8 / D9) take an existing `0.0.x`
placeholder to a `0.1.0` substantive implementation; D19 is
a **new crate** (no placeholder yet on crates.io, no
submodule wired into the workspace). None touch the crypto
path; the connector-service framework already validates
tokens and decrypts payloads — implementations only need
to implement the `Implementation` trait.

- **D7** `philharmonic-connector-impl-email-smtp` (Tier 2).
  Implement per
  [`docs/design/08-connector-architecture.md` §SMTP](design/08-connector-architecture.md#smtp).
  Hard requirements (locked 2026-05-12 via HUMANS.md):
  - **Port 25 rejected** unconditionally.
  - **Username + password required**; anonymous submission
    refused at config validation.
  - **Explicit `connection_mode` knob** in the endpoint
    config: `starttls` / `smtps` / `auto` (default). When
    set, wins over the port-driven inference. The full
    SMTP server config — host, port, credentials, mode,
    strictness — lives in the endpoint config and is
    never visible to workflow code.
  - **Port-driven defaults** (when `connection_mode` is
    `auto`): 587 → STARTTLS, 465 → SMTPS, otherwise
    STARTTLS. **Auto-discovery** (`auto` + no port):
    try 587/STARTTLS, then 465/SMTPS.
  - **Four-valued `tls_strictness` enum**: `strict`
    (default), `sloppy`, `opportunistic`,
    `opportunistic_sloppy`. Independent of
    `connection_mode`.
  - **Request shape** `{mail_from, recipients[], body}`
    with minimal MIME envelope fixing (insert
    `MIME-Version` / `Date` / `Message-Id` / default
    `Content-Type` only when the submission server would
    reject otherwise; CRLF-normalise line endings; never
    inject security-relevant headers).
  - Transport: `lettre` over `rustls`.
- **D8** `philharmonic-connector-impl-llm-anthropic` (Tier 3).
- **D9** `philharmonic-connector-impl-llm-gemini` (Tier 3) —
  must support **both** Google API surfaces for Gemini:
  - **Google AI Studio**
    (`https://generativelanguage.googleapis.com/`): API-key
    auth, simplest single-tenant deployment shape, free-
    tier-friendly.
  - **Vertex AI on GCP**: Service Account JSON key auth.
    The SA JSON lives **inside** the SCK-encrypted endpoint
    config alongside the API-Studio mode's API key —
    consistent with how `llm-openai-compat` carries its
    `api_key` field. Encryption-at-rest is handled by the
    existing SCK boundary; per-tenant credential rotation
    happens via the existing endpoint-config rotation flow.
    Endpoint shape under
    `<region>-aiplatform.googleapis.com/v1/projects/<project>/`.

  The runtime endpoint config carries a discriminator
  selecting which mode is active; the impl handles auth
  + endpoint construction accordingly per mode. Detailed
  shape (discriminator field name, exact field names for
  the Vertex mode's project / region / SA JSON,
  OAuth2 access-token caching for Vertex AI) defers to
  D9's prompt-drafting time.

- **D19** `philharmonic-connector-impl-dns` (Tier 2,
  **new crate**). Implement per
  [`docs/design/08-connector-architecture.md` §DNS](design/08-connector-architecture.md#dns).
  Hard requirements (locked 2026-05-12 via HUMANS.md):
  - Arbitrary DNS querying via the system's stub
    resolver (consults `/etc/resolv.conf`). No custom
    recursive resolver, no caching layer beyond the OS.
  - **Resolv.conf fallback**: when `/etc/resolv.conf` is
    `ENOENT` (typical in minimal-base / distroless /
    scratch container images), fall back to a hardcoded
    Cloudflare resolver set:
    `2606:4700:4700::1111`,
    `2606:4700:4700::1001`,
    `1.1.1.1`,
    `1.0.0.1`. Any other read error
    (permission denied, malformed file, I/O) surfaces as
    a startup error — fallback fires only on ENOENT.
  - `IN` class only.
  - Endpoint config carries optional `allowed_types`
    (RR type allowlist), `allowlist_zones`, and
    `blocklist_zones` (domain-suffix gates). When both
    lists exist: query passes if its zone is
    allowlisted AND not blocklisted. Blocklist is a
    strict overlay-deny.
  - Resolver library: `hickory-resolver` (formerly
    `trust-dns-resolver`) in system-config mode —
    pure-Rust, async, no `unsafe`.
  - Capability name `dns_query`; realm `dns`. Request
    `{name, type, timeout_ms?}`; response
    `{records: [{type, name, ttl, data}, ...]}`. v1
    emits rdata as presentation-format strings;
    per-type structured objects are a sub-shape
    decision at prompt-drafting time.

  Pre-D19 setup is complete (2026-05-12): submodule wired,
  crates.io `0.0.x` placeholder published. Claude drafts
  the D19 prompt; Codex implements + tests inside the
  submodule. No crypto gate — DNS connector is HTTP-realm
  style (token + encrypted payload over the connector
  framework); just one more `Implementation` trait impl.
  D19 consumes the in-tree `mechanics-dns` resolver layer.

Independent of one another; safe to run in parallel.

### Suggested sequencing

**Next dispatchable**: Tier-2 batch = D7 + D19, dispatched
together. Tier 3 (D8 + D9) follows separately.

- **D7** (`philharmonic-connector-impl-email-smtp`, Tier 2)
  — `email_send` wire shape locked in
  [`docs/design/08-connector-architecture.md` §SMTP](design/08-connector-architecture.md#smtp);
  prompt draft ready.
- **D19** (`philharmonic-connector-impl-dns`, Tier 2) —
  fully spec'd from
  [`docs/design/08-connector-architecture.md` §DNS](design/08-connector-architecture.md#dns);
  consumes the in-tree `mechanics-dns` resolver layer.
  Prompt draft ready.
- **D8** (`philharmonic-connector-impl-llm-anthropic`,
  Tier 3) — fully spec'd from
  [`docs/design/08-connector-architecture.md` §llm_anthropic](design/08-connector-architecture.md#llm_anthropic--config);
  prompt draft ready.
- **D9** (`philharmonic-connector-impl-llm-gemini`, Tier 3)
  — dual-mode AI Studio + Vertex AI; Claude proposes the
  discriminator field, Vertex-mode field names, and OAuth2
  access-token caching strategy in the prompt; Yuka
  overrides at prompt-review time if she has a preference.

### Dispatch discipline reminder

Per [`.claude/skills/codex-prompt-archive/SKILL.md`](../.claude/skills/codex-prompt-archive/SKILL.md)
and [`CONTRIBUTING.md §15.2`](../CONTRIBUTING.md#152-codex-prompt-archive):
every Codex prompt is archived to
`docs/codex-prompts/YYYY-MM-DD-NNNN-<slug>.md` and committed
**before** the spawn. Each dispatch above corresponds to one
archived prompt; multi-round work shares the daily-sequence
`NNNN` and increments the trailing `-NN`.
