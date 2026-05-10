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
- Gate-1 proposal for embedding-datasets ephemeral lowering:
  **APPROVED 2026-05-10** (Approach B — synthesized non-persisted
  `EntityId<WorkflowInstance>`, no public-trait change, no
  crypto-shape change). Approval at
  [`docs/crypto/approvals/2026-05-04-post-v1-embed-dataset-lowerer-ephemeral.md`](crypto/approvals/2026-05-04-post-v1-embed-dataset-lowerer-ephemeral.md);
  proposal at
  [`docs/crypto/proposals/2026-05-04-post-v1-embed-dataset-lowerer-ephemeral.md`](crypto/proposals/2026-05-04-post-v1-embed-dataset-lowerer-ephemeral.md).
  **D4 and D5 unblocked.**
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

Total: **12 Codex dispatches plus 1 Gate-1 proposal.** D1, D2,
D10 are done; Gate 1 is approved.

### A. Embedding datasets (6 dispatches + 1 Gate-1)

Authoritative design:
[`docs/design/16-embedding-datasets.md`](design/16-embedding-datasets.md).

- **(Gate 1)** Lowerer ephemeral support — **APPROVED
  2026-05-10**, Approach B (synthesized non-persisted
  `EntityId<WorkflowInstance>` per embed job, no public-trait
  change). D4 and D5 were gated on this; both now unblocked.
- **D1** Substrate `MEDIUMBLOB → LONGBLOB` migration in
  `philharmonic-store-sqlx-mysql`. **DONE 2026-05-02 (`ee2bd61`).**
- **D2** `mechanics-core`: optional `MechanicsJob.run_timeout`
  override. **DONE 2026-05-02 (`ee2bd61`).**
- **D3** Embedding-datasets backend:
  - `EmbeddingDataset` entity + scalar/content slots in
    `philharmonic-policy`.
  - Permission atoms (`embed_dataset:create|read|update|retire`).
  - API CRUD endpoints + source-items + corpus endpoints in
    `philharmonic-api`.
  - `WorkflowTemplate.data_config` content slot + API validation.
  - Workflow engine `data` assembly in `execute_step`
    (`philharmonic-workflow`).

  Cross-crate but cohesive feature surface; one dispatch.
  Independent of Gate 1. If Codex hits scope limits, split into
  "policy entity + atoms" round-01 and "API + workflow data
  assembly" round-02.
- **D4** Lowerer ephemeral support per Approach B — touches the
  API server lowerer only (no public-trait change to
  `philharmonic-workflow`). **Unblocked.**
- **D5** Ephemeral embed job: built-in JS embed script (Codex-
  authored, compiled into the API binary as a static string)
  plus the background tokio task in `philharmonic-api-server`
  that lowers the embed endpoint, dispatches the mechanics job,
  and appends `Ready` / `Failed` revisions. **Gated on D4.**
- **D6** Embedding-datasets WebUI: structured table editor for
  source items, Import modal for CSV/JSON bulk import,
  collapsed-by-default vector view, i18n for `en.ts` / `ja.ts`.
  Depends on D3's API endpoints; can run in parallel with D4/D5.
  Per the HUMANS.md erratum, **no persistent raw-JSON view of
  the dataset itself**.

### B. Phase 7 Tier 2/3 connector implementations (3 dispatches)

Each is one substantive crate going from `0.0.x` placeholder to
`0.1.0` substantive implementation. None of these touch the
crypto path; the connector-service framework already validates
tokens and decrypts payloads — implementations only need to
implement the `Implementation` trait.

- **D7** `philharmonic-connector-impl-email-smtp` (Tier 2).
- **D8** `philharmonic-connector-impl-llm-anthropic` (Tier 3).
- **D9** `philharmonic-connector-impl-llm-gemini` (Tier 3).

Independent of one another and of section A; safe to run in
parallel.

### C. Connector enhancements (1 dispatch)

- **D12** `philharmonic-connector-impl-llm-openai-compat`:
  add a `custom_headers: HashMap<String, String>` knob to the
  runtime endpoint config so deployments can attach
  provider-specific HTTP headers to upstream calls. Driven by
  Hugging Face Inference's `X-HF-Bill-To` (org billing); also
  covers OpenAI's `OpenAI-Organization` / `OpenAI-Project`,
  OpenRouter's `HTTP-Referer` / `X-Title`, and similar
  per-provider knobs across the OpenAI-compatible ecosystem.

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

### D. WebUI infrastructure and docs (2 dispatches)

- **D10** CodeMirror 6 in the WebUI. **DONE 2026-05-02
  (`ee2bd61`).**
- **D11** Workflow authoring guide rewrite (English). Codex
  re-reads the design docs + connector-architecture spec, then
  rewrites
  [`docs/guide/workflow-authoring.md`](guide/workflow-authoring.md)
  from scratch to reflect current implementation reality. The
  Japanese mirror in [`docs-jp/ワークフロー作成ガイド.md`](../docs-jp/)
  is **not** a Codex dispatch — `docs-jp/README.md` reserves
  that submodule to Claude Code. Claude regenerates the JP
  guide after D11 lands.

### Suggested sequencing

1. **D1, D2, D10** — DONE 2026-05-02 (`ee2bd61`).
2. **Gate 1** — APPROVED 2026-05-10 (`0772184`, after Claude's
   self-review revision `81936f2`). Approach B chosen.
3. **Embedding datasets feature**: D3 → D4 → D5; D6 in parallel
   after D3. With Gate 1 now cleared, the full chain is
   dispatchable as bandwidth allows.
4. **D12 first** (small, unblocks production HF Inference
   org-billing now): custom-headers knob on the existing
   `llm-openai-compat` impl.
5. **Tier 2/3 connectors after D12**: D7 / D8 / D9 — Anthropic,
   Gemini, SMTP. Independent of one another and of section A;
   safe to run in parallel after D12 lands.
6. **Anytime**: D11 (independent of everything else).

### Dispatch discipline reminder

Per [`.claude/skills/codex-prompt-archive/SKILL.md`](../.claude/skills/codex-prompt-archive/SKILL.md)
and [`CONTRIBUTING.md §15.2`](../CONTRIBUTING.md#152-codex-prompt-archive):
every Codex prompt is archived to
`docs/codex-prompts/YYYY-MM-DD-NNNN-<slug>.md` and committed
**before** the spawn. Each dispatch above corresponds to one
archived prompt; multi-round work shares the daily-sequence
`NNNN` and increments the trailing `-NN`.
