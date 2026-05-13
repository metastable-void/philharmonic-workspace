# Philharmonic Implementation Roadmap

**Audience**: Claude Code, working in the `philharmonic-workspace`
repo.

**Purpose**: The authoritative implementation plan for Philharmonic.
V1 is complete through the first working end-to-end deployment;
active work now lives in the post-v1 dispatch plan (§3 below).

**Current state** (2026-05-12):

- Design: complete.
- v1 implementation path: **complete through Phase 9.**
- Reference deployment: operational since 2026-05-02; a
  WebUI-created workflow runs end-to-end through API, mechanics
  worker, connector router/service, and an OpenAI-compatible
  upstream LLM.
- Post-v1 quick wins **D1** (LONGBLOB substrate migration),
  **D2** (`MechanicsJob.run_timeout` override), **D10**
  (CodeMirror 6 in the WebUI) landed in unified Codex dispatch
  on 2026-05-02 (`ee2bd61`).
- **Embedding-datasets feature: shipped end-to-end 2026-05-10.**
  Both crypto gates cleared. **Gate 1** (Approach B —
  synthesized non-persisted `EntityId<WorkflowInstance>`, no
  public-trait change, no crypto-shape change) approved at
  [`crypto/approvals/2026-05-04-post-v1-embed-dataset-lowerer-ephemeral.md`](crypto/approvals/2026-05-04-post-v1-embed-dataset-lowerer-ephemeral.md).
  **Gate 2** approved at
  [`crypto/approvals/2026-05-04-post-v1-embed-dataset-lowerer-ephemeral-gate-2.md`](crypto/approvals/2026-05-04-post-v1-embed-dataset-lowerer-ephemeral-gate-2.md).
  Implementation: D3 round 01 (`bbc26f9` data layer) + D3
  round 02 (`b134d44` workflow-engine `data` assembly + 7 API
  routes + template `data_config`) + D4+D5+caps+409
  (`e37f956`) + Gate-2 hardening (`e845101` + `1a6b4c8`
  deferred-tasks cleanup) + D6 WebUI (`b581b50`).
- **D12** (`llm-openai-compat` `custom_headers` knob, Hugging
  Face `X-HF-Bill-To` driver) shipped 2026-05-10 (`2fff3bb`).
- **D13** (chat-style testing UI in WebUI per HUMANS.md §"Chat
  UI for easy testing") shipped 2026-05-10 (`ee99b79`
  philharmonic submodule + `58cf408` parent). One-click
  create-and-chat from `TemplateDetail` / `Templates`,
  third tab on `InstanceDetail` with empty-content dual-
  purpose probe, runtime structural detection of the
  `{messages: [{role, content}, ...]}` shape, localStorage
  for last-used instance + scroll position. No backend
  changes.
- **D11** (workflow authoring guide rewrite, English)
  shipped 2026-05-10 (`10acd7f`). 530 → 1350 lines
  reflecting current implementation reality, three
  load-bearing recipes (D13 chat, embedding-datasets,
  combined RAG).
- **D11 follow-ups + Late-Sunday fix-its** all landed
  2026-05-10 (JP mirror regeneration, WebUI
  `data_config` structured editor, design/07 + /10
  reconciliation, `scripts/build-status.sh` extension,
  config-paste UX guide callout, connector-path body cap
  2 MiB → 32 MiB). Verbatim detail + commit SHAs
  preserved at
  [`docs/archive/2026-05-11-roadmap-completed-arc-trim.md`](archive/2026-05-11-roadmap-completed-arc-trim.md).
- **2026-05-11 HUMANS.md follow-up dispatches done**:
  D16 (`tool_call_fallback_auto` dialect — `e523238`
  submodule + `b368c4b` parent); D14 (markdown rendering
  in chat with DOMPurify) + D15 (`abstract_config`
  structured editor) — bundled in `f750b4a` philharmonic
  submodule + `c1fbff7` parent. All from
  [`HUMANS.md` §"Follow-up tasks from 2026-05-10 work"](../HUMANS.md).
- **2026-05-11 deployment-time polish** (not numbered
  Codex dispatches; surfaced during real testing):
  - `mechanics-core` **0.3.2 → 0.4.0** (`5cbe72c` +
    `6ed5ee2`) — runtime stopped overriding `main`'s
    fulfilled-promise success with "Unhandled promise
    rejection" engine errors. Boa's
    `NativeFunction::from_async_fn` rejection-tracker
    didn't balance reliably across the await-wrapper
    chain; the strict check produced false-positives
    for any workflow with `try { await endpoint(...) }
    catch (e) { ... }`. Module-evaluation-time check
    kept strict.
  - `philharmonic-api` **0.1.7 → 0.1.8** (`ab7bc25` +
    `d19cc76`) — `WhoamiResponse` extended with
    `permissions: Vec<String>` (effective atom set
    after envelope clipping). Powers the WebUI nav /
    button filtering below; additive on the wire.
  - **WebUI permission-aware nav + disabled
    non-actionable buttons + sticky sidebar footer**
    (Codex r01) — sidebar hides routes the caller has
    no read permission for; action buttons across all
    15 pages render `disabled` with title-attribute
    tooltips naming the missing atom instead of letting
    users click into a 403; `usePermissions` hook reads
    from `authSlice.permissions`. Server-side route-
    protector enforcement unchanged (still the security
    boundary). Sticky-footer fix via `.sidebar
    position: sticky; max-height: 100vh`.
  - **Assistant `name` field bubble surfacing**
    (`afbc660` + `0c95618`) — D13 chat tab renders an
    OpenAI-style assistant `name` (non-empty string) as
    the bubble role label in place of the generic
    "Assistant" / "アシスタント".
  - **Workflow authoring guide per-connector
    request/response shapes** (`9f96e2d`) — every
    shipped connector subsection in `docs/guide/
    workflow-authoring.md` (en + jp) now has explicit
    Request body + Response body tables;
    `http_forward`'s `response.body.body` double-nest
    semantics called out.
  - **Audit-log producer gap closed** —
    `philharmonic-policy` 0.2.2 → 0.2.3 (`b37f894`)
    ships the `audit_event_type` module with 17
    canonical i64 discriminants;
    `docs/design/09-policy-and-tenancy.md §Audit trail`
    contract lock-in (`1ce191a`) covers the `event_data`
    JSON schema, token-mint payload privacy restriction
    (subject_id + authority_id only; never injected
    claims), and the audit-write failure semantics (log
    warn + return success on underlying mutation);
    `philharmonic-api` (`881c48a` + `8d20d1d`) wires 19
    producer call sites across 7 route files
    (principals, roles, memberships, endpoints,
    authorities, mint, operator) using a shared
    `emit_audit_event` helper, with 7 e2e tests
    (mint.rs's enforces the privacy restriction by
    absence-assertion). Open follow-up design questions
    queued: separate `AUTHORITY_ROTATED = 34`
    discriminant?, future `TENANT_MODIFIED` for
    non-status updates?, `GET /v1/audit` response
    surfacing canonical names via
    `audit_event_type::name`?
  - Pre-D15 detail and per-day work preserved in the
    archive linked above.
- Yuka was on Golden Week 2026-04-29 → 2026-05-06 plus a
  personal vacation 2026-05-07 / 05-08; first regular working
  day back was Mon 2026-05-11. Real deployment-time
  testing started this week and has been the source of
  the post-D15 polish work above.
