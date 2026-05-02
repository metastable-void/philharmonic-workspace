# Documentation Fix Follow-Up Audit

**Date:** 2026-05-02
**Prompt:** Direct chat request, 2026-05-02: "please check whether docs fixes landed cleanly by Claude, or still something wrong; also read HUMANS.md."
**Related reports:**
`docs/codex-reports/2026-05-02-0001-embedding-datasets-design-audit.md`,
`docs/codex-reports/2026-05-02-0002-full-docs-audit.md`

## Scope

I checked whether Claude's documentation-fix commits appear to have closed
the findings from the two earlier Codex reports and from the new `HUMANS.md`
errata. I read `HUMANS.md`, reviewed the recent git history, inspected the
changed live docs, and cross-checked the remaining suspicious text against
current mechanics endpoint TypeScript definitions and the vector-search
crate docs/code.

I treated historical prompts, notes, ROADMAP wave plans, crypto proposals,
and upstream fixture provenance notes as point-in-time records unless they
looked likely to mislead a current implementation prompt.

## Repository state observed

The recent history shows Claude/Yuka landed the Codex reports and follow-up
fixes in this order:

- `1a64a44` — `codex(docs): audit by Codex (commit by Yuka)`.
- `efff032` — batch 1: embedding-datasets design fixes,
  `TenantEndpointConfig` model coherence, workflow guide EN+JP example
  fixes, design 06 rewrite.
- `75db7f2` — batch 2: design 03/06 refresh, MSRV exceptions, phase/status
  language, rate limiting, `/v1/whoami`, operator routes, template PATCH.
- `be9cd07` — batch 3: link rot, fixture README path fixes, version-pin
  cleanup, `TenantCredential` -> `TenantEndpointConfig`, ROADMAP layer count.
- `dc3cb33` — note-to-humans summary of the full-docs-audit fixes.

During the original follow-up review the worktree was clean. When writing
this report, `git status --short` showed an unrelated untracked
`scripts/stats-log.sh`; I did not inspect or modify it.

## HUMANS.md context

`HUMANS.md` contains current workspace direction relevant to this audit:

- Keep docs, roadmaps, and plan docs up to date as implementation moves.
- The embedding datasets component should add `data` to the JS argument,
  inject assigned datasets as `data.embed_datasets.<assigned_name>`, use
  WebUI management, and run an ephemeral JS embed task inside the API bin.
- The explicit embedding DB errata are:
  - Embedding DB content slots should use deterministic CBOR.
  - Content slots should migrate to `LONGBLOB`, automatically on startup.
  - No raw JSON editor for Embedding DB; add a friendly UI.
- The WebUI should stay current and should use a sensible, maintained code
  editor dependency for JSON/JS editing.
- The workflow authoring guides in English and Japanese still need a fuller
  rewrite, but Claude's recent fixes targeted the concrete correctness bugs
  from the audit.

## Fixes that appear to have landed cleanly

### Embedding datasets storage and implementation constraints

`docs/design/16-embedding-datasets.md` now documents deterministic CBOR for
source items and corpus content slots, the MySQL `MEDIUMBLOB` to `LONGBLOB`
prerequisite, idempotent startup migration, operator-tunable hard limits,
source-items and corpus endpoints, per-job timeout work, and crypto-review
gating for lowerer changes.

This closes the largest structural problems from the first embedding dataset
audit: JSON blob density, 16 MiB substrate limits, nonexistent per-job
timeout surface, inconsistent "lowerer changes/no changes" language, missing
source-items endpoint, 409-vs-queue ambiguity, and the need to route
ephemeral lowerer/token/AAD changes through Yuka's two-gate crypto review.

### TenantEndpointConfig model coherence

The current live design docs now consistently describe the endpoint config
model as:

- `implementation` is a plaintext content slot.
- `encrypted_config` contains only the connector-implementation-specific
  config.
- The lowerer assembles `{realm, impl, config}` after reading the plaintext
  implementation, resolving the realm via deployment config, and decrypting
  the SCK blob.

The old byte-identical re-encryption model remains in historical ROADMAP and
crypto proposal text, but the live docs that should guide implementation no
longer rely on it. `docs/design/09-policy-and-tenancy.md` explicitly calls
out the earlier byte-identical model as abandoned.

### Workflow authoring guide examples

The English workflow authoring guide now includes `url_param_specs` for the
generic HTTP example and explains that every `{name}` in `url_template` must
have a matching spec entry. It also fixes the LLM examples to include
`output_schema` and to read normalized responses via
`response.body.output.<field>`, not OpenAI-native
`choices[0].message`.

The Japanese guide now has the same targeted bug fixes. `HUMANS.md` still
asks for a fuller guide rewrite later; that is broader than this audit's
"examples are technically wrong" finding.

### Other full-docs-audit items

The design 03/06 current-state refresh, MSRV 1.89 exception documentation,
substantive-vs-placeholder crate language, design 15 historical-current
split, per-minting-authority rate-limiting text, `/v1/whoami` and operator
route documentation, workflow template PATCH documentation, link-rot fixes,
fixture README path updates, `TenantCredential` replacement, and ROADMAP
layer-count correction all appear to be present.

## Remaining live issues

### High: embedding datasets script example still uses the old endpoint shape

`docs/design/16-embedding-datasets.md:253-275` still has a broken workflow
script example:

- `const embedResult = await endpoint("embed", ...)`
- `const queryVector = embedResult.embeddings[0]`
- `const searchResult = await endpoint("vector_search", ...)`
- `const relevantDocs = searchResult.results.map(...)`
- `const llmResult = await endpoint("llm", ...)`
- `return { output: { text: llmResult.output.text }, ... }`

