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