- **End-to-end PoC milestone — 2026-05-11 evening**: a
  complete chatbot use-case ran successfully on the
  deployment, exercising the full retrieval + DB +
  LLM stack in a single workflow:
  - **Retrieval**: embedding dataset (`embed_datasets`
    feature, D3/D4/D5/D6) + `embed` connector
    (`philharmonic-connector-impl-embed`, BGE-M3 via
    tract/ONNX, inline-blob model bundling) + `vector_search`
    connector (`philharmonic-connector-impl-vector-search`,
    stateless).
  - **Relational data**: `sql_postgres` connector
    (`philharmonic-connector-impl-sql-postgres`).
  - **LLM**: `llm_openai_compat` connector pointing at
    OVHCloud's Hugging Face Inference Provider endpoint
    serving `Qwen/Qwen3-32B`; uses D12's `custom_headers`
    knob for the HF billing header.
  - **Path**: API server → mechanics worker → connector
    router → connector service → external upstreams, with
    workflow steps composed in the workflow script using
    today's per-connector wire-shape documentation (en/jp).
  - All deployment-time fixes that landed earlier 2026-05-11
    (mechanics-core 0.4.0 unhandled-rejection,
    permission-aware WebUI, audit-log producer wiring,
    connector body cap 2 MiB → 32 MiB) were either
    triggered by or validated against this PoC session.
  - This is the **first full real-world chatbot RAG flow**
    on the platform — proves the platform's stated use-case
    (RAG-grounded chat over a vector index + relational DB,
    served by a self-or-partner-hosted LLM) is now real,
    not just integration-test scaffolding.