That contradicts the current mechanics endpoint contract. The TypeScript
definition in `mechanics-core/ts-types/mechanics-endpoint.d.ts:43-66` says
`endpoint(...)` returns an `EndpointResponse` envelope with `body`, `headers`,
`status`, and `ok`; parsed JSON lives in `response.body`. The fixed workflow
guide now teaches the same thing at `docs/guide/workflow-authoring.md:244-251`
and `docs/guide/workflow-authoring.md:263-270`.

The same example also calls the `llm` endpoint without `output_schema`.
`docs/design/08-connector-architecture.md:840-869` says `output_schema` is
required, and the fixed workflow guide examples now include it. A reader who
copies this design 16 script would get a connector validation failure or a
runtime `undefined` access.

Suggested correction shape:

```js
const embedResponse = await endpoint("embed", {
  body: { texts: [input.text] }
});
const queryVector = embedResponse.body.embeddings[0];

const searchResponse = await endpoint("vector_search", {
  body: { query_vector: queryVector, corpus, top_k: 5 }
});
const relevantDocs = searchResponse.body.results
  .map(r => r.payload?.text)
  .join("\n");

const llmResponse = await endpoint("llm", {
  body: {
    model: "gpt-5.5",
    messages: [
      { role: "system", content: `Answer using: ${relevantDocs}` },
      { role: "user", content: input.text }
    ],
    output_schema: {
      type: "object",
      properties: { text: { type: "string" } },
      required: ["text"],
      additionalProperties: false
    }
  }
});

return { output: { text: llmResponse.body.output.text }, context, done: true };
```

The exact schema can differ, but the example should use `response.body` and
include `output_schema`.

### Medium: raw JSON editor wording still conflicts with HUMANS.md

`docs/design/16-embedding-datasets.md:532-537` correctly says "Friendly UI,
not raw JSON" and explains why a raw textarea is unusable for datasets. But
`docs/design/16-embedding-datasets.md:547-555` then says the create form has
a collapsed JSON editor for each row's payload and that "A raw-JSON view is
available behind a toggle for power users / batch import."

This is at least ambiguous and probably conflicts with the explicit
`HUMANS.md` erratum: "No raw JSON editor for Embedding DB: please add a
friendly UI." If the intended allowance is "paste/import JSON into a
structured importer" rather than "edit the dataset as raw JSON", the design
should say that clearly. If payload-level JSON editing is still allowed, it
should be distinguished from an entire dataset raw JSON editor.

Suggested direction:

- Keep structured row editing for `id` and `text`.
- For `payload`, either use a maintained JSON editor component with schema
  validation or a structured key/value editor.
- For batch import, use an import modal that accepts CSV/JSON and converts it
  into structured rows after validation.
- Avoid a persistent "raw JSON view/editor" for the dataset itself unless
  Yuka explicitly accepts that interpretation.

### Medium: vector_search docs still say "strings-only id payload"

Two live design docs still describe the vector-search implementation as
"strings-only id payload":

- `docs/design/08-connector-architecture.md:1150-1154`
- `docs/design/14-open-questions.md:72-75`

Current crate docs and code disagree. The vector-search README says the
implementation returns ranked `{id, score, payload}` results. The request and
response structs in `philharmonic-connector-impl-vector-search/src/request.rs`
and `philharmonic-connector-impl-vector-search/src/response.rs` carry
`payload: Option<JsonValue>`, and tests assert payloads are echoed when
present.

This is a small wording bug, but it matters because design 16 depends on
vector-search payloads to carry source text (`r.payload?.text`). The docs
should say something like "string IDs plus optional JSON payload echoed from
matching corpus items."

### Low: design 16 mentions a SQLite substrate that does not exist yet

`docs/design/16-embedding-datasets.md:99-100` and
`docs/design/16-embedding-datasets.md:609-612` say the SQLite substrate uses
untyped `BLOB` and is unaffected by the `LONGBLOB` migration. I found no
current `philharmonic-store-sqlx-sqlite` crate in this checkout. The only
other mention is a long-term note in
`docs/notes-to-humans/2026-04-27-0003-phase-8-design-and-decisions.md`.

This is lower risk than the script example, but live design docs should not
present a nonexistent substrate as current. Either remove the SQLite sentence
or make it explicitly future-looking.

## Non-issues checked

- `choices[0].message` references in upstream fixture READMEs are provenance
  notes about OpenAI-native upstream envelopes, not current workflow-authoring
  guidance.
- Byte-identical re-encryption text remains in historical ROADMAP wave-plan
  sections and older crypto proposals. The current design docs now mark that
  model as abandoned, so I did not treat those historical records as live
  contradictions.
- The link fixes for README, AGENTS, ROADMAP, crypto proposal paths,
  `CONTRIBUTING.md` from under `docs/`, and fixture README paths appear to
  have landed.

## Verification commands

Commands run during this follow-up:

- `uname -s` -> `Linux`.
- `./scripts/xtask.sh calendar-jp` -> JST 2026-05-02 Saturday, early
  afternoon; weekend/out-of-hours context.
- `git status --short`.
- `git log --oneline --decorate -8`.
- `find docs/notes-to-humans -maxdepth 1 -type f -name '2026-05-02*'`.
- Targeted `rg`, `sed`, and `nl -ba` inspections across
  `docs/design/16-embedding-datasets.md`,
  `docs/guide/workflow-authoring.md`,
  `docs/design/08-connector-architecture.md`,
  `docs/design/14-open-questions.md`,
  `mechanics-core/ts-types/mechanics-endpoint.d.ts`, and
  `philharmonic-connector-impl-vector-search`.
- `./scripts/check-md-bloat.sh`.
- `./scripts/tokei.sh`.

No Rust files were edited, and no cargo/pre-landing run was needed for this
documentation-only follow-up.
