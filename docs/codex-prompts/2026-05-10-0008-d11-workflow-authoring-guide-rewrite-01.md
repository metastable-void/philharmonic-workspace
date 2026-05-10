# D11 — Workflow authoring guide rewrite (initial dispatch)

**Date:** 2026-05-10
**Slug:** `d11-workflow-authoring-guide-rewrite`
**Round:** 01 (initial dispatch — D11, ROADMAP §3.D, single
file rewrite of `docs/guide/workflow-authoring.md`, no code
changes)
**Subagent:** `codex:codex-rescue`

## Motivation

The English workflow-authoring guide
[`docs/guide/workflow-authoring.md`](../guide/workflow-authoring.md)
predates several post-v1 features and now misrepresents
current implementation reality:

- **Embedding datasets (D3 / D4 / D5 / D6, ROADMAP §3.A,
  shipped 2026-05-10)**: the script argument is now
  `{context, args, input, subject, data}` — the existing
  guide shows only `{context, args, input, subject}`.
  `data.embed_datasets.<assigned_name>` is the canonical
  way workflows read corpus content, including the missing-
  during-first-embed and previous-corpus-during-re-embed
  semantics. The guide doesn't mention `data` or
  embedding datasets at all.
- **Chat-style testing UI (D13, ROADMAP §3.D, shipped
  2026-05-10)**: the WebUI now provides a one-click
  chat-tab affordance for any workflow that conforms to
  the `{content: <user_input>}` in / `{messages: [...]}`
  out contract, with empty-content POST as a dual-purpose
  probe (greeting on a fresh instance, transcript fetch
  on an existing one). Authoring a D13-compatible chat
  workflow is the most common new-user task; the guide
  should walk through the recipe explicitly.
- **`llm_openai_compat` `custom_headers` (D12, ROADMAP
  §3.C, shipped 2026-05-10)**: the endpoint config now
  carries a `custom_headers: BTreeMap<String, String>`
  field for provider-specific HTTP headers (Hugging Face
  `X-HF-Bill-To`, OpenAI `OpenAI-Organization`,
  OpenRouter `HTTP-Referer`/`X-Title`, etc.). The guide's
  `llm_openai_compat` config table doesn't list it.
- **Connector implementations enumeration**: the existing
  guide covers `llm_openai_compat`, `http_forward`,
  `sql_postgres` / `sql_mysql`, `embed`, `vector_search`.
  The remaining Tier 2/3 names are reserved on crates.io
  but not yet implemented (`email_smtp` D7, `llm_anthropic`
  D8, `llm_gemini` D9 — all pending). The guide should
  acknowledge the placeholder state, not pretend they don't
  exist.
- **Policy / API-token / rate-limit / endpoint-rotation
  flows** the guide skips: e.g., the four-byte permission
  envelope semantics, `pht_` token format, endpoint
  rotation API. These deserve at least pointers to design
  docs even if not full coverage.

The rewrite reflects current implementation reality — not
"what we wished we'd built". When the design docs and the
code disagree, **the code wins** (flag the divergence in
your structured output so Claude can reconcile the design
doc later).

The Japanese mirror at
[`docs-jp/ワークフロー作成ガイド.md`](../../docs-jp/) is **not**
in scope — `docs-jp/README.md` reserves that submodule to
Claude Code. Claude regenerates the JP guide after D11
lands.

## User's specific focus (verbatim from the dispatch)

> "D11, with a focus on how to author D13-compat-style
> workflows, with embed datasets."

Two recipes are load-bearing in the rewrite:

1. **Authoring a D13-compatible chat workflow** — input
   shape, output shape, empty-content dual-purpose
   semantics, the message accumulator pattern, when to
   set `done: true` for chat (almost never — the chat
   tab keeps probing, but the workflow can be marked
   complete via the existing "Complete" action on the
   instance detail page if the conversation has a
   natural end).
