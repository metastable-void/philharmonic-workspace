# 2026-05-11 ROADMAP trim — completed post-v1 arc

Pre-trim verbatim text of the completed §3.A
(Embedding datasets, 6 dispatches + 1 Gate-1), §3.C D12
(custom_headers), §3.D D10/D11/D13 (WebUI), and the
Suggested-sequencing steps 1-6 from `docs/ROADMAP.md`.

Trimmed from the live ROADMAP on 2026-05-11 because that
file is for forward-looking planning, not per-dispatch
retrospective. Commit history and the
`docs/codex-prompts/` archives carry the implementation
detail; this file preserves the planning-shaped per-
dispatch what/why/how for the same arc. The Current state
preamble at the top of `docs/ROADMAP.md` still summarises
the same facts at executive level.

Prior trim archive: [`2026-05-10-readme-roadmap-trim.md`](2026-05-10-readme-roadmap-trim.md)
(pre-v1 milestone phases archive, separate concern).

---

## §3.A Embedding datasets (6 dispatches + 1 Gate-1) — DONE 2026-05-10

Authoritative design:
[`docs/design/16-embedding-datasets.md`](../design/16-embedding-datasets.md).

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

---

## §3.C D12 — `llm_openai_compat` custom_headers — DONE 2026-05-10

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

---

## §3.D WebUI completed work (D10, D11, D13) — DONE 2026-05-02 / 2026-05-10

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
  `docs-jp/ワークフロー作成ガイド.md` is **not** a Codex
  dispatch — `docs-jp/README.md` reserves that submodule to
  Claude Code. Claude regenerates the JP guide as a follow-up.
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
  Bundle delta ~+3.0 KiB gzipped. Open follow-ups:
  markdown rendering in chat bubbles → **promoted to D14**
  per HUMANS.md 2026-05-11 follow-up directive; full
  instance-list dropdown for templates with many active
  chats (deferred); JP phrasing review (deferred); optional
  global "resume last chat" shortcut (deferred).

---

## Suggested-sequencing steps 1-6 — completed work history

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
   2026-05-10 (`10acd7f`). JP mirror regeneration done
   the same day (`e159e88` docs-jp + `6913a9d` parent).

---

## D11 follow-up landed 2026-05-10 (already covered in the live ROADMAP preamble; reproduced here for the completed-arc record)

- **JP mirror** of the workflow authoring guide regenerated
  to match the new English version
  (`docs-jp/ワークフロー作成ガイド.md`, 406 → 1368 lines;
  `e159e88` docs-jp + `6913a9d` parent).
- **WebUI template-form `data_config` editor** — structured
  embedding-dataset binding editor on Create + Edit forms
  with binding-name validation against
  `^[A-Za-z_$][A-Za-z0-9_$]{0,63}$`, retired/missing warning
  badges, friendly-UI per HUMANS.md (Codex r01 `f040dce`
  philharmonic + `db9f737` parent).
- **Design-doc reconciliation** — `design/07` script-arg
  shape five-field `{context, args, input, subject, data}`
  with full `data` semantics; `design/10` template body +
  PATCH semantics extended with `data_config` (`4b6a122`).

## Late-Sunday fix-its 2026-05-10 evening

- `scripts/build-status.sh` extended to detect running
  `build-script-build` executables (previously reported "no
  active Rust build processes" when a build.rs was the only
  thing running; `86c7312`).
- Workflow authoring guide (en + jp) now flags the WebUI
  config-paste UX trap — `display_name` / `implementation`
  go in form fields; only the inner `config` value goes in
  the Config JSON editor (`48fe697`).
- **Connector-path body cap raised 2 MiB → 32 MiB**
  (`philharmonic-connector-router` 0.1.1 → 0.1.2;
  `85e2ad8`). The previous 2 MiB axum default rejected
  `vector_search` corpus bodies over ~170 items at 1024-dim
  with an HTTP 413 propagated up as a generic internal-error
  envelope. No crypto-shape change.

---

# Evening trim — 2026-05-11

Pre-trim verbatim text of the D14 / D15 / D16 verbose §3.C
and §3.D entries plus the Suggested-sequencing steps 7-9
from the 2026-05-11 evening trim. Trimmed once all three
were done and the section-3 verbose-description form had
served its planning purpose.

## §3.C D16 — `tool_call_fallback_auto` dialect — DONE 2026-05-11

- **D16** `philharmonic-connector-impl-llm-openai-compat`
  `tool_call_fallback_auto` dialect variant — **DONE
  2026-05-11** (`e523238` submodule + `b368c4b` parent;
  Codex r01 under prompt
  `2026-05-11-0001-d16-llm-openai-compat-tool-call-fallback-auto-01`).
  Sub-shape 1 chosen: `tool_call_fallback.rs` exposes a
  shared `pub(crate) translate_request_with_tool_choice`
  helper that both variants call; the new module supplies
  `json!("auto")` and delegates response extraction
  directly. Version 0.1.1 → 0.1.2 (patch bump per pre-1.0
  SemVer; semver-checks flagged the public-enum-variant
  addition as breaking — anticipated, surfaced in
  residuals). Existing `tool_call_fallback` tests remain
  green byte-for-byte (back-compat guarantee).
  Structured-output-contract resume needed (original
  background task died post-verification; rescue spawn
  emitted the six-section report against the
  working-tree state). Streak now 9/9 since the contract
  was added.

