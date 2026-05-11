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
  combined RAG).
- **D11 follow-ups** all shipped 2026-05-10:
  - **JP mirror** of the workflow authoring guide
    regenerated to match the new English version
    (`docs-jp/ワークフロー作成ガイド.md`, 406 → 1368
    lines; `e159e88` docs-jp + `6913a9d` parent).
  - **WebUI template-form `data_config` editor** —
    structured embedding-dataset binding editor on
    Create + Edit forms with binding-name validation,
    retired/missing warning badges, friendly-UI per
    HUMANS.md (Codex r01 `f040dce` philharmonic +
    `db9f737` parent).
  - **Design-doc reconciliation** — `design/07`
    script-arg shape five-field `{context, args,
    input, subject, data}` with full `data` semantics;
    `design/10` template body + PATCH semantics
    extended with `data_config` (`4b6a122`).
- **Late-Sunday fix-its (2026-05-10 evening)**:
  - `scripts/build-status.sh` extended to detect
    running `build-script-build` executables (previously
    reported "no active Rust build processes" when a
    build.rs was the only thing running; `86c7312`).
  - Workflow authoring guide (en + jp) now flags the
    WebUI config-paste UX trap — `display_name` /
    `implementation` go in form fields; only the inner
    `config` value goes in the Config JSON editor
    (`48fe697`).
  - **Connector-path body cap raised 2 MiB → 32 MiB**
    (`philharmonic-connector-router` 0.1.1 → 0.1.2;
    `85e2ad8`). The previous 2 MiB axum default rejected
    `vector_search` corpus bodies over ~170 items at
    1024-dim with an HTTP 413 propagated up as a generic
    internal-error envelope. No crypto-shape change.