2. **Authoring a workflow that consumes embedding
   datasets** — `data_config.embed_datasets` template
   field, `data.embed_datasets.<name>` runtime access,
   `CorpusItem` shape, the four detail-page UI states
   (first embed in progress / re-embed with prior corpus
   / failed with prior corpus / failed without fallback),
   how to handle the dataset being absent from the Record
   (the workflow JS must defensively check; see the rule
   in HUMANS.md "Any data field can be absent to run a
   workflow successfully (unless JS requires it)").

The two recipes also combine: a chat workflow with a
knowledge-base lookup (RAG-style) is the canonical
Embedding-Datasets-+-Chat use case. Add a third recipe
combining both, mirroring how the Templates → Test in
chat → empty-content probe → greeting flow lights up
with a corpus-grounded assistant.

## References (authoritative — re-read in this order)

**Design docs** (Codex: re-read these before starting):

- [`docs/design/07-workflow-orchestration.md`](../design/07-workflow-orchestration.md)
  — workflow lifecycle, instance/template/step model,
  context/args/input/subject/data assembly. **Authoritative
  for the script argument shape.**
- [`docs/design/16-embedding-datasets.md`](../design/16-embedding-datasets.md)
  — embedding-datasets feature: data layer, deterministic
  CBOR codec, ephemeral embed job, the carry-forward
  rule for missing/failed embeds, the four detail-page
  UI states, the runtime `data.embed_datasets.<name>`
  access pattern.
- [`docs/design/08-connector-architecture.md`](../design/08-connector-architecture.md)
  — connector framework, the `Implementation` trait, the
  `llm_generate` capability's normalized
  `{output, stop_reason, usage}` shape with mandatory
  `output_schema`, transport envelope semantics
  (`{body, headers, status, ok}`), the
  `overridable_request_headers` / `exposed_response_headers`
  allowlists.
- [`docs/design/09-policy-and-tenancy.md`](../design/09-policy-and-tenancy.md)
  — permission atoms, role/membership model, tenancy
  scoping. Not a primary focus for the guide, but the
  Permissions section at the bottom of the existing
  guide needs the embedding-dataset atoms added.
- [`docs/design/10-api-layer.md`](../design/10-api-layer.md)
  — REST API surface, `pht_` token format, the seven
  embed-dataset routes, the workflow-template `data_config`
  field added in D3 round 02.
- [`docs/design/11-security-and-cryptography.md`](../design/11-security-and-cryptography.md)
  — pointers only (the guide should not duplicate
  cryptographic detail; cross-reference for context).

**Code (canonical for "what's actually shipped")**:

- `philharmonic-workflow/src/engine.rs` — the actual
  script argument assembly. `build_script_data` (around
  line 558) is the canonical source for the
  `data.embed_datasets.<name>` mapping, the
  carry-forward-prior-corpus rule, and the
  missing-from-Record semantics. **If this code
  contradicts design 16, the code wins.**
- `philharmonic-policy/src/embed_dataset_codec.rs` —
  CBOR encode/decode for `SourceItem` (input) and
  `CorpusItem` (workflow-visible). The `CorpusItem`
  shape (`{id, vector: f32[], payload?}`) is what
  workflows see at `data.embed_datasets.<name>[i]`.
- `philharmonic-policy/src/entity.rs` — the
  `EmbeddingDataset` entity definition, status enum
  (`Created=0`, `Embedding=1`, `Ready=2`, `Failed=3`),
  the `data_config` slot on `WorkflowTemplate`.
- `bins/philharmonic-api-server/src/embed_script.js` —
  the built-in JS embed script (Codex-authored, compiled
  into the API binary via `include_str!`). **Do not
  document this for users to copy** — it's an internal
  ephemeral job, not a workflow template authors write.
  Reference it only when discussing the embed pipeline's
  guarantees.
- `philharmonic-connector-impl-llm-openai-compat/src/config.rs`
  — current config shape including `custom_headers`,
  `dialect` enum, validation rules.
- `philharmonic-connector-impl-http-forward/src/config.rs`
  — `HttpEndpoint` shape Codex should mirror in the
  config-shape table.
- `philharmonic-connector-impl-llm-{anthropic,gemini}` and
  `philharmonic-connector-impl-email-smtp` — current
  state is `0.0.x` placeholder. Note as "reserved /
  pending implementation" in the guide; do not invent
  config shapes for them.
- `philharmonic-connector-impl-embed/src/config.rs` and
  `philharmonic-connector-impl-vector-search/src/config.rs`
  — current config shapes for the embed and vector-search
  endpoints; cross-check the existing guide's snippets
  against current code.
- `mechanics-core/ts-types/` — TypeScript shapes for the
  built-in `mechanics:*` modules. The existing guide
  enumerates them; verify the list and signatures are
  current.
- `philharmonic/webui/src/api/client.ts` — `ChatMessage` /
  `ChatTranscript` / `ChatRole` shapes from D13 (lines
  177-253). These are the runtime types the chat UI
  parses; the guide's chat-workflow recipe must produce
  output that conforms.

**Other ground-truth docs**:

- [`HUMANS.md`](../../HUMANS.md) §"Embedding DB component"
  — Yuka's spec for the `data` field's behavior:
  "Any data field can be absent to run a workflow
  successfully (unless JS requires it). `data` is `{}`
  when no data fields exist." The guide must surface this
  as a hard contract.
- [`HUMANS.md`](../../HUMANS.md) §"Chat UI for easy
  testing" — the load-bearing source for the D13 input/
  output shapes and the empty-content dual-purpose
  semantics.
- The D13 archived prompt
  ([`docs/codex-prompts/2026-05-10-0007-d13-chat-testing-ui-01.md`](2026-05-10-0007-d13-chat-testing-ui-01.md))
  — concrete details on the runtime structural detection,
  the dual-purpose probe, and the
  `parseChatOutput` rules. The guide must keep workflows
  authored against it on the supported side of detection.

## Context files pointed at

- `docs/guide/workflow-authoring.md` — **target file for
  rewrite**. Current content is 530 lines; the rewrite
  may grow to roughly 700-1100 lines depending on how
  much detail the embedding-datasets and chat recipes
  need. Don't artificially cap.
- `docs/guide/` — only file in the directory currently;
  the rewrite stays a single file.

## Outcome

**Completed 2026-05-10** — single-file rewrite of
`docs/guide/workflow-authoring.md` landed at commit
`10acd7f`. 530 → 1350 lines (~4,463 words).

`./scripts/test-scripts.sh` + `./scripts/check-md-bloat.sh`
both passed. No Rust / TS / shell touched. Wire-shape
verifications via grep all green:

- `vector_search` request fields (`query_vector`,
  `corpus`, `top_k`, `score_threshold`) match
  `philharmonic-connector-impl-vector-search/src/request.rs`
  lines 9-17.
- `embed` request (`texts`) / response (`embeddings: f32[][]`)
  match `philharmonic-connector-impl-embed/src/{lib,
  response}.rs`.
- `data` field assembly + carry-forward / missing-from-
  Record states match `philharmonic-workflow/src/engine.rs`
  `build_script_data` (line 558+).
- D12 `custom_headers` field + reserved-header validation
  rules match
  `philharmonic-connector-impl-llm-openai-compat/src/config.rs`.
- D13 chat-output detection rules match the rewrite's
  description of the WebUI's `parseChatOutput` parser
  in `philharmonic/webui/src/api/client.ts`.

**Codex's deliverable choices**:

- Three recipes implemented as the user's focus directive
  asked: D13-compat chat (state-driven accumulator
  pattern), embedding-datasets workflow with five
  availability states, combined chat+RAG with a system-
  prompt context-block pattern. All three include
  copy-pasteable script source + template JSON +
  endpoint setup JSON + WebUI behavior table +
  permissions list.
- Five (not four) availability states for embedding
  datasets — Codex correctly added `Retired` as a
  separate row in the visibility-states table on top
  of the four design-16 documented states. Matches
  `is_retired = true → omitted` per design 16.
- Tier 2/3 connectors (email_smtp, llm_anthropic,
  llm_gemini) flagged as "reserved / pending
  implementation" rather than fabricated config shapes.
- Permission atoms table extended with `embed_dataset:*`
  atoms; existing workflow / endpoint / tenant / mint /
  audit groups preserved.
- Cross-references to design docs, ROADMAP, HUMANS.md,
  and the D13 archived prompt.

**Structured-output-contract honored** for the **seventh**
consecutive round. Streak now 7/7 since the contract was
added.

**Open questions Codex surfaced** (carried forward, all
follow-up work for Claude — not in D11 scope):

1. **Design-doc divergences requiring reconciliation**:
   - `docs/design/07-workflow-orchestration.md` still
     documents the pre-D3 4-field script-arg shape
     `{context, args, input, subject}`; needs the 5-field
     `+ data` update.
   - `docs/design/10-api-layer.md` does not list the
     `data_config` field in template body docs added
     in D3 round 02.
2. **JP mirror regeneration** at
   `docs-jp/ワークフロー作成ガイド.md` is a Claude follow-up
   task per ROADMAP §3.D — will follow this commit.
3. **Product gap**: WebUI template form lacks
   `data_config` exposure (currently API-only); worth
   tracking as a UX follow-up.

**Day's dispatch arc complete**:
D1/D2/D3/D4/D5/D6/D10/D11/D12/D13 done plus Gate 1 +
Gate 2 approved (10 of 13). Remaining post-v1 work: D7
(SMTP), D8 (Anthropic), D9 (Gemini dual-mode AI Studio +
Vertex AI per ROADMAP §3.B). All three are independent
Tier 2/3 connector implementations with no crypto path
involvement.

---

## STRUCTURED-OUTPUT-CONTRACT — READ THIS FIRST

Rounds 02 / 03 / D12 / D6 / D13 (the last five) all honored
the contract: `RUN STATUS: COMPLETE` token + six-section
report emitted before `task_complete`. Streak is 6/6 since
the contract was added. **Maintain the bar — do not break
the streak.**

The contract is repeated at the end of the prompt; it's on
you to actually emit it before `task_complete`.

---

## Verification expectations

This is a docs-only rewrite. There's no Rust to compile, no
TypeScript to typecheck, no bundle to build. **Verification
is correctness against the cited authoritative sources, not
build-pipeline output.**

What you should do instead of running build pipelines:

1. After drafting each section, **grep the code** for the
   specific claim (function name, field name, JSON key,
   permission atom, route path) to confirm the doc matches.
   The grep output is the verification artifact.
2. **Run `./scripts/test-scripts.sh`** at the end to
   confirm you didn't accidentally touch a `scripts/*.sh`
   file (you shouldn't have).
3. **Run `./scripts/check-md-bloat.sh`** if you grow the
   guide significantly — the script reports markdown file
   sizes and flags bloated docs. Not a gate, but a sanity
   check against runaway expansion.
4. **Skip `pre-landing.sh`** unless you accidentally
   touched Rust source — D11 is a docs-only dispatch.

---

## Prompt (verbatim)

<task>
Rewrite `docs/guide/workflow-authoring.md` from scratch to
reflect current implementation reality. Single file, no
code changes, no other doc edits.

The user's specific focus (verbatim from the dispatch):

> "D11, with a focus on how to author D13-compat-style
> workflows, with embed datasets."

Two recipes are load-bearing in the rewrite:

1. Authoring a D13-compatible chat workflow.
2. Authoring a workflow that consumes embedding datasets.

A third combined recipe (chat + knowledge-base lookup, the
canonical RAG use case) ties the two together and is the
most common new-user shape.

If anything below contradicts
[`docs/design/07-workflow-orchestration.md`](docs/design/07-workflow-orchestration.md)
or
[`docs/design/16-embedding-datasets.md`](docs/design/16-embedding-datasets.md)
or the canonical implementation files cited in this prompt,
**the implementation files win**. Flag the contradiction in
your structured output so Claude can reconcile the design
docs separately.

If you hit scope limits, finish whichever sections are
closest to done and report what's left in the structured
output. **Do not** ship a guide that's half-rewritten with
inconsistent claims between sections — either revert
in-progress sections or finish them.

## Section structure (suggested — adjust if a different
##  shape serves the recipes better)

```
# Workflow Authoring Guide

## What workflows are            (lifecycle, vocabulary —
                                  templates / instances /
                                  steps; statuses)

## Setting up endpoints           (one subsection per
                                  shipped connector impl;
                                  Tier 2/3 placeholders
                                  flagged)

## Creating a template            (API + WebUI; including
                                  abstract_config and the
                                  new data_config slot)

## Writing scripts                (script-arg shape with
                                  the `data` field; return
                                  shape; sandboxing;
                                  built-in modules)

## Calling endpoints              (mechanics:endpoint;
                                  llm_generate's normalized
                                  shape with mandatory
                                  output_schema; transport
                                  envelope; overridable
                                  headers)

## Reading embedding datasets     (data_config; data.
                                  embed_datasets.<name>;
                                  CorpusItem shape;
                                  carry-forward; defensive
                                  presence check)

## Authoring a chat workflow (D13-compatible)
                                  (full recipe with empty-
                                  content dual-purpose
                                  semantics; how the WebUI
                                  detects compatibility)

## Authoring a chat workflow with embedding datasets (RAG)
                                  (combined recipe; the
                                  most common shape)

## Running a workflow             (create instance; execute
                                  step; check status; view
                                  step history; complete /
                                  cancel)

## Instance lifecycle             (state machine)

## Examples                       (echo, basic LLM chat,
                                  D13-compatible chat,
                                  embedding-datasets RAG)

## Permissions                    (atoms table — workflow:*,
                                  endpoint:*, embed_dataset:*,
                                  tenant:*, mint:*, audit:*)

## Cross-references               (design docs, ROADMAP for
                                  pending items, HUMANS.md
                                  for product directives,
                                  WebUI README if any)
```

The "Examples" section can either expand each example
inline or cross-reference the recipe sections. Codex's
call.

## Hard requirements

### Script argument shape (load-bearing)

The current shape per
`philharmonic-workflow/src/engine.rs` (verify by reading)
is:

```js
{
  context, // mutable state from previous steps
  args,    // instance creation arguments
  input,   // step-specific input
  subject, // caller identity (read-only)
  data,    // {} or {embed_datasets: {<name>: CorpusItem[]}, ...}
}
```

The `data` field is **always present** in the script
argument; its value is `{}` when the template has no
`data_config`. When the template has `data_config.
embed_datasets.<name> = <dataset-uuid>`, the engine
populates `data.embed_datasets.<name>` with the dataset's
current `CorpusItem[]`. **Crucial nuance** (verify from
`build_script_data` in `engine.rs`):

- A dataset that is currently first-embedding (status
  `Created`/`Embedding` with no prior `Ready` corpus) is
  **omitted from `data.embed_datasets`** — the JS must
  defensively check for the key's presence.
- A dataset re-embedding (status `Embedding` with prior
  `Ready` corpus) keeps serving the **previous corpus**
  until the new embed completes, so the JS sees stable
  data.
- A dataset that previously embedded successfully but
  whose latest re-embed `Failed` keeps serving the
  previous corpus indefinitely (until next successful
  re-embed); the workflow can stay functional even
  through embed failures.
- A dataset whose **first embed** failed and never
  succeeded is omitted entirely (no fallback). Workflow
  JS must handle this case gracefully.
- A dataset that is `is_retired = true` is omitted (per
  design 16's "retired datasets are excluded from all
  queries" rule).

These four states map to the four UI states in the D6
WebUI's `EmbedDatasetDetail` page. Document them.

### CorpusItem shape (load-bearing)

Per `philharmonic-policy/src/embed_dataset_codec.rs`:

```ts
type CorpusItem = {
  id: string,
  vector: number[],   // f32 array, dimensionality from embed config
  payload?: JsonValue, // optional, omitted entirely if absent
};
```

The vector is a JSON number array of f32 components. The
payload is whatever the dataset author put in the source
item's optional `payload` field (a JsonValue passed through
end-to-end without interpretation; for RAG workflows,
typical payloads are `{title, source_url, chunk_index}` or
similar).

### D13 chat-workflow contract (load-bearing)

Per
[`HUMANS.md`](HUMANS.md) §"Chat UI for easy testing" and
ROADMAP §3.D:

**Input shape** (sent by the chat UI on every turn):

```js
arg.input = {} // empty-content probe (dual-purpose)
arg.input = { content: "<user message>" } // user turn
```

The empty-content case is dual-purpose:

- **On a fresh instance** (no prior context): the
  workflow generates the **opening turn** — typically a
  greeting — so the chat UI lands the admin on a
  populated transcript without needing to send a dummy
  message.
- **On an existing instance** (populated context): the
  workflow returns the **current transcript unchanged**,
  with no new turn appended. Useful for tab re-opens,
  refresh, sharing a chat URL.

**Output shape** (returned every turn):

```js
return {
  output: {
    messages: [
      { role: "user" | "assistant" | "system" | string, content: "..." },
      ...
    ],
  },
  context: { messages },  // accumulator pattern
  done: false,            // chat workflows generally don't terminate
};
```

The `messages` array is the **full** transcript — every
turn, not a delta. The chat UI re-renders all bubbles
from each response (no client-side merging).

The `role` field accepts `user`, `assistant`, `system`, or
any non-empty string (the parser is permissive on roles
beyond the OpenAI canonical three; non-canonical roles
get a neutral bubble style with the role label visible).

**Per-message extra fields** (e.g., `name`, `tool_calls`,
provider-specific) are preserved by the WebUI's
`parseChatOutput` parser as opaque passthrough. Workflows
may attach them; they round-trip through the UI but
don't render specially.

**Detection** (runtime, not metadata-based): the WebUI
fires `executeInstance(id, {input: {}})` once on chat-tab
mount and inspects the output. If `output.messages` is
an array of `{role: non-empty string, content: string}`
objects, the workflow is chat-compatible. Anything else
gets a clean "not chat-compatible" empty state. **Document
the detection rule** so workflow authors understand why
returning, say, `{transcript: [...]}` instead of
`{messages: [...]}` makes their workflow non-detectable.

**Done-semantics**: chat workflows generally **do not**
return `done: true` because the chat UI keeps the
conversation open indefinitely. If you want a workflow to
terminate after N turns or some condition, use the
`Complete` button on the instance detail page (the
workflow can also signal terminal state by returning a
final assistant message and `done: true`, but the chat
tab will stop being usable after that).

**Two reasonable patterns** for the empty-content branch:

1. **State-driven**: inspect `arg.context.messages`. If it's
   empty / undefined, generate a greeting and append it.
   Otherwise return the existing transcript unchanged.
2. **Idempotent reducer**: always materialize
   `arg.context.messages || []`, optionally append a turn
   when `arg.input.content` is non-empty, return the
   accumulator. The empty-content-on-fresh-instance case
   is handled by an explicit "if messages is empty and no
   input, push a greeting" branch.

The recipe should pick one and walk through it.

### Embedding-datasets workflow contract

The workflow author:

1. Creates an embed endpoint (POST /v1/endpoints with
   `implementation: "embed"`).
2. Creates the embedding dataset (POST /v1/embed-datasets
   with `embed_endpoint_id`, `display_name`, source items).
   The system kicks off an ephemeral embed job in the
   background; the dataset's `status` transitions
   `Created → Embedding → Ready` (or `Failed` on error).
3. Assigns the dataset to a workflow template via the
   template's `data_config.embed_datasets.<assigned_name>`
   = `<dataset_uuid>` field. The `<assigned_name>` becomes
   the key the workflow JS reads via `data.embed_datasets.
   <assigned_name>`.
4. Re-embeds whenever source items change (POST
   /v1/embed-datasets/{id}/update). The previous corpus
   keeps serving workflows during re-embed.

Each `CorpusItem` carries a vector that the workflow can
feed to a vector-search endpoint (cosine similarity, top-K,
etc. — the `vector_search` connector handles the actual
similarity calculation; the workflow just provides the
corpus + query vector).

For RAG: the workflow embeds the user's query (via the
embed endpoint), passes the corpus from
`data.embed_datasets.<name>` plus the query vector to the
`vector_search` endpoint, gets top-K matching items, and
includes their `payload` (or original source-item text)
in the LLM prompt as retrieved context.

## Recipe-level requirements

For each of the three recipes, include:

1. **Setup steps** — endpoint configs + template
   creation + dataset creation if applicable, with
   complete copy-pasteable JSON / shell.
2. **Full script source** — verbatim, copy-pasteable.
   Don't truncate with `// ...` ellipses; the user
   should be able to paste the script into the WebUI
   editor and run it.
3. **What you'll see in the WebUI** — for the chat
   recipes, where the chat tab appears, what the
   "Test in chat" button does on TemplateDetail / list
   rows, what the empty-content probe shows on a
   fresh instance.
4. **Failure modes and how the WebUI surfaces them** —
   transport errors as toasts; the not-chat-compatible
   empty state if the output shape drifts; the embed
   dataset status badges and four UI states.
5. **Permissions required to run the recipe end-to-end**.

## Style and tone

Match the surrounding docs' tone: concise, declarative,
verifiable, no marketing fluff, no emoji, no "best
practices" boilerplate, no LLM stylistic flourishes.

Use code fences with explicit language tags
(```javascript / ```json / ```http / ```sh) — the docs
build pipeline parses them.

Cross-references use the existing markdown format
(`[link text](relative/path.md#section)`) — the lint
pipeline doesn't enforce link checking but downstream
LLM consumers will, so keep links accurate.

Keep paragraphs short (4-6 lines max). Tables for
field-shape enumerations rather than nested bullet lists.

**No invented features.** If the design docs and the code
contradict each other, the code wins; if both leave a
question unanswered, **flag it in the structured output**
rather than guessing — leaving the gap visible is better
than papering over it.

## Reconciliation requirements

The rewrite must reconcile against:

1. **Existing design docs** (07, 08, 09, 10, 16) — if
   the rewrite says X but design says Y, either align
   the rewrite to design or flag the divergence
   explicitly in the rewrite as "Note: design 07 says X
   but code in `engine.rs:NNN` does Y; following the
   code." In the structured output's residual-risks
   section, list every such divergence so Claude can
   reconcile design separately.
2. **Authoritative implementation** (the .rs / .ts /
   .toml files cited above) — these win on factual
   disputes. If they disagree with each other (e.g.,
   `engine.rs` and `embed_dataset_codec.rs` give
   different CorpusItem shapes), pick the one that's
   actually emitted by the engine and flag.
3. **HUMANS.md** — Yuka's product directives are
   load-bearing for the chat-UI behavior and the
   embedding-datasets `data` semantics. If
   implementation deviates from HUMANS.md, **the
   implementation wins** for the guide's "what's true
   today" purpose, but the divergence belongs in
   residuals so Yuka can decide whether to refactor
   the implementation back to HUMANS.md's spec or
   refactor HUMANS.md to match.

## Cross-deliverable: file scope

**Files to edit**: `docs/guide/workflow-authoring.md`
(rewrite — full overwrite is acceptable; this isn't a
diff-style update).

**Files NOT to edit** (flag if you find a reason to):

- Any other markdown file (design docs, README,
  ROADMAP, HUMANS.md, AGENTS.md, CLAUDE.md,
  CONTRIBUTING.md). If you find an inaccuracy in a
  design doc, surface it in residuals — don't fix it
  yourself in this dispatch.
- Any Rust crate, TypeScript file, or shell script.
- Any `.claude/`, `docs-jp/`, `bins/`, or `scripts/`
  content.

<structured_output_contract>
**Critical: emit this report before `task_complete`.**

Six sections, in this order:

1. **Summary** — one paragraph: what landed, what
   sections were rewritten, how the chat + embed-
   datasets focus was addressed. Include the verbatim
   string "RUN STATUS: COMPLETE" or "RUN STATUS:
   PARTIAL — <reason>" for grep.

2. **Touched files** — exhaustive list with
   `(new|edited|deleted) <path> — <one-line note>`.
   Should be exactly one entry: `edited
   docs/guide/workflow-authoring.md`. If anything else
   appears, justify why.

3. **Verification results** — exact commands + outcomes:
   - `./scripts/test-scripts.sh` — pass/fail (you
     shouldn't have touched any scripts).
   - `./scripts/check-md-bloat.sh` — pass/fail/output
     excerpt (sanity check on size).
   - Spot-check greps you ran to verify factual claims
     (e.g., `grep -n "data:" philharmonic-workflow/src/engine.rs`).
     Include the grep commands and what they confirmed.
   - Word-count and line-count of the rewritten guide
     (vs. the prior 530 lines).

4. **Residual risks / known issues** — including:
   - Every divergence between the design docs and the
     code that you encountered, with which one you
     followed and why (line numbers).
   - Every gap in the design docs / implementation that
     you couldn't fill from the cited references (and
     where you flagged the gap in the rewrite).
   - JP guide regeneration is a Claude follow-up — note
     that it is not part of this dispatch.
   - Whether you preserved any sentences verbatim from
     the prior guide (cross-reference for tone
     consistency) — note which.
   - Whether the three recipes are runnable end-to-end
     against the current API server (or whether some
     step requires a feature not yet implemented).

5. **Git state** — current `HEAD` SHA in the parent
   workspace repo. Confirm no commits made.

6. **Open questions** — questions for Yuka or Claude:
   - JP regeneration order — should Claude regenerate
     `docs-jp/ワークフロー作成ガイド.md` immediately after
     this lands, or batch with other JP updates?
   - Whether the "not invented features" rule should
     extend to flagging design-doc inaccuracies as
     follow-up Codex prompts (one per design doc).
   - Whether the guide should grow a separate
     "Troubleshooting" or "Common errors" section —
     deferred for D11 unless you can do it concisely.
   - Whether the `mechanics:*` built-in module
     enumeration is current (verify against
     `mechanics-core/ts-types/`).
</structured_output_contract>

<default_follow_through_policy>
- Read all the cited authoritative sources first, in
  the order listed. Take notes on contradictions and
  gaps; the rewrite resolves them, the residual risks
  enumerate them.
- Draft the section structure first, then fill each
  section. After each section, grep the relevant code
  / design doc to verify the claims you just wrote.
- The two recipes (chat, embedding-datasets) are
  load-bearing per the user's focus directive. Allocate
  the most space and care to them. The third combined
  recipe ties them together.
- Match the tone of the surrounding docs (design/07,
  CONTRIBUTING.md). Concise, declarative, verifiable.
  No marketing fluff, no emoji, no "best practices"
  boilerplate.
- If you find a factual inaccuracy in a design doc,
  flag it in residuals — do not edit the design doc
  in this dispatch. Design-doc reconciliation is a
  follow-up dispatch, not part of D11.
- The Japanese mirror is **out of scope**. Claude
  regenerates it after this lands. Do not touch
  `docs-jp/`.
</default_follow_through_policy>

<completeness_contract>
"Done" means:

- `docs/guide/workflow-authoring.md` rewritten,
  reflecting current implementation reality (not the
  pre-v1 / v1 reality of the prior version).
- Both load-bearing recipes (chat, embedding-datasets)
  fully written with copy-pasteable JSON + script.
- The combined recipe (chat + embedding-datasets RAG)
  written.
- Permissions table updated to include
  `embed_dataset:*` atoms.
- Connector implementations enumeration updated to
  include `custom_headers` for `llm_openai_compat`,
  and to flag `email_smtp` / `llm_anthropic` /
  `llm_gemini` as reserved/pending.
- Cross-references to design docs accurate (paths and
  section anchors).
- Structured output report emitted before
  `task_complete`.

Partial completion is acceptable if you hit a token
limit or genuine blocker — but you must say so
explicitly with "RUN STATUS: PARTIAL — <reason>".
Half-rewritten sections that contradict each other are
worse than missing sections; either revert in-progress
sections or finish them.

A run without the structured-output report is
**incomplete**, even if the guide rewrite landed.
</completeness_contract>

<verification_loop>
For every section:
1. Read the authoritative source (design doc, code).
2. Draft the section.
3. Grep-verify the specific factual claims (function
   names, field names, JSON keys, route paths,
   permission atoms).
4. Move on.

Once all sections are drafted:
5. Re-read end-to-end for consistency (same terminology
   in every section; cross-references work).
6. Run `./scripts/test-scripts.sh` (sanity).
7. Run `./scripts/check-md-bloat.sh` (size sanity).
8. Emit the structured output report.
9. Then `task_complete`.
</verification_loop>

<missing_context_gating>
If you find yourself needing information not in the
cited authoritative sources, **stop** and report what's
missing in the structured output's "Open questions"
section.

Specifically: do **not**:

- Touch any Rust crate, TypeScript file, or shell
  script. D11 is a docs-only dispatch.
- Edit any other markdown file (design docs, README,
  ROADMAP, HUMANS.md, etc.). Surface inaccuracies in
  residuals.
- Edit `docs-jp/` content. The JP mirror is Claude's
  follow-up, not Codex's.
- Invent connector config shapes for unimplemented
  Tier 2/3 connectors (`email_smtp`, `llm_anthropic`,
  `llm_gemini`). Flag as reserved/pending.
- Invent script-arg fields beyond the five
  documented (`context, args, input, subject, data`).
- Invent built-in modules beyond what
  `mechanics-core/ts-types/` declares.
- Invent permissions beyond what's in
  `philharmonic-policy/src/permission.rs` or wherever
  the atoms live.
</missing_context_gating>

<action_safety>
Files allowed to be created or modified:

- `docs/guide/workflow-authoring.md` (rewrite — full
  overwrite is acceptable).

Files NOT to touch (flag if you find a reason to):

- Any other markdown file under `docs/` (design docs,
  ROADMAP, README pointers, etc.).
- `HUMANS.md`, `CLAUDE.md`, `AGENTS.md`,
  `CONTRIBUTING.md`, any other top-level doc.
- Any Rust crate (`philharmonic-*`, `mechanics-*`,
  `bins/`, etc.).
- Any TypeScript file (`philharmonic/webui/`).
- Any shell script (`scripts/`).
- `docs-jp/` (Claude regenerates after).
- `.claude/`, `xtask/`, `Cargo.toml`, etc.

Git rules: signed-off + signed commits via
`scripts/commit-all.sh` only. **Codex never runs
`commit-all.sh`** — Claude commits after reviewing your
work. No `cargo publish`. No raw `git commit` /
`git push` / `git add` / `git reset` / `git rebase` /
`git revert`. Read-only `git log` / `git diff` /
`git show` is fine.
</action_safety>
</task>