## §3.D D14 — markdown rendering in chat with DOMPurify — DONE 2026-05-11

- **D14** Markdown parsing and rendering in WebUI chat
  bubbles with DOMPurify hardening — **DONE 2026-05-11**
  (`f750b4a` philharmonic submodule + `c1fbff7` parent;
  bundled with D15 in Codex r01 under prompt
  `2026-05-11-0002-d14-d15-...-01`). New
  `components/MarkdownView.tsx` (122 LOC): `marked` +
  `dompurify` with a strict ALLOWED_TAGS allowlist (block
  elements + inline emphasis + tables + http(s)-only
  `<a><img>`), ALLOWED_ATTR limited to alt/href/src/title,
  FORBID_TAGS for scripts/iframes/forms/styles/etc., and
  an `afterSanitizeAttributes` hook that removes on*
  handlers, strips non-http(s) href/src, and hardens
  surviving `<a>` with target=_blank +
  rel=noopener noreferrer nofollow. `useMemo` on source
  prevents per-bubble re-parse on chat-tab re-render. The
  chat tab on `InstanceDetail.tsx` wraps each bubble's
  content with `<MarkdownView className="chat-markdown"
  source={message.content} />` inside the existing
  chat-bubble div — bubble layout unchanged. Codex's
  informal XSS sanity check via throwaway /tmp jsdom
  harness confirmed the sanitiser drops scripts /
  onclick / javascript: / data: and hardens https:
  links as specified. `parseChatOutput` detection rules
  unchanged. Bundle delta +22,480 B gzipped.

## §3.D D15 — `abstract_config` structured editor — DONE 2026-05-11