- **New follow-up tasks 2026-05-11** per
  [`HUMANS.md` §"Follow-up tasks from 2026-05-10 work"](../HUMANS.md):
  D14 (markdown rendering in chat with DOMPurify
  hardening), D15 (workflow-template `abstract_config`
  structured editor — pull-down endpoint selector), D16
  (`tool_choice: "auto"` option for
  `llm_openai_compat`'s `tool_call_fallback` dialect).
  See §3.C and §3.D below for the full entries.
- Yuka was on Golden Week 2026-04-29 → 2026-05-06 plus a
  personal vacation 2026-05-07 / 05-08; first regular working
  day back is Mon 2026-05-11. Real deployment-time
  testing begins this week.

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

Total: **16 Codex dispatches plus 1 Gate-1 proposal.**
**D1, D2, D3, D4, D5, D6, D10, D11, D12, D13 are done**
(10 of 16). Gate 1 and Gate 2 both approved. Remaining:
D7, D8, D9 (Tier 2/3 connectors), plus D14, D15, D16
added 2026-05-11 from
[`HUMANS.md` §"Follow-up tasks from 2026-05-10 work"](../HUMANS.md).

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

- **D16** `philharmonic-connector-impl-llm-openai-compat`:
  add a new dialect variant `tool_call_fallback_auto`
  alongside the existing `tool_call_fallback`. Shipping
  the auto variant rather than a per-request flag on the
  forced variant keeps the dialect enum a clean
  discriminator (decision locked 2026-05-11: option (a)
  per [`HUMANS.md` §"Follow-up tasks from 2026-05-10 work"](../HUMANS.md)
  + this conversation). The new variant carries the same
  tools-array shape but sends `tool_choice: "auto"`
  instead of the forced
  `{type: "function", function: {name: "emit_output"}}`.
  Some OpenAI-compatible inference providers — notably
  some local LLM server implementations and some Hugging
  Face Inference Providers — reject the forced form and
  need `tool_choice: "auto"` (with the script-supplied
  tool still being the only one offered, so the model
  effectively must pick it).

  Touches `philharmonic-connector-impl-llm-openai-compat`
  only — no public-trait change, no other crate edits, no
  crypto path touched. Bump version 0.1.1 → 0.1.2 +
  CHANGELOG entry. Tests: dialect dispatch, the generated
  upstream request shape, response extraction (which
  should still parse `choices[0].message.tool_calls[0].
  function.arguments` exactly as `tool_call_fallback`
  does), reuse of the existing tool_call_fallback
  extraction path. WebUI gets no special treatment —
  endpoint configs are JSON-edited through the existing
  CodeMirror 6 editor (D10). Independent of everything
  else; small.

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

Two pending:

- **D14** Markdown parsing and rendering in WebUI chat
  bubbles, with **DOMPurify hardening** (or equivalent
  HTML sanitiser) — the assistant's reply content is
  workflow-script-generated and the script can be
  authored by anyone with `workflow:template_create`, so
  the chat tab must treat the content as untrusted and
  sanitise after parse, before render. Added 2026-05-11
  from
  [`HUMANS.md` §"Follow-up tasks from 2026-05-10 work"](../HUMANS.md).

  Lives in `philharmonic/webui/src/pages/InstanceDetail.tsx`
  (the chat tab) plus likely a new
  `components/MarkdownView.tsx` for reuse. New npm
  dependencies: a markdown parser (e.g. `marked` or
  `markdown-it`) plus `dompurify`. Bundle-size delta will
  not be trivial; surface in the prompt's residual risks.

  Sanitiser configuration must drop `script`, inline-event
  handlers (`onclick`, etc.), `javascript:` / `data:` URIs
  on links, and `iframe` / `object` / `embed`. Code blocks
  (\`\`\` fenced) should render with monospaced styling
  but no syntax highlighting in v1 (highlight.js adds
  significant bundle weight; defer). Tables, lists,
  headings, links (`http(s):` only), `code`, `pre`,
  `blockquote`, bold/italic/strikethrough are kept.

  Touches WebUI only; no backend changes. The chat-
  detection rules in `parseChatOutput` are unchanged —
  detection still uses the literal `content: string`
  shape. Markdown is a rendering concern, not a wire-
  format concern.

- **D15** Workflow-template `abstract_config` structured
  editor in the WebUI — replace the current raw-JSON
  CodeMirror 6 editor for `abstract_config` with a
  pull-down-menu-based UI: one row per (script-side
  endpoint name, endpoint UUID) binding, with the endpoint
  UUID column populated by a dropdown of active tenant
  endpoints. Added 2026-05-11 from
  [`HUMANS.md` §"Follow-up tasks from 2026-05-10 work"](../HUMANS.md).

  Structurally analogous to the `DataConfigEditor` shipped
  as the D11 follow-up on 2026-05-10 (`f040dce`
  philharmonic submodule). Differences:
  - Source for the dropdown is *all* active endpoints
    (filtered for `is_retired === false`), not just embed
    endpoints.
  - Binding-name validation rule is the same script-side
    name regex (`^[A-Za-z_$][A-Za-z0-9_$]{0,63}$`) — the
    abstract name becomes the JS-property the script uses
    in `endpoint("<name>", ...)`.
  - Retired-bound and missing-bound rows should surface
    the same warning badges to avoid silently dropping user
    data.

  Once D15 lands, the raw-JSON `abstract_config` editor
  goes away; both Create and Edit forms use the structured
  editor only. The friendly-UI mandate per HUMANS.md
  "Embedding DB component" final erratum applies
  transitively to `abstract_config` for the same reason it
  applies to `data_config`.

  Touches `philharmonic/webui` only (Templates.tsx,
  TemplateDetail.tsx, a new `components/
  AbstractConfigEditor.tsx`, `api/client.ts` type
  refinements, `templates.abstractConfig.*` i18n strings).
  No backend changes — the API already accepts the same
  `{<name>: <uuid>}` shape on Create and PATCH.

### Suggested sequencing

**Steps 1-6 (completed work, 2026-05-02 through 2026-05-10):**
D1+D2+D10 → Gate 1 → embedding-datasets feature end-to-end
(D3 r01 → r02 → D4+D5+caps+409 → Gate 2 → D6 WebUI) → D12 →
D13 → D11 (+ JP mirror). Per-step commit SHAs preserved at
[`docs/archive/2026-05-11-roadmap-completed-arc-trim.md`](archive/2026-05-11-roadmap-completed-arc-trim.md).

**Step 7 — next dispatchable**: D7 / D8 / D9 (Tier 2/3
connectors — SMTP, Anthropic, Gemini); D9 carries the
dual-mode AI Studio + Vertex AI requirement. All three are
independent and parallel-safe.

**Step 8 — newly added 2026-05-11** from HUMANS.md
follow-up directive: **D14** (markdown rendering in chat
with DOMPurify hardening, promoted from D13's deferred
list), **D15** (`abstract_config` structured editor in the
WebUI), **D16** (`tool_choice: "auto"` for
`llm_openai_compat`'s `tool_call_fallback` dialect). D14
and D15 are independent WebUI work; D16 is an independent
single-crate connector enhancement. Recommended ordering:
D16 first (unblocks providers currently rejecting forced
tool_choice) → D15 (UX smoothing that reduces config-paste
support burden) → D14 (chat UX polish, biggest bundle
impact).

### Dispatch discipline reminder

Per [`.claude/skills/codex-prompt-archive/SKILL.md`](../.claude/skills/codex-prompt-archive/SKILL.md)
and [`CONTRIBUTING.md §15.2`](../CONTRIBUTING.md#152-codex-prompt-archive):
every Codex prompt is archived to
`docs/codex-prompts/YYYY-MM-DD-NNNN-<slug>.md` and committed
**before** the spawn. Each dispatch above corresponds to one
archived prompt; multi-round work shares the daily-sequence
`NNNN` and increments the trailing `-NN`.
