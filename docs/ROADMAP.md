# Philharmonic Implementation Roadmap

**Audience**: Claude Code, working in the `philharmonic-workspace`
repo.

**Purpose**: The authoritative implementation plan for Philharmonic.
V1 is complete through the first working end-to-end deployment;
active work now lives in the post-v1 dispatch plan (§3 below).

**Current state** (2026-05-10):

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
  combined RAG). JP mirror regeneration is a Claude
  follow-up.
- Yuka was on Golden Week 2026-04-29 → 2026-05-06 plus a
  personal vacation 2026-05-07 / 05-08; first regular working
  day back is Mon 2026-05-11.

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

Total: **13 Codex dispatches plus 1 Gate-1 proposal.**
**D1, D2, D3, D4, D5, D6, D10, D11, D12, D13 are done**
(10 of 13). Gate 1 and Gate 2 both approved. Remaining:
D7, D8, D9.

### A. Embedding datasets (6 dispatches + 1 Gate-1)

Authoritative design:
[`docs/design/16-embedding-datasets.md`](design/16-embedding-datasets.md).

- **(Gate 1)** Lowerer ephemeral support — **APPROVED
  2026-05-10** (`0772184` after self-review revision
  `81936f2`). Approach B chosen: synthesized non-persisted
  `EntityId<WorkflowInstance>` per embed job, no public-trait
  change.
- **(Gate 2)** Implementation review on the embed-job
  dispatcher — **APPROVED 2026-05-10** (`354e82d`) after
  Codex pre-review surfaced 3 findings, all addressed in
  `e845101`; deferred items (HTTP-response-size cap +
  duplicate-/unknown-ID rejection + parse-fn unit tests)
  closed in `1a6b4c8`.
- **D1** Substrate `MEDIUMBLOB → LONGBLOB` migration in
  `philharmonic-store-sqlx-mysql`. **DONE 2026-05-02 (`ee2bd61`).**
- **D2** `mechanics-core`: optional `MechanicsJob.run_timeout`
  override. **DONE 2026-05-02 (`ee2bd61`).**
- **D3** Embedding-datasets backend (split at dispatch into
  two rounds): `EmbeddingDataset` entity + permission atoms +
  deterministic-CBOR codec in `philharmonic-policy` +
  `WorkflowTemplate.data_config` slot in `philharmonic-workflow`
  (round 01); workflow-engine `data` assembly in `execute_step`
  + 7 API CRUD/read routes + template `data_config`
  request/response (round 02). **DONE 2026-05-10**: round 01
  `bbc26f9`, round 02 `b134d44`.
- **D4** Lowerer ephemeral support per Approach B — touches
  the API server lowerer only (no public-trait change to
  `philharmonic-workflow`). **DONE 2026-05-10** (fused with
  D5 in `e37f956`).
- **D5** Ephemeral embed job: built-in JS embed script
  (Codex-authored, compiled into the API binary via
  `include_str!`) plus the background tokio task in
  `philharmonic-api-server` that lowers the embed endpoint,
  dispatches the mechanics job, and appends `Ready` / `Failed`
  revisions. Includes round-02 follow-ups: `EmbedDatasetCaps`
  wired through `ApiConfig` and the new `ApiError::Conflict`
  variant for 409-on-Embedding. **DONE 2026-05-10** (`e37f956`,
  with Gate-2 hardening in `e845101` + `1a6b4c8`).
- **D6** Embedding-datasets WebUI: structured-table source-
  items editor, CSV/JSON bulk-import modal, collapsed-by-
  default corpus vector view, polling refresh, i18n
  (en/ja). **DONE 2026-05-10** (`b581b50`). The
  `permissions.ts` follow-up to register the four
  `embed_dataset:*` atoms (Codex flagged in residuals,
  Claude patched) is in the same commit.

### B. Phase 7 Tier 2/3 connector implementations (3 dispatches)

Each is one substantive crate going from `0.0.x` placeholder to
`0.1.0` substantive implementation. None of these touch the
crypto path; the connector-service framework already validates
tokens and decrypts payloads — implementations only need to
implement the `Implementation` trait.

- **D7** `philharmonic-connector-impl-email-smtp` (Tier 2).
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

Independent of one another and of section A; safe to run in
parallel.

### C. Connector enhancements (1 dispatch)

- **D12** `philharmonic-connector-impl-llm-openai-compat`:
  add a `custom_headers: BTreeMap<String, String>` knob to
  the runtime endpoint config so deployments can attach
  provider-specific HTTP headers to upstream calls. Driven by
  Hugging Face Inference's `X-HF-Bill-To` (org billing); also
  covers OpenAI's `OpenAI-Organization` / `OpenAI-Project`,
  OpenRouter's `HTTP-Referer` / `X-Title`, and similar
  per-provider knobs across the OpenAI-compatible ecosystem.
  `BTreeMap` (not `HashMap`) for deterministic-fixture
  comparisons + sorted serialised keys matching the
  workspace's canonical-JSON / deterministic-CBOR discipline.
  **DONE 2026-05-10 (`2fff3bb`).**

  The field belongs to the **runtime endpoint config** — i.e.
  the impl-side decrypted-config struct in
  `philharmonic-connector-impl-llm-openai-compat/src/config.rs`,
  which rides inside the existing SCK-encrypted blob on
  `TenantEndpointConfig`. `#[serde(default)]` keeps existing
  configs valid (back-compat). The impl applies the headers
  to its outbound reqwest builder before sending; no
  primitive, AAD, or signed-claim change.

  Reserved headers (`authorization`, `content-type`,
  `content-length`, `host`, `transfer-encoding`,
  `connection`, plus CRLF-injection guards on values) are
  rejected at config-validation time rather than at request
  time, so a bad config is caught at endpoint-config write.

  Touches `philharmonic-connector-impl-llm-openai-compat`
  only — no public-trait change, no other crate edits, no
  crypto path touched. Bump version + CHANGELOG. Tests:
  header pass-through to the upstream request, reserved-
  header rejection, CRLF rejection. WebUI gets no special
  treatment — endpoint configs are JSON-edited through the
  existing CodeMirror 6 editor (D10) which accepts the new
  field naturally.

  Independent of everything else; small. **Lands before
  D7/D8/D9** — production deployments hitting Hugging Face
  Inference need the `X-HF-Bill-To` header now to bill an
  organisation rather than the personal account, and the
  fix is single-crate / single-config-field-sized. The
  Tier 2/3 implementations (Anthropic / Gemini / SMTP) are
  larger and don't unblock anything for HF users.