- **D15** Workflow-template `abstract_config` structured
  editor — **DONE 2026-05-11** (`f750b4a` philharmonic
  submodule + `c1fbff7` parent; bundled with D14 in the
  same Codex r01). New
  `components/AbstractConfigEditor.tsx` (317 LOC) mirrors
  the `DataConfigEditor.tsx` 315 LOC precedent from D11
  follow-up #3: per-row binding-name validation against
  `/^[A-Za-z_$][A-Za-z0-9_$]{0,63}$/`, duplicate-name
  detection, dropdown filtered for `is_retired=false`,
  retired-bound / missing-bound warning badges so user
  data is never silently dropped, disabled mode for
  save-in-flight. `api/client.ts` adds an `AbstractConfig`
  type alias + a `listEndpoints` helper with a
  cursor-walking auto-loader (handles >100-endpoint
  tenants cleanly without truncation, Codex's call vs.
  the prompt's truncate-with-hint alternative). Both
  Templates.tsx Create and TemplateDetail.tsx Edit forms
  now use the structured editor; the raw-JSON CodeMirror
  abstract_config editor and its `configText` state slot
  are removed entirely (no fallback). New
  `templates.abstractConfig.*` i18n namespace in en + ja
  mirroring `templates.dataConfig.*` shape. No new
  permission atoms — reuses `endpoint:read_metadata` +
  `workflow:template_create`. Bundle delta +828 B
  gzipped. Combined D14+D15 bundle delta +23,308 B
  (~+22.8 KiB) gzipped.

## Suggested-sequencing steps 7-9 — completed-work history

7. **D16** `tool_call_fallback_auto` for `llm_openai_compat`
   — DONE 2026-05-11 (`e523238` submodule + `b368c4b`
   parent).
8. **D14** markdown rendering in chat with DOMPurify
   hardening — DONE 2026-05-11 (bundled with D15 in
   `f750b4a` philharmonic submodule + `c1fbff7` parent).
9. **D15** `abstract_config` structured editor — DONE
   2026-05-11 (same Codex r01 as D14).

## 2026-05-11 post-D15 deployment-time polish (NOT numbered Codex dispatches)

Beyond the three numbered dispatches D14/D15/D16, the
2026-05-11 deployment-time testing surfaced a series of
fixes and polish items that landed the same day. None of
these were on the original ROADMAP §3 dispatch plan; they
were testing-time observations promoted to small fixes:

- **`mechanics-core` 0.3.2 → 0.4.0** (`5cbe72c` mechanics-core
  submodule + `6ed5ee2` parent) — runtime stopped overriding
  `main`'s fulfilled-promise success with "Unhandled promise
  rejection" engine errors. Boa's `NativeFunction::from_async_fn`
  rejects an inner promise that the await-chain machinery
  wraps in an outer continuation promise; the spec-compliant
  `promise_rejection_tracker` fires `Reject` on the inner
  rejection (no handlers at that moment) but the matching
  `Handle` event doesn't reliably propagate to the inner
  promise when the await's handler attaches to the wrapper.
  Counter ended positive even when every JS-visible
  rejection got caught. The strict check produced
  false-positive step failures for any workflow with
  `try { await endpoint(...) } catch (e) { ... }`. Module-
  evaluation-time check kept strict; trade-off accepts
  silently-misbehaving fire-and-forget rejections (rare in
  practice) over breaking the common correct pattern.
- **`philharmonic-api` 0.1.7 → 0.1.8** (`ab7bc25`
  philharmonic-api + `d19cc76` parent) — `WhoamiResponse`
  extended with `permissions: Vec<String>` (effective atom
  set after envelope clipping). Principal-auth path joins
  role-membership permissions; ephemeral-auth path clones
  token's clipped claims. Sort + dedup. Additive field;
  older clients ignore it.
- **WebUI permission-aware nav + disabled non-actionable
  buttons + sticky sidebar footer** (`eb9184d` philharmonic
  submodule, same parent commit as above) — Codex r01 under
  prompt
  `2026-05-11-0003-webui-permission-aware-ui-and-sidebar-sticky-01`.
  Sidebar hides routes the caller has no read permission
  for; action buttons across all 15 pages render `disabled`
  with `title="Missing permission: <atom>"` tooltips
  instead of letting users click into a 403; sidebar
  `position: sticky; top: 0; align-self: start;
  max-height: 100vh` so the language switcher / token /
  logout footer stays reachable regardless of nav-list
  length. Server-side route-protector enforcement
  unchanged (still the security boundary; the WebUI just
  stops users from hitting it accidentally). `usePermissions`
  hook reads from `authSlice.permissions`, populated from
  `WhoamiResponse` on login.
- **Chat bubble assistant `name` field surfacing**
  (`afbc660` philharmonic submodule + `0c95618` parent) —
  D13 chat tab now renders an assistant turn's optional
  OpenAI-style `name` field (non-empty string) as the bubble
  role label in place of the generic "Assistant" /
  "アシスタント" string. Other roles unchanged. Workflow
  authoring guide (en + jp) describes the special-case UI
  surfacing.
- **Workflow authoring guide per-connector request/response
  shapes** (`9f96e2d` parent) — each shipped connector
  subsection in `docs/guide/workflow-authoring.md` (en + jp)
  gained a Request body + Response body description so
  workflow authors don't reverse-engineer the wire shapes
  from the connector crate source. Universal mechanics-core
  transport envelope (response = `{body, headers, status,
  ok}`) disambiguated from connector-specific
  `response.body` shapes; `http_forward`'s double-nest
  semantics (`response.body.body` for upstream body)
  explicitly called out.
- **Audit-log producer gap closed** (`b37f894`
  philharmonic-policy + `1ce191a` parent + `881c48a`
  philharmonic-api + `8d20d1d` parent) — three-piece fix:
  - philharmonic-policy 0.2.2 → 0.2.3: new
    `audit_event_type` module with 17 canonical i64
    discriminants for every audit-event category (1-9
    principals, 10-19 roles/memberships, 20-29 endpoints,
    30-39 authorities, 40-49 token mint, 50-59 tenant
    lifecycle); `name(i64) -> Option<&'static str>` for
    canonical snake_case labels; append-only numbering
    rule.
  - `docs/design/09-policy-and-tenancy.md §Audit trail`
    contract lock-in: `event_data` JSON schema convention
    (`principal_id` + `route` + `correlation_id` required;
    `target_entity_id` + `subject` per-event optional);
    token-mint payload privacy restriction (subject_id +
    authority_id only; never injected claims); audit-write
    failure semantics (log warn + return success on
    underlying mutation). Status block corrected from
    "audit events are shipped" to acknowledge the producer
    gap.
  - philharmonic-api 0.1.8 (no version bump): 19 producer
    call sites across 7 route files (principals, roles,
    memberships, endpoints, authorities, mint, operator),
    all using a shared `pub(crate) emit_audit_event`
    helper that wraps `write_audit_event` with the
    locked failure pattern (warn + continue). mint.rs's
    privacy restriction enforced by absence-assertions
    in `tests/audit_producers.rs` (7 e2e tests, all green).
    Open follow-up design questions queued (separate
    `AUTHORITY_ROTATED = 34` discriminant?, future
    `TENANT_MODIFIED` event for non-status updates?,
    `GET /v1/audit` response surfacing canonical names
    via `audit_event_type::name`?).

Per-piece commit threads + structured-output-contract
streak (now 11/11) preserved in the Codex prompt outcomes
under `docs/codex-prompts/2026-05-11-{0001,0002,0003,0004}-*.md`.