- **2026-05-12 work** (post-PoC, day-after-the-milestone):
  - **D17 landed** — `mechanics-core` 0.4.0 → 0.4.1
    (`0e6c3e7` submodule + `743e091` parent). Worker
    run-job response now returns when the script's
    top-level settles; unawaited promises and endpoint
    calls continue polling on the worker tokio task
    until quiescence or `max_execution_time`. Authoritative
    behavior spec at [`design/06` §Tail-promise polling](design/06-execution-substrate.md#tail-promise-polling).
    Codex chose sub-shape B (`RunJobsExit` enum +
    `run_jobs_until(predicate)` helper); `tracing = "0.1"`
    dep + `setTimeout` global builtin added (the
    `setTimeout` global is being reverted under D18 per
    HUMANS.md's "no non-ES globals" hard rule; see §3.F).
  - **D7 wire shape locked** — HUMANS.md surfaced a
    complete SMTP submission spec (port-25 ban, port-driven
    TLS, four-valued strictness enum, request shape,
    minimal MIME envelope fixing). Locked into
    [`design/08` §SMTP](design/08-connector-architecture.md#smtp);
    `email_send wire shape` removed from §Open questions.
    D7 entry in §3.B updated to point at it as the
    authoritative spec.
  - **D18 added** — `mechanics-core` module-surface
    refactor: feature-gate every non-endpoint module +
    ship four new modules (`mime` non-default; `url`,
    `console`, `html` default). HUMANS.md §"MIME module at
    `mechanics-core`" is the driver. New §3.F entry.
  - **WebUI chat tab** also got two small follow-ups:
    read-only step-history probe (so terminated instances
    render correctly) and step-elapsed `"n.n s"` muted
    caption below the transcript.
  - **WebUI CodeMirror TAB capture** wired in (HUMANS.md
    §WebUI follow-up): TAB now indents inside the editor,
    Shift+TAB unindents, Escape-then-TAB releases focus
    for keyboard navigation.
  - **D19 added** — `philharmonic-connector-impl-dns`
    (Tier 2 DNS connector, new crate) surfaced via
    HUMANS.md. Spec locked into
    [`design/08` §DNS](design/08-connector-architecture.md#dns).
    Submodule wired (`scripts/new-submodule.sh`),
    crates.io 0.0.0 placeholder published. D19 ready
    for prompt draft.
  - **Ring removal** (parent `7723e1c`): switched three
    sqlx-using crates from `runtime-tokio-rustls` (which
    expanded to `tls-rustls-ring`) to `runtime-tokio +
    tls-rustls-aws-lc-rs`. All three release bins
    (`mechanics-worker`, `philharmonic-api-server`,
    `philharmonic-connector-bin`) now show
    `aws-lc-rs 1.16.3` as the sole runtime crypto
    provider with no `ring` in the dep tree.
  - **NUMERIC overflow regression fix** (parent
    `18f1bb2`): sql-postgres now decodes NUMERIC via
    `sqlx::types::BigDecimal` instead of `String`, so
    `Error::IntegerOverflow → UpstreamError` actually
    fires for `SELECT 9223372036854775808::numeric`-class
    queries (was silently being raised as `Internal`
    with a type-mismatch detail since cc4e991 /
    2026-04-24, only surfaced today because pre-landing's
    --ignored Docker phase only runs the postgres
    connector's tests when its crate is touched).
    `bigdecimal` added to sqlx feature list.
  - **D20 added** — workspace-wide webpki-roots-only TLS
    trust posture. sqlx is already
    aws-lc-rs+webpki-roots after the ring removal;
    reqwest (every outbound HTTP path) still picks up
    platform-native via `rustls-platform-verifier` +
    `rustls-native-certs`. D20 forces webpki-roots
    everywhere via
    `ClientBuilder::use_preconfigured_tls()` with a
    `RootCertStore` populated from
    `webpki_roots::TLS_SERVER_ROOTS`. See §3.G.
    Codex prompt archived at
    [`docs/codex-prompts/2026-05-12-0002-d20-webpki-roots-only-tls-01.md`](codex-prompts/2026-05-12-0002-d20-webpki-roots-only-tls-01.md);
    pivoted 2026-05-13 from runtime-bypass to a new
    `mechanics-http-client` crate and landed the same day —
    see §3.G and
    [`docs/archive/2026-05-13-roadmap-d20-done.md`](archive/2026-05-13-roadmap-d20-done.md).
  - **HTTPS hardening** (parent `876b6fe`,
    `mechanics` 0.4.1 → 0.4.2): every HTTPS response now
    stamped with
    `Strict-Transport-Security: max-age=63072000`
    (2 years, RFC 6797-scoped to HTTPS-only paths via
    middleware on the three release bins;
    `includeSubDomains` intentionally omitted so adjacent
    non-HTTPS subdomains aren't broken — operators add
    that at the reverse proxy). rustls `ServerConfig`
    cipher-suite list pruned of AES128 across all three
    bins, leaving AES256-GCM + CHACHA20-POLY1305 across
    TLS 1.3 and TLS 1.2. Other rustls defaults (KX
    groups, signature schemes, ALPN preferences,
    protocol versions) unchanged.
  - **D21 added** — `scripts/pre-landing.sh` dep-aware
    test filtering. Today the script runs
    `cargo test --workspace` for the default phase
    regardless of which crates were modified; after the
    workspace grew to ~25 crates with substantial Boa +
    crypto build costs, that's several minutes of work
    per run that mostly re-tests unaffected crates.
    D21 narrows the default phase to the union of dirty
    crates and their transitive reverse-dependency
    closure (workspace-wide for `scripts/`-, `Cargo.toml`-,
    or `Cargo.lock`-dirty runs; `--full` flag forces the
    pre-D21 behavior). See §3.H.
  - **Staging deployment** (end of day 2026-05-12):
    Yuka deploying the release build (musl-static,
    `--features https` default-on via the new
    `release-build.sh` flag) to the staging server.
    First deployment that carries the post-PoC arc end-
    to-end: D17 tail-promise polling +
    sql-postgres NUMERIC fix + sqlx aws-lc-rs ring
    removal + HSTS/cipher hardening + CodeMirror TAB
    capture + chat tab read-only-probe + step-elapsed
    indicator + DNS connector placeholder.
  - Pre-rewrite ROADMAP text preserved verbatim at
    [`docs/archive/2026-05-12-roadmap-d17-done-d7-spec-d18-added.md`](archive/2026-05-12-roadmap-d17-done-d7-spec-d18-added.md).

Authoritative sources for things this file used to restate but
now cross-references:

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

## 1. Completed v1 milestone archive

Phases 0–9 (workspace setup → reference deployment) all landed.
The detailed per-phase plans, definition-of-done, completed-
crate inventory, and pre-Phase-9 cross-cutting notes were
trimmed from this roadmap on 2026-05-10. The full pre-trim text
is preserved verbatim at
[`docs/archive/2026-05-10-readme-roadmap-trim.md`](archive/2026-05-10-readme-roadmap-trim.md)
(under "Pre-trim `docs/ROADMAP.md`" → §4 "Completed v1 Milestone
Archive" and §8 "Definition of done for v1").

One-line summary: **Phase 0** workspace setup, **Phase 1**
`mechanics-config` extraction, **Phase 2** `philharmonic-policy`,
**Phase 3** `philharmonic-connector-common`, **Phase 4**
`philharmonic-workflow`, **Phase 5** connector triangle (client +
service + router) under Yuka's two-gate crypto review, **Phase 6**
first connector implementations (`http_forward`,
`llm_openai_compat`), **Phase 7 Tier 1** SQL Postgres / SQL MySQL /
stateless vector search / local embedding (with `inline-blob`),
**Phase 8** `philharmonic-api 0.1.0`, **Phase 9** integration +
reference deployment (2026-05-02).

Historical implementation detail also lives in dated
`docs/codex-prompts/`, `docs/codex-reports/`,
`docs/notes-to-humans/`, and
`docs/crypto/{proposals,approvals}/` files.

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

Phase 9 is complete (2026-05-02) and the reference deployment is
operational. The work below is post-v1 / post-GW: it does not
block deployment and is sequenced for the next development
cycle. Each numbered item is one Codex dispatch with its own
archived prompt under `docs/codex-prompts/` (see
[`.claude/skills/codex-prompt-archive/SKILL.md`](../.claude/skills/codex-prompt-archive/SKILL.md)).
The single `(Gate 1)` item is **not** a Codex dispatch — Claude
drafts the proposal, Yuka reviews per the two-gate crypto-review
protocol (§2).

Total: **24 Codex dispatches plus 1 Gate-1 proposal.**
**D1, D2, D3, D4, D5, D6, D10, D11, D12, D13, D14, D15, D16,
D17, D20, D21 are done** (16 of 24; D20 + D21 both landed via
Claude-direct implementation on 2026-05-13, with user override
on the default Codex-dispatch path). Gate 1 and Gate 2 both
approved.

**Sequencing directive (Yuka, 2026-05-13):** production-
security cleanups (§3.J — D23 testcontainers replacement +
D24 default-features audit) take **priority over** every
other pending dispatch, including the Tier 2/3 connector
work (D7 / D8 / D9 / D19) and D18 (mechanics-core module
refactor). Framing: "serious production-ready security" —
clean release-binary runtime trees and minimised per-dep
feature surface are baseline requirements, not optional
polish. D22 server-integration is the one exception that
co-sequences with §J (it's part of the in-flight D22 arc).

Remaining, in landing order:
- **D22 server-integration** (in-flight; wire
  `mechanics-http-server` into `mechanics` +
  `philharmonic-api` + `philharmonic-connector-service`
  + the three bins' `bind_h3: Option<SocketAddr>` config
  fields).
- **D23** (in-tree minimal testcontainers replacement;
  drops bollard's `rustls-native-certs` pull — see §3.J).
- **D24** (workspace-wide `default-features = false`
  audit; per-dep feature trim across every workspace
  Cargo.toml — see §3.J).
- D18 (`mechanics-core` module-surface refactor: feature
  gating + new `mime`/`url`/`console`/`html` modules + the
  2026-05-13 setTimeout-global removal per HUMANS.md's
  "no non-ES globals" hard rule — see §3.F).
- D7 / D8 / D9 / D19 (Tier 2/3 connectors: SMTP /
  Anthropic / Gemini / DNS).

### A. Embedding datasets (6 dispatches + 1 Gate-1) — DONE

Authoritative design:
[`docs/design/16-embedding-datasets.md`](design/16-embedding-datasets.md).

Both gates approved and all six dispatches landed
2026-05-10: D1 LONGBLOB substrate migration, D2
`MechanicsJob.run_timeout` override, D3 backend (two
rounds — entity + codec, then engine `data` assembly +
API routes), D4 lowerer ephemeral support per Approach B,
D5 ephemeral embed job + caps + 409-on-Embedding, D6
WebUI surface end-to-end. Per-dispatch detail and commit
SHAs preserved verbatim at
[`docs/archive/2026-05-11-roadmap-completed-arc-trim.md`](archive/2026-05-11-roadmap-completed-arc-trim.md).

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

  **Pre-D19 setup** (Yuka, one-time):
  1. Create the
     `philharmonic-connector-impl-dns` GitHub repo
     under `metastable-void` (mirror the layout of
     the existing connector submodules).
  2. Reserve the crate name on crates.io by publishing a
     `0.0.1` placeholder.
  3. Add a submodule entry to `.gitmodules` + the
     workspace root.
  4. Add the crate to the workspace `Cargo.toml`
     `[workspace] members` list.

  After setup, Claude drafts the D19 prompt; Codex
  implements + tests inside the new submodule. No
  crypto gate — DNS connector is HTTP-realm style
  (token + encrypted payload over the connector
  framework); just one more `Implementation` trait
  impl.

Independent of one another and of section A; safe to run in
parallel (modulo the D19 setup prerequisite).

### C. Connector enhancements (2 dispatches)

- **D12** `llm_openai_compat` `custom_headers` knob —
  **DONE 2026-05-10 (`2fff3bb`).** Per-provider header
  pass-through (Hugging Face `X-HF-Bill-To`, OpenAI
  `OpenAI-Organization`, OpenRouter `HTTP-Referer`, etc.)
  in the runtime endpoint config, with reserved-header
  rejection and CRLF guards at config-validation time.
  Full per-dispatch rationale and shape detail preserved
  at
  [`docs/archive/2026-05-11-roadmap-completed-arc-trim.md`](archive/2026-05-11-roadmap-completed-arc-trim.md).

- **D16** `llm_openai_compat` `tool_call_fallback_auto`
  dialect variant — **DONE 2026-05-11** (`e523238`
  submodule + `b368c4b` parent). New variant alongside
  the existing `tool_call_fallback`; sends
  `tool_choice: "auto"` instead of the forced
  function-name literal, for providers that reject the
  forced form. `philharmonic-connector-impl-llm-openai-compat`
  0.1.1 → 0.1.2 (patch bump per pre-1.0 SemVer). Full
  shape detail at
  [`docs/archive/2026-05-11-roadmap-completed-arc-trim.md`](archive/2026-05-11-roadmap-completed-arc-trim.md).

### D. WebUI infrastructure, features, and docs (5 dispatches)

Three landed:

- **D10** CodeMirror 6 in the WebUI — **DONE 2026-05-02
  (`ee2bd61`).**
- **D11** Workflow authoring guide rewrite (English) —
  **DONE 2026-05-10 (`10acd7f`).** 530 → 1350 lines with
  three load-bearing recipes (D13 chat, embedding-datasets,
  combined RAG). JP mirror regenerated same day (`e159e88`
  docs-jp + `6913a9d` parent).
- **D13** Chat-style testing UI in `philharmonic/webui` for
  `{content}` → `{messages}` workflows — **DONE 2026-05-10
  (`ee99b79` philharmonic submodule + `58cf408` parent).**
  One-click "Test in chat" from `TemplateDetail`/`Templates`,
  chat tab on `InstanceDetail` with empty-content dual-
  purpose probe, runtime structural detection via
  `parseChatOutput`. Markdown rendering in bubbles
  promoted to D14 below; remaining D13 deferred follow-ups
  (instance-list dropdown for templates with many active
  chats, JP phrasing review, optional global "resume last
  chat" shortcut) listed in the archive.

Per-dispatch rationale and shape detail for the above
three preserved at
[`docs/archive/2026-05-11-roadmap-completed-arc-trim.md`](archive/2026-05-11-roadmap-completed-arc-trim.md).

Two more landed 2026-05-11 (bundled Codex r01):

- **D14** Markdown rendering in WebUI chat bubbles with
  DOMPurify hardening — **DONE 2026-05-11** (`f750b4a` +
  `c1fbff7`). `MarkdownView.tsx` with `marked` +
  `dompurify`, strict allowlist, link-target hardening
  (`target=_blank rel=noopener noreferrer nofollow`),
  `useMemo` for per-bubble efficiency. Bundle delta
  +22,480 B gzipped.
- **D15** Workflow-template `abstract_config` structured
  editor — **DONE 2026-05-11** (`f750b4a` + `c1fbff7`).
  `AbstractConfigEditor.tsx` mirrors the
  `DataConfigEditor.tsx` precedent; binding-name validation
  + retired/missing warning badges + cursor-walking
  endpoint loader. Raw-JSON `abstract_config` editor
  removed entirely. Bundle delta +828 B gzipped.

Per-dispatch rationale, sub-shape decisions, and the
verbose post-completion descriptions for D14 / D15 / D16
are at
[`docs/archive/2026-05-11-roadmap-completed-arc-trim.md`](archive/2026-05-11-roadmap-completed-arc-trim.md)
under "Evening trim — 2026-05-11".

### E. Execution-substrate runtime semantics (1 dispatch) — DONE

- **D17** `mechanics-core` response-detached background-poll
  runtime — **DONE 2026-05-12** (`mechanics-core` 0.4.0 →
  0.4.1; submodule `0e6c3e7` + parent `743e091`). The worker's
  run-job response now returns when the script's top-level
  settles; unawaited promises, endpoint calls, and
  `setTimeout` callbacks continue polling on the same worker
  tokio task until quiescence or `max_execution_time`. The
  script's `return` is the response fence; quiescence is not.

  Codex chose sub-shape B: `RunJobsExit { Complete,
  DeadlineExceeded(QueueSnapshot) }` enum + `run_jobs_until
  (predicate)` helper in `executor.rs`. `tracing = "0.1"`
  dep added; deadline-mid-tail-poll emits one structured
  `tracing::warn!` line with job ID + in-flight + queued
  counts. `setTimeout(callback, delayMs)` was added as a
  global builtin inside the script realm during D17 (the
  realm had no timer surface pre-D17); **this global will
  be reverted under D18** (2026-05-13 clarification per
  HUMANS.md's "no non-ES globals" hard rule — `setTimeout`
  is WHATWG, not ECMAScript spec; the D17 tail-promise
  polling *behavior* stays, only the user-facing
  `setTimeout` *surface* leaves). See §3.F.
  Authoritative behavior spec landed at
  [`docs/design/06-execution-substrate.md` §Tail-promise
  polling](design/06-execution-substrate.md#tail-promise-polling).
  Codex prompt archive + post-mortem at
  [`docs/codex-prompts/2026-05-12-0001-d17-mechanics-core-tail-promise-polling-01.md`](codex-prompts/2026-05-12-0001-d17-mechanics-core-tail-promise-polling-01.md);
  pre-rewrite §3.E text preserved verbatim at
  [`docs/archive/2026-05-12-roadmap-d17-done-d7-spec-d18-added.md`](archive/2026-05-12-roadmap-d17-done-d7-spec-d18-added.md).

### F. Mechanics module surface (1 dispatch)

Surfaced via HUMANS.md §"MIME module at `mechanics-core`" and
the surrounding directive on feature-gating non-endpoint
modules.

- **D18** `mechanics-core` module-surface refactor. Make every
  non-endpoint built-in module feature-gated and ship four
  new modules (one non-default, three default):

  - **Refactor**: every existing non-endpoint module
    (`mechanics:rand`, `mechanics:uuid`, `mechanics:encoding`)
    moves behind a Cargo feature flag. Pre-existing modules
    keep their previous availability by being members of
    default features.
  - **Feature `rand`** (default) — `mechanics:rand` +
    `mechanics:uuid`. Without it, `Math.random()` is seeded
    with zero (per HUMANS.md).
  - **Feature `encoding`** (default) — form-urlencoded,
    base64, base32, hex. Existing surface; gets gated.
  - **Feature `html`** (default, **new**) — wraps the
    `htmlize` crate: `htmlize::escape_text` → `escapeText`,
    `htmlize::escape_all_quotes` → `escapeAttribute`,
    `htmlize::unescape` → `unescapeText`,
    `htmlize::unescape_attribute` → `unescapeAttribute`.
  - **Feature `url`** (default, **new**) — WHATWG-compliant
    `mechanics:url`. Default export `URL`; named export
    `URLSearchParams`. Backed by the `url` crate.
  - **Feature `console`** (default, **new**) — minimal
    WHATWG-compliant `mechanics:console`. Levels: `log`,
    `info`, `warn`, `error`, `debug`. Stdout/stderr
    routing per worker config (out of scope for first
    pass — default to host-side `tracing` emission).
  - **Feature `mime`** (non-default, **new**) —
    structured MIME composer + parser at `mechanics:mime`.
    `import { compose, parse } from 'mechanics:mime'`.
    Handles Base64 and multipart cleanly; emits
    standards-compliant MIME messages. Format-only;
    does **not** know about HTML, headers semantics, or
    SMTP. Useful both standalone and as a workflow-author
    helper for the D7 `email_smtp` connector
    (workflows can keep hand-writing the `body` string
    when `mime` isn't enabled).

  Additional scope folded in 2026-05-13: **remove the
  non-ES `setTimeout` global** that D17 inadvertently added
  to the Mechanics realm. Per HUMANS.md the rule is "no
  non-ES globals" (hard rule, just reiterated 2026-05-13);
  `setTimeout` and `setInterval` are WHATWG/Web Platform
  globals, not ECMAScript spec, so neither belongs in the
  realm's global object or in any `mechanics:*` module
  export. Engine-level timer plumbing (Boa's `TimeoutJob`
  queue + the D17 tail-promise polling loop) stays — it's
  needed for spec-conformant Promise microtask handling
  and shouldn't be torn out. The fix is narrowly:
  - Drop `set_timeout` + `install_timer_builtins` from
    `mechanics-core/src/internal/runtime.rs` (D17's
    additions; ~25 lines).
  - Rewrite the two `setTimeout`-using fixtures in
    `mechanics-core/src/internal/pool/tests/runtime_behavior.rs`
    (the D17 tail-poll behavior tests) to exercise the same
    invariants via Promise-based async patterns (e.g.
    `new Promise(resolve => endpoint(...).then(resolve))`)
    so D17's tail-poll behavior remains tested without the
    `setTimeout` surface.
  - Update `docs/design/06-execution-substrate.md`
    §Tail-promise polling to drop the `setTimeout`
    callback bullet (Promise chains + endpoint calls remain
    as the only legitimate sources of tail work).
  - Update `docs/ROADMAP.md` §2 daily-log (2026-05-12 D17
    entry) and §3.E to note the `setTimeout` addition was
    reversed under D18.
  - `mechanics-core/CHANGELOG.md`: new `[0.6.0]` entry
    naming the global-removal as a breaking change
    (alongside the rest of D18's surface).
  - Add an `Outcome` addendum to the D17 prompt archive +
    the 2026-05-12 ROADMAP trim archive linking forward
    here, so future readers tracing the D17 reversal don't
    chase a phantom feature.

  Hard constraints:

  - `jsdom` won't work with Mechanics — the runtime has no
    non-ES globals on purpose. Modules expose ES-style
    `import`s only; no implicit globals. This is a **hard
    rule** (reiterated by Yuka 2026-05-13). `setTimeout`,
    `setInterval`, `requestAnimationFrame`, `queueMicrotask`,
    and anything else from the Web Platform / WHATWG global
    surface MUST NOT appear as Mechanics realm globals or
    as `mechanics:*` module exports. Engine-internal job
    queue plumbing is fine — the constraint is the user-
    facing surface, not the runtime's internals.
  - No new public-API breakage on the Rust side beyond the
    feature gates themselves and the `setTimeout`-global
    removal (existing Rust consumers stay green with default
    features on; JS workflows that called `setTimeout()`
    explicitly will fail — that's the intended behavior).
  - All modules respect Mechanics's per-job stateless
    contract — no cross-job state, no globalThis
    mutations that persist.
  - Workflow-authoring guide (en + jp) re-synced as part of
    the dispatch per HUMANS.md §"Keep the workflow authoring
    guide up-to-date" — the new modules need recipe-shaped
    documentation alongside the existing connector
    walkthroughs.

  Claude drafts the Codex prompt; Codex implements + tests.
  No crypto-review gate — runtime module surface only.
  Independent of D7-D9.

### G. HTTP-client transport + TLS trust posture (1 dispatch) — DONE

- **D20 — DONE 2026-05-13.** Built
  [`mechanics-http-client`](https://crates.io/crates/mechanics-http-client)
  (hyper-rustls + webpki-roots + aws-lc-rs; published at
  0.1.0) and migrated every workspace reqwest call site to it.
  All three release binaries (`philharmonic-api-server`,
  `mechanics-worker`, `philharmonic-connector-bin`) now have
  runtime dep trees free of `reqwest`,
  `rustls-platform-verifier`, `rustls-native-certs`, and
  `ring`; TLS provider is `aws-lc-rs` 1.16.3 and trust store
  is `webpki-roots` 1.0.7.

  Cascade version bumps shipped: `mechanics-core` 0.4.1 →
  0.5.0 (rename `ReqwestEndpointHttpClient` →
  `DefaultEndpointHttpClient`), `mechanics` 0.4.2 → 0.5.0
  (mechanics-core dep bump), `philharmonic` 0.2.0 → 0.3.0
  (mechanics + connector dep bumps),
  `philharmonic-connector-impl-http-forward` 0.1.0 → 0.2.0,
  `philharmonic-connector-impl-llm-openai-compat` 0.1.2 →
  0.2.0. Each crate's dep on mechanics-http-client is
  `"0.1"` (workspace-root `[patch.crates-io]` redirects to
  the local path for dev builds).

  Implementation: Claude direct, user override of default
  Codex-dispatch path. Pre-trim §3.G context (problem
  statement, earlier runtime-bypass plan, full scope notes,
  hard constraints): archived verbatim at
  [`docs/archive/2026-05-13-roadmap-d20-done.md`](archive/2026-05-13-roadmap-d20-done.md).
  Superseded D20 Codex prompt:
  [`2026-05-12-0002-d20-webpki-roots-only-tls-01.md`](codex-prompts/2026-05-12-0002-d20-webpki-roots-only-tls-01.md).

### H. Workspace tooling (1 dispatch) — DONE

- **D21** `scripts/pre-landing.sh` dep-aware test filtering —
  **DONE 2026-05-13** (Claude direct, not Codex; user override
  during the same-day implementation pass).
  Today the script's `--ignored` phase only runs ignored tests
  on auto-detected modified crates, but the default (non-
  ignored) test phase still runs `cargo test --workspace`
  across every member crate. After the workspace grew to ~25
  crates with substantial Boa + crypto build costs, that's
  several minutes of work per pre-landing run, most of it
  re-running tests in crates that can't possibly have been
  affected by the change. The 2026-05-12 sql-postgres
  NUMERIC-overflow fix exposed how much latent breakage hides
  behind always-paying-the-full-bill: tests in untouched crates
  pass repeatedly while the modified crate's own ignored phase
  was the first chance to catch the regression.

  Locked design (2026-05-12): default pre-landing.sh to
  **skip tests for member crates that are not dirty AND are
  not in the transitive reverse-dependency closure of any
  dirty crate**. Other phases (fmt, check, clippy, rustdoc)
  stay workspace-wide — they're cheap and the
  feature-unification surprises they catch don't respect
  modified-crate boundaries.

  Implementation sketch (your call at prompt-drafting time):

  - Compute the "dirty" set from `git status --porcelain`
    + `Cargo.toml` `[workspace] members` paths (the script
    already does this for `--ignored`).
  - Compute the reverse-dep closure via `cargo metadata` —
    starting from dirty crates, walk every member that
    transitively depends on any of them.
  - Run `cargo test -p <name>` per crate in the union of
    {dirty} ∪ {reverse-deps-of-dirty}, rather than
    `cargo test --workspace`.
  - When the workspace `Cargo.toml`, `Cargo.lock`, or any
    file under `scripts/` is dirty, fall back to the current
    workspace-wide behavior (those touch every crate's build
    universe).
  - Provide a `--full` flag to force the old behavior.

  No public API surface change. Speeds up the
  edit/pre-landing iteration loop substantially for
  single-crate changes (common case post-v1) without
  weakening the gate for workspace-wide changes.

  Claude drafts the Codex prompt; Codex implements + tests.
  No crypto-review gate.

### I. HTTP/3 client + server (1 dispatch, future session)

Captured 2026-05-13 alongside the D20 revision; explicitly
deferred to a later session.

- **D22** HTTP/3 (QUIC) support on both the outbound HTTP
  client side and the inbound HTTPS server side. Sequenced
  after D20's `mechanics-http-client` lands — the new crate
  is the natural home for the HTTP/3 client transport, and
  the alt-svc negotiation hook fits its `Client::send()`
  path. Sequenced after deployment-time feedback on the
  HTTP/2 release builds — HTTP/3 in production needs
  operational validation (firewalls, load balancers,
  observability) that doesn't exist yet for this workspace.

  **Client side** (in `mechanics-http-client`):

  - Pull in `quinn` (rustls-based QUIC) + `h3` (HTTP/3 over
    QUIC) under a non-default `http3` feature. Reuse the
    same aws-lc-rs + webpki-roots TLS configuration; UDP
    socket lifecycle is QUIC's concern.

  - **Discovery priority** (matches Firefox 84+ /
    Chrome 113+ / curl 8.10+ behavior):

    1. **HTTPS DNS RR lookup (primary).** Before opening any
       connection, query the `HTTPS` resource record (RFC 9460
       SVCB/HTTPS, DNS TYPE 65) for the origin hostname. If
       the `alpn` SvcParam advertises `h3`, attempt HTTP/3
       immediately — no HTTP/2 bootstrap round-trip required.
       Honour the `port`, `ipv4hint`, and `ipv6hint`
       SvcParams when present (the address hints skip the
       follow-up A/AAAA query). Cache per HTTPS RR TTL.
       This is the **higher-priority** discovery path, on by
       default for first-contact connections. Resolver:
       `hickory-resolver` in system-config mode (consults
       `/etc/resolv.conf`); same crate D19 introduces, so
       sequencing D19 before D22 amortises the dep
       introduction.
    2. **`Alt-Svc` response header (fallback).** When the
       HTTPS RR lookup returned NODATA, NXDOMAIN, or no `h3`
       in `alpn`, the first request goes over HTTP/2. If the
       server's response carries
       `Alt-Svc: h3=":443"; ma=N`, subsequent requests to
       that origin upgrade to HTTP/3 within the advertised
       `max-age` lifetime. Per-origin Alt-Svc cache in the
       `Client`.
    3. **HTTP/2-only fallback.** When neither HTTPS RR nor
       Alt-Svc advertises h3, or the QUIC handshake fails,
       requests stay on HTTP/2 / HTTP/1.1 for the origin
       (and the negative result is cached briefly to avoid
       per-request DNS thrash).

  - HTTPS RR caching obeys the resource record's TTL; on
    expiry the next first-contact request re-queries.
    Alt-Svc cache obeys the `ma` (max-age) directive; entries
    purge on expiry. Both caches live in the `Client`
    (cheap-to-clone `Arc` state already).
  - **Privacy/correctness note.** HTTPS RR lookup happens
    via the resolver configured in `/etc/resolv.conf`; if the
    host uses an encrypted upstream (DoH / DoT / DoQ via the
    stub) the privacy properties of the lookup are equal to
    or better than the equivalent A/AAAA query. We do **not**
    bundle our own DoH resolver in v1 — that's a separate
    decision if the workspace's deployment model later wants
    it.

  **Server side** (`mechanics`, `bins/philharmonic-api-server`,
  `bins/philharmonic-connector`):

  - Add a UDP/443 listener (alongside the existing TCP/443
    HTTPS listener) under each bin's `https` feature.
    `quinn` provides the QUIC endpoint; `h3` plus an axum-
    or tower-style adapter routes HTTP/3 requests into the
    same handler chain as HTTP/2.
  - Each bin advertises `Alt-Svc: h3=":443"; ma=86400`
    (or similar) on every HTTPS response so clients learn
    of HTTP/3 capability after the first HTTP/2 exchange.
    Alt-Svc remains the **secondary** discovery path; the
    deployment guide should recommend operators publish an
    HTTPS RR alongside their A/AAAA records (e.g.
    `example.com. IN HTTPS 1 . alpn="h3,h2"`) so
    HTTPS-RR-capable clients skip the HTTP/2 bootstrap
    round-trip on first contact. Alt-Svc covers the case
    where operators can't edit the DNS zone (managed-DNS
    providers without HTTPS-type support, etc.).
  - 0-RTT replay safety: only allow 0-RTT on idempotent
    methods (GET / HEAD) at first; explicit per-route opt-in
    for others.

  **TLS posture stays D20's**: aws-lc-rs + webpki-roots,
  Mozilla bundle only, no native-roots. ALPN must include
  `h3` alongside `h2` / `http/1.1`. HSTS continues to apply
  to HTTP/3 responses identically (RFC 6797 is
  transport-agnostic; the header lives in HTTP semantics
  layer above QUIC).

  **Hard constraints:**

  - Mechanics-Philharmonic independence stays.
    `mechanics-http-client` owns the HTTP/3 client surface.
    Philharmonic crates consume it. No Philharmonic dep on
    mhc.
  - Optional / feature-gated. HTTP/3 ships under a non-
    default `http3` Cargo feature on every affected crate.
    Operators opt in when they're ready.
  - Existing HTTP/1.1 + HTTP/2 paths unchanged. HTTP/3 is
    additive.
  - **HTTP/1.1 must keep working in every form it works in
    today, on both client and server.** HTTP/3 is QUIC-over-
    TLS by design and cannot be served over plaintext UDP —
    that's inherent, not a project choice — so the constraint
    is that *enabling the `http3` feature must not regress
    any existing HTTP/1.1 (or HTTP/2) path*. The three
    HTTP/1.1 modes that all stay supported:
    1. **Plain HTTP/1.1 over cleartext TCP** (`http://`).
       No TLS, no ALPN, no discovery; the existing `hyper`
       HTTP/1.x cleartext path stays untouched.
    2. **HTTP/1.1 over TLS** (`https://`) when the server
       only offers HTTP/1.1 — i.e. ALPN negotiation lands on
       `http/1.1` because the server doesn't advertise `h2`.
    3. **HTTP/1.1 selected via ALPN** when the server offers
       multiple protocols but the client chooses (or is
       forced down to) `http/1.1`. The client's ALPN list
       MUST continue to include `http/1.1` after D22 lands;
       adding `h3` to the advertised protocol set is
       additive, not a replacement.

    Concretely:
    - **Client.** `http://` URLs flow through the existing
      `hyper` HTTP/1.1/HTTP/2 cleartext path with **no**
      HTTPS RR lookup, **no** QUIC attempt, **no** Alt-Svc
      honouring. Discovery-priority logic only kicks in for
      `https://` origins. For `https://` origins where the
      HTTPS RR / Alt-Svc paths produce no HTTP/3 hit, or
      where the QUIC handshake fails, ALPN over the TCP+TLS
      fallback negotiates HTTP/2 or HTTP/1.1 exactly as it
      does pre-D22. The crate's URL-scheme allowlist (`http`
      and `https`, per
      [`mechanics-http-client/src/request.rs`](../mechanics-http-client/src/request.rs))
      stays open to both, and the TLS-side ALPN list stays
      `h2, http/1.1` (with `h3` added under the `http3`
      feature in the QUIC-side ALPN list — separate channel).
    - **Server.** Each release bin keeps its current shape:
      plain HTTP/1.1 over TCP is the default when the `https`
      feature is off, full stop. With `https` on but `http3`
      off: TLS-side ALPN advertises `h2, http/1.1` (current
      behaviour). With both `https` and `http3` on: TLS-side
      ALPN stays `h2, http/1.1` (clients that don't speak
      h3 over QUIC must still get HTTP/2 or HTTP/1.1 over
      TCP+TLS), and the QUIC-side ALPN is `h3`. The HTTP/3
      UDP listener is gated behind the *existing* `https`
      feature (no new umbrella feature, no implicit upgrade)
      — turning on `http3` without `https` is either a Cargo-
      feature-validation error at build time or a runtime
      no-op, whichever Codex picks at prompt-drafting time.
      A bin built without `https` MUST NOT open a UDP
      listener, MUST NOT emit `Alt-Svc`, and MUST NOT
      advertise an HTTPS RR-style capability anywhere in its
      response shape.
  - No `unsafe` blocks introduced by D22 work beyond what
    quinn / h3 require internally.

  Sequencing note: D20 (✓ landed 2026-05-13) was the
  code-side prerequisite — `mechanics-http-client` is the
  natural home for the HTTP/3 client transport. D19 (DNS
  connector, hickory-resolver) is a soft prerequisite worth
  honouring: doing D19 first introduces `hickory-resolver`
  to the workspace under one Cargo.toml, and D22 then reuses
  the dep for HTTPS RR lookup rather than adding it twice in
  the same workspace. The server side technically doesn't
  depend on either D19 or D20, but operationally it's
  cleaner to ship client + server together so a deployment
  that turns on HTTP/3 gets a coherent picture (HTTPS RR
  publishing + Alt-Svc emission + UDP listener all line up).

  Implementation owner: Claude direct or Codex dispatch
  TBD when D22 is scheduled. No crypto-review gate —
  transport-layer change only, no AAD / AEAD / SCK / COSE
  touches.

### J. Production-security dep cleanup (2 dispatches) — TOP PRIORITY

**Sequencing directive (Yuka, 2026-05-13):** these
production-security cleanups land **before any remaining
Tier 2/3 connector work (D7 SMTP, D8 Anthropic, D9 Gemini,
D19 DNS) and before D18 (mechanics-core module refactor)**.
The framing is "serious production-ready security" — the
workspace's release-binary runtime trees being clean of
non-aws-lc-rs / non-webpki-roots TLS and the per-dep
feature surface being minimised are baseline requirements,
not optional polish.

#### D23 — in-tree minimal testcontainers replacement

Captured 2026-05-13 as the cleanup-residual of the
`ring` / `rustls-platform-verifier` / `rustls-native-certs`
bans-tightening pass. After that pass (philharmonic-api
reqwest → mhc dev-dep migration, rcgen aws_lc_rs feature,
ureq `rustls-no-provider` + manual aws-lc-rs provider
install, testcontainers/bollard switched to `aws-lc-rs`
feature), exactly one wrapper remains in `deny.toml`:
`rustls-native-certs` allowed when its direct parent is
`bollard`. The path is `bollard`'s `ssl_providerless`
feature (which both `aws-lc-rs` and `ssl` build on)
unconditionally pulling `rustls-native-certs` — bollard
uses it to validate the Docker daemon's registry-side TLS
when pulling images.

Replace `testcontainers` / `testcontainers-modules`
  with a minimal in-tree dev-tooling crate
  (working name `xtask-testcontainers` or
  `mechanics-testcontainers`; bikeshed at prompt-drafting
  time) that uses `bollard` with **only the features the
  workspace's integration tests actually need**, dropping
  `ssl_previderless` / `home` / `rustls-native-certs`
  entirely.

  **What workspace tests use today** (audit before drafting
  the prompt; this is the baseline):
  - Start a MySQL container (`mysql:8` or similar public
    image) with a healthcheck-style wait.
  - Start a Postgres container with the same pattern.
  - Inspect host + assigned port for the connection string.
  - Tear down on `Drop`.
  - All over a local Docker daemon Unix socket (the
    workspace doesn't currently use remote-daemon test
    setups).

  **What bollard needs for that subset:**
  - `unix-socket` feature (talk to local Docker daemon).
  - Image-pull / container-start / container-inspect /
    container-stop API calls.
  - **No TLS**: registry-side TLS is the daemon's concern,
    not bollard's, when the workspace talks to the daemon
    over a Unix socket.
  - **No `home`**: workspace tests use public images that
    don't need Docker Hub auth from `~/.docker/config.json`.

  **Hard requirements:**
  - Public surface mirrors what the workspace's test code
    actually uses today (a `Container<Image>` handle with
    `.start().await`, `.get_host_port_ipv4(...)`,
    `.get_host()`, `.with_startup_timeout(...)`, Drop-based
    teardown). Migration sites are mostly mechanical.
  - The MySQL and Postgres "image" types from
    `testcontainers-modules` get re-implemented as
    minimal wrappers (image name + env vars + ready
    probe).
  - Dev-tooling crate, never published. `publish = false`.
  - In-tree (non-submodule) member, mirroring the existing
    `xtask/` placement convention.
  - No `unsafe`, no panics on reachable paths in lib code
    (test fixtures themselves may `.expect()` per the
    workspace's test conventions).

  **Migration scope** (~6 consumer crates):
  - `philharmonic-api/tests/{e2e_mysql.rs, e2e_full_pipeline.rs}`
  - `philharmonic-policy/tests/...`
  - `philharmonic-workflow/tests/...`
  - `philharmonic-store-sqlx-mysql/tests/...`
  - `philharmonic-connector-impl-sql-mysql/tests/...`
  - `philharmonic-connector-impl-sql-postgres/tests/...`

  Each migrates from `testcontainers` / `testcontainers-modules`
  imports to the in-tree replacement. Existing test logic
  unchanged.

  **Concurrency-limit knob** (Yuka, 2026-05-13): today the
  workspace's testcontainer tests are file-lock-serialized
  via `serial_test` (see comments in
  `philharmonic-connector-impl-sql-{mysql,postgres}/Cargo.toml`)
  because spinning up many containers concurrently used to
  OOM the dev box. The current host is more capable —
  the new default is **`min(4, available_parallelism / 4)`**
  concurrent testcontainer tests (computed via
  `std::thread::available_parallelism()`). On a 16-core
  box that's 4; on an 8-core box, 2; on a 4-core box, 1;
  on resource-tight CI runners, the floor of 1 falls back
  to the current serial behaviour automatically. Implement
  via either a semaphore-style file-based limiter (so the
  limit applies across cargo's per-test-binary process
  fan-out) or a concurrency knob on the replacement
  crate's fixture initialiser. Wire the formula in one
  place in the replacement crate (a `pub const` or a
  `LazyLock<usize>`) so future tuning is one-edit. Verify
  by running the SQL connector test suites under a load
  monitor (`./scripts/build-status.sh` or the
  resource-pressure xtask) at the computed concurrency
  and confirming the dev box stays healthy; the formula
  errs on the conservative side.

  **Acceptance:** after D23, the `rustls-native-certs`
  entry in `deny.toml` becomes a no-wrapper full ban
  (matching `native-tls`, `rustls-platform-verifier`).
  `cargo tree --workspace --invert rustls-native-certs`
  prints nothing.

  Claude drafts the Codex prompt; Codex implements + tests
  + migrates consumers. No crypto-review gate — dev-tooling
  only, no AAD / AEAD / SCK / COSE touches.

  **Why not fork bollard's features instead?** Bollard's
  Cargo.toml entangles `home` / `ssl_providerless` /
  `rustls-native-certs` such that disabling them from a
  consumer's feature flags isn't possible without a fork.
  The in-tree wrapper approach is structurally cleaner —
  one small crate that depends on bollard with the precise
  minimum features the workspace needs, instead of a
  bollard fork that has to be kept in sync.

#### D24 — workspace-wide `default-features = false` audit

Captured 2026-05-13. Production-security driver: each
direct dep's default-feature set is whatever the upstream
maintainer ships, often broader than what the workspace
actually uses. Untouched default features (a) inflate
compile time and binary size, (b) expand the supply-chain
attack surface unnecessarily (each pulled crate is one
more compromise vector), (c) sometimes pull crypto
backends or HTTP clients we don't want — the `ring` /
`native-tls` / `rustls-platform-verifier` chains found
during the D23 bans pass were specifically default-feature
leaks that the workspace's runtime intent didn't authorise.

The audit walks every workspace crate's `Cargo.toml`,
direct dep by direct dep, and:

- Sets `default-features = false` on every dep where the
  workspace's usage doesn't need the upstream's defaults.
- Enumerates the explicit feature list the crate actually
  uses, picked from reading the crate's own `src/` calls.
- Applies the same discipline to internal workspace-deps
  (philharmonic-* / mechanics-* using each other) — when
  crate A depends on crate B, it pins `default-features =
  false` and only enables the B features it actually
  needs. This is the cross-crate piece Yuka called out
  explicitly: "we'll trim … with default-features = false
  for our own crates too."

Scope estimate: 25+ workspace Cargo.tomls, ~150-200 dep
entries to audit (per-Cargo.toml ranges from ~5 to ~25
direct deps). Mechanical-but-thorough; the per-dep
question is always "which features does this crate's src/
actually touch", which is grep-able. Codex's bread and
butter.

**Hard requirements:**

- Workspace `cargo build --workspace` stays green
  end-to-end after each crate's audit (no missing features
  cascade).
- `./scripts/pre-landing.sh` clean after the whole pass.
- Test paths preserved: `#[cfg(test)]` features added
  where dev-deps need them.
- Per-crate version-bump policy: this is a behaviour-
  preserving refactor for the workspace's published
  crates. Patch-version-bump on any crate that lands
  Cargo.toml changes (e.g. mhc 0.2.0 → 0.2.1 if its deps
  shifted; ditto every other touched published crate).
- For any dep where trimming exposed a real previously-
  hidden behaviour change (rare; called out in residual
  risks), the Codex prompt may opt to leave defaults on
  and document. The bias is toward trimming.

**Acceptance:**

- Every workspace crate's direct deps either have
  `default-features = false` with an explicit feature list
  *or* an inline comment explaining why defaults are kept
  (e.g. "axum's default feature set is the cheapest path
  for our usage; trimming explored and not worth").
- `cargo tree --workspace -e all --duplicates` shows no
  new duplicates introduced by the audit (feature
  unification can sometimes worsen with trimming; document
  any case where it does and accept).
- Compile-time wins documented per published crate as a
  CHANGELOG patch entry.

Claude drafts the Codex prompt; Codex implements; Claude
reviews + commits. No crypto-review gate — Cargo.toml
edits + CHANGELOG entries only. Lands **after D23** (so
the bans wrapper churn isn't doubled up) and **before
D7 / D8 / D9 / D18 / D19** per the §J sequencing
directive.

### Suggested sequencing

**Completed work (2026-05-02 through 2026-05-11):** D1 +
D2 + D10 → Gate 1 → embedding-datasets feature end-to-end
(D3 r01 → r02 → D4+D5+caps+409 → Gate 2 → D6 WebUI) → D12
→ D13 → D11 (+ JP mirror) → D16 → D14 + D15. Per-step
commit SHAs and per-dispatch shape detail preserved at
[`docs/archive/2026-05-11-roadmap-completed-arc-trim.md`](archive/2026-05-11-roadmap-completed-arc-trim.md).
Additional 2026-05-11 deployment-time polish (not
numbered Codex dispatches — mechanics-core 0.4.0,
philharmonic-api 0.1.8, WebUI permission-aware UI,
assistant `name` field surfacing, connector wire-shape
guide expansion, audit-log producer gap closed)
summarised in the Current state preamble at the top of
this file with the same archive pointer.

**Next dispatchable**: D7 / D8 / D9 / D18 / D19 / D20, all
six independent and parallel-safe. D21 landed 2026-05-13.
D22 (HTTP/3) sequenced after D20 lands (`mechanics-http-client`
is the natural client-side home) and after deployment-time
HTTP/2 validation feedback — explicit later-session work.

- **D7** is unblocked — the `email_send` wire shape locked
  in [`docs/design/08-connector-architecture.md` §SMTP](design/08-connector-architecture.md#smtp)
  on 2026-05-12 via HUMANS.md. Claude can draft the
  Codex prompt directly from §SMTP.
- **D8** is fully spec'd from
  [`docs/design/08-connector-architecture.md` §llm_anthropic](design/08-connector-architecture.md#llm_anthropic--config);
  ready for prompt draft.
- **D9** carries the dual-mode AI Studio + Vertex AI
  requirement; Claude proposes the discriminator field,
  Vertex-mode field names, and OAuth2 access-token caching
  strategy in the prompt; Yuka overrides at prompt-review
  time if she has a preference.
- **D18** (`mechanics-core` module-surface refactor) is
  fully spec'd from §3.F above; ready for prompt draft.
- **D19** (DNS connector) is fully spec'd from
  [`docs/design/08-connector-architecture.md` §DNS](design/08-connector-architecture.md#dns),
  and setup-unblocked 2026-05-12 (submodule wired, crates.io
  `0.0.0` placeholder published). Ready for prompt draft.
- **D20** (webpki-roots-only TLS trust posture) is fully
  spec'd from §3.G above; ready for prompt draft.

**D17** (execution-substrate tail-promise polling) landed
2026-05-12; no further work in this arc.

### Dispatch discipline reminder

Per [`.claude/skills/codex-prompt-archive/SKILL.md`](../.claude/skills/codex-prompt-archive/SKILL.md)
and [`CONTRIBUTING.md §15.2`](../CONTRIBUTING.md#152-codex-prompt-archive):
every Codex prompt is archived to
`docs/codex-prompts/YYYY-MM-DD-NNNN-<slug>.md` and committed
**before** the spawn. Each dispatch above corresponds to one
archived prompt; multi-round work shares the daily-sequence
`NNNN` and increments the trailing `-NN`.
