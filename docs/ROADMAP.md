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
    top-level settles; unawaited promises, endpoint
    calls, and `setTimeout` callbacks continue polling on
    the worker tokio task until quiescence or
    `max_execution_time`. Authoritative behavior spec
    at [`design/06` §Tail-promise polling](design/06-execution-substrate.md#tail-promise-polling).
    Codex chose sub-shape B (`RunJobsExit` enum +
    `run_jobs_until(predicate)` helper); `tracing = "0.1"`
    dep + `setTimeout` global builtin added.
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

Total: **20 Codex dispatches plus 1 Gate-1 proposal.**
**D1, D2, D3, D4, D5, D6, D10, D11, D12, D13, D14, D15, D16,
D17 are done** (14 of 20). Gate 1 and Gate 2 both approved.
Remaining: D7, D8, D9, D19 (Tier 2/3 connectors — D19 is
the new DNS connector surfaced 2026-05-12 via HUMANS.md), D18
(`mechanics-core` module-surface refactor: feature gating +
new `mime`/`url`/`console`/`html` modules), D20 (workspace-wide
webpki-roots-only TLS trust posture surfaced 2026-05-12 — sqlx
already aws-lc-rs+webpki-roots after the ring removal, reqwest
still picks up platform-native via rustls-platform-verifier).

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
  counts. `setTimeout(callback, delayMs)` added as a global
  builtin inside the script realm (the realm had no timer
  surface pre-D17). Authoritative behavior spec landed at
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

  Hard constraints:

  - `jsdom` won't work with Mechanics — the runtime has no
    non-ES globals on purpose. Modules expose ES-style
    `import`s only; no implicit globals.
  - No new public-API breakage on the Rust side beyond the
    feature gates themselves (existing consumers stay
    green with default features on).
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

### G. TLS trust posture (1 dispatch)

Surfaced 2026-05-12 after the ring-removal work (commit
`18f1bb2`): the workspace's TLS trust-store posture is
inconsistent. sqlx (Postgres/MySQL connectors,
philharmonic-api's MySQL substrate store) verifies against
the bundled Mozilla CA bundle via `webpki-roots`; reqwest
(every outbound HTTP path — mechanics-core's endpoint client,
http_forward, llm_openai_compat, and the upcoming Tier 3 LLM
connectors) verifies against the OS-native trust store via
`rustls-platform-verifier` + `rustls-native-certs`. Operator
consequences: a tenant-installed corporate CA gets picked up
for HTTP outbound but not for SQL outbound; air-gapped
environments need different mitigations for each path; the
HTTP trust set drifts on OS package updates while SQL trust
is frozen at compile time.

- **D20** Force `webpki-roots` everywhere; remove the
  reqwest → `rustls-platform-verifier` / `rustls-native-certs`
  path. Locked design choice (2026-05-12, Yuka): the bundled
  Mozilla bundle is the workspace's single source of CA truth.
  No native-roots fallback.

  reqwest 0.13.3 does not have a `webpki-roots`-only feature
  variant — its `rustls` feature unconditionally enables
  `rustls-platform-verifier` as a dep. The cleanest in-workspace
  fix is `reqwest::ClientBuilder::use_preconfigured_tls()` with
  a `rustls::ClientConfig` whose `RootCertStore` is populated
  from `webpki_roots::TLS_SERVER_ROOTS`. The
  `rustls-platform-verifier` crate stays in the dep tree
  (binary weight cost) but is never invoked at runtime if every
  reqwest client construction routes through the helper.

  Touched call sites (4 production + 4 test, grep
  `reqwest::Client(::|<)|reqwest::Client::new|Client::builder\(\)`):

  - `mechanics-core/src/internal/pool/api.rs:119` (production)
  - `bins/philharmonic-api-server/src/executor.rs:24` (production)
  - `philharmonic-connector-impl-http-forward/src/client.rs:18`
    (production)
  - `philharmonic-connector-impl-llm-openai-compat/src/client.rs:15`
    (production)
  - `mechanics-core/src/internal/pool/tests/{mod,queue,lifecycle}.rs`
    (4 sites; test-only, but update for consistency)

  Per-crate Cargo.toml additions: `rustls = "0.23"` (already in
  the tree transitively at 0.23.40) and `webpki-roots = "1"`
  (already at 1.0.7). No feature unification surprises — the
  existing `aws-lc-rs` provider feature stays enabled.

  Codex picks: helper-function-per-crate (inline 5-6 line
  factory) vs. shared crate. The `philharmonic-connector-common`
  crate is one natural home for the connector-impl sites but
  has no current HTTP-client responsibilities; mechanics-core
  is on the wrong side of the Mechanics-Philharmonic
  independence boundary to be a workspace-wide helper home.
  Per-crate inline is the safest default; the dispatch may
  choose otherwise.

  Version bumps: patch bumps on every published crate the
  dispatch touches (`mechanics-core` 0.4.1 → 0.4.2,
  `philharmonic-connector-impl-http-forward` 0.1.0 → 0.1.1,
  `philharmonic-connector-impl-llm-openai-compat` 0.1.2 →
  0.1.3). `bins/philharmonic-api-server` is unpublished so
  no bump. Pre-1.0 SemVer permits patch bump for an internal
  behavior change of this scope.

  Hard constraints:

  - No native-roots fallback. Even `rustls-native-certs` /
    `openssl-probe` should drop out of the runtime tree where
    possible (they may remain as transitive deps of
    `rustls-platform-verifier` which itself stays compiled —
    that's acceptable as long as the runtime control flow
    never touches them).
  - aws-lc-rs stays as the sole crypto provider (no
    re-introduction of `ring`).
  - No HTTP wire format change. No public API surface change.

  Claude drafts the Codex prompt; Codex implements + tests.
  No crypto-review gate — trust-store config only, no
  changes to AAD/AEAD/SCK/COSE paths.

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
six independent and parallel-safe.

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