### D. WebUI infrastructure, features, and docs (3 dispatches)

- **D10** CodeMirror 6 in the WebUI. **DONE 2026-05-02
  (`ee2bd61`).**
- **D11** Workflow authoring guide rewrite (English).
  **DONE 2026-05-10** (`10acd7f`). 530 → 1350 lines
  reflecting current implementation reality post-D3/D4/
  D5/D6/D12/D13. Three load-bearing recipes per Yuka's
  focus directive: D13-compat chat workflow (state-driven
  accumulator), embedding-datasets workflow (five
  availability states), combined chat + RAG. All three
  copy-pasteable end-to-end with verbatim script + endpoint
  + template JSON + WebUI behavior tables + per-recipe
  permission lists. Wire-shape accuracy grep-verified
  against `philharmonic-connector-impl-{vector-search,
  embed,llm-openai-compat}/src/`,
  `philharmonic-workflow/src/engine.rs build_script_data`,
  `philharmonic/webui/src/api/client.ts ChatMessage`.
  Tier 2/3 connectors flagged as reserved/pending rather
  than fabricated. Codex flagged design-doc divergences
  for follow-up: design/07 still shows pre-D3 4-field
  script-arg shape; design/10 doesn't list `data_config`
  in template body docs. The Japanese mirror in
  [`docs-jp/ワークフロー作成ガイド.md`](../docs-jp/)
  is **not** a Codex dispatch — `docs-jp/README.md` reserves
  that submodule to Claude Code. Claude regenerates the JP
  guide as a follow-up.
- **D13** Chat-style testing UI in `philharmonic/webui` for
  workflows that accept `{"content": "<user_input>"}` as
  input and return `{"messages": [<turns>]}` as output
  (OpenAI-style chat-completion turn shape). **DONE
  2026-05-10** (`ee99b79` philharmonic submodule + `58cf408`
  parent). Six surfaces landed end-to-end on Codex's first
  attempt: types + `parseChatOutput` runtime structural
  detector in `api/client.ts`; chat tab on `InstanceDetail`
  with `?tab=chat` URL hook; "Test in chat" actions on
  `TemplateDetail` (with last-used-instance shortcut) and
  `Templates` list rows; chat UI with bubbles, autoscroll,
  send-on-Enter, in-flight indicator, error-toast on
  transport failures; `util/chatStorage.ts` localStorage
  helpers (last-used instance per template, scroll
  position); `chat.*` i18n namespace in en/ja. The
  empty-content POST (`{}`) dual-purpose semantics are
  delegated to the workflow's JS — UI always probes on
  first chat-tab mount; server-side script generates a
  greeting on empty context, returns the existing
  transcript otherwise. No backend changes; reuses
  `workflow:instance_create` + `workflow:instance_execute`.
  Bundle delta ~+3.0 KiB gzipped. Open follow-ups
  (deferred): markdown rendering in chat bubbles; full
  instance-list dropdown for templates with many active
  chats; JP phrasing review; optional global "resume last
  chat" shortcut.

### Suggested sequencing

1. **D1, D2, D10** — DONE 2026-05-02 (`ee2bd61`).
2. **Gate 1** — APPROVED 2026-05-10 (`0772184`).
3. **Embedding-datasets feature** — DONE 2026-05-10
   (end-to-end). D3 r01 (`bbc26f9`) → D3 r02 (`b134d44`) →
   D4+D5+caps+409 (`e37f956`) → Gate 2 fix (`e845101`) →
   Gate-2 deferred cleanup (`1a6b4c8`) → D6 WebUI
   (`b581b50`). Gate 2 approved (`354e82d`).
4. **D12** custom-headers knob — DONE 2026-05-10
   (`2fff3bb`).
5. **D13** chat-style testing UI — DONE 2026-05-10
   (`ee99b79` + `58cf408`).
6. **D11** workflow authoring guide rewrite — DONE
   2026-05-10 (`10acd7f`). JP mirror regeneration is a
   Claude follow-up.
7. **Next dispatchable**: D7 / D8 / D9 (Tier 2/3
   connectors — SMTP, Anthropic, Gemini); D9 carries
   the dual-mode AI Studio + Vertex AI requirement. All
   three are independent and parallel-safe.

### Dispatch discipline reminder

Per [`.claude/skills/codex-prompt-archive/SKILL.md`](../.claude/skills/codex-prompt-archive/SKILL.md)
and [`CONTRIBUTING.md §15.2`](../CONTRIBUTING.md#152-codex-prompt-archive):
every Codex prompt is archived to
`docs/codex-prompts/YYYY-MM-DD-NNNN-<slug>.md` and committed
**before** the spawn. Each dispatch above corresponds to one
archived prompt; multi-round work shares the daily-sequence
`NNNN` and increments the trailing `-NN`.
