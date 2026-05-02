# Full-docs-audit fixes (2026-05-02)

Codex produced two audit reports:

- `docs/codex-reports/2026-05-02-0001-embedding-datasets-design-audit.md`
  — review of `docs/design/16-embedding-datasets.md`.
- `docs/codex-reports/2026-05-02-0002-full-docs-audit.md` — full
  sweep of `docs/`, `README.md`, `AGENTS.md`, `CONTRIBUTING.md`,
  workflow authoring guide (EN + JP), and crate-level docs.

Plus three errata items added to `HUMANS.md`'s Embedding DB
section: deterministic CBOR storage, `LONGBLOB` migration,
friendly UI (no raw JSON editor).

I worked through both reports + the errata in three commits and
pushed; what follows is what was actually wrong and what
landed.

## Commits

- `efff032` — batch 1: embedding-datasets design fixes,
  TenantEndpointConfig model coherence, workflow guide
  EN+JP, design 06 rewrite.
- `75db7f2` — batch 2: design 03 + 06 versionless,
  drop design 00-index, MSRV 1.89 exceptions, placeholder
  vs substantive language, design 15 historical path,
  per-minting-authority rate limiting, `/v1/whoami` +
  operator routes + `PATCH workflows/templates/{id}`.
- `be9cd07` — batch 3: link rot, fixture READMEs, version
  pins removed from 04/07, `TenantCredential` →
  `TenantEndpointConfig`, Six → Seven layers.

## Embedding datasets (design 16)

What was wrong, per the embedding-datasets audit + your
errata:

- **Storage**: source-items + corpus stored as JSON content
  blobs would blow past `MEDIUMBLOB`'s 16 MB cap with even
  modestly-sized embedded corpora.
- **Per-job timeout**: claimed on `MechanicsJob`, which has
  no such surface; only `MechanicsPoolConfig.run_timeout`
  exists today.
- **Lowerer changes "required" and "denied"**: the design
  introduced `LowerScope { Step, Ephemeral }` for the
  ephemeral embed job, but later said "Lowerer: no
  changes." The change is also crypto-sensitive (`inst`,
  `step`, AEAD AAD).
- **`max_batch_size`**: said to be read by the embed script
  from "the lowered config" — but the connector
  implementation's private config is inside COSE and not
  visible to JS.
- **Permission grouping**: `embed_dataset:*` was suggested
  for the `deployment` group; deployment is operator-scope,
  embedding datasets are tenant resources.
- **Source Items WebUI tab**: documented but had no
  backing endpoint.
- **Concurrency**: design first said "queue updates," then
  said "reject with 409 in v1" — pick one.
- **WebUI**: raw JSON editor pattern, which doesn't scale
  to corpus-shaped data (per your errata).
- **Encoding density**: should use deterministic CBOR
  (RFC 8949 §4.2.1), not JSON, for content slots.

What I changed:

- Added a "Storage substrate prerequisites" section
  documenting the required `MEDIUMBLOB → LONGBLOB`
  migration in `philharmonic-store-sqlx-mysql` (auto-applied
  on startup; SQLite unaffected) and explicit hard limits
  (10k items, 64 KiB per text/payload, 256 MiB source-items
  blob, 1 GiB corpus blob, all operator-tunable).
- Switched content slots to deterministic CBOR
  (RFC 8949 §4.2.1).
- Per-job timeout: documented as a `mechanics-core` change
  — optional `run_timeout` override on `MechanicsJob`. The
  earlier "no mechanics worker change" claim was incorrect
  and is removed.
- Lowerer integration: kept `LowerScope` enum as
  Approach A but explicitly flagged the change as
  crypto-sensitive and **gated on Yuka's two-gate review
  before any code lands**. Recommended **Approach B** for
  v1 (synthesize a non-persisted instance UUID, no
  trait/AEAD shape change) as the lower-risk path. Either
  way, Gate 1 must clear first. Removed the "Lowerer: no
  changes" claim.
- `max_batch_size`: API server reads it from the decrypted
  endpoint config (it has SCK) and passes it in the embed
  job's `arg`, e.g.
  `{items, max_batch_size: 32}`. JS never has to read
  connector-private config.
- Permissions: kept `embed_dataset:*` prefix (own group in
  WebUI) — tenant resources don't go in the
  `deployment:*` group; the existing prefix-based grouping
  in `permissions.ts` falls out cleanly.
- Added `GET /v1/embed-datasets/{id}/source-items` to back
  the WebUI Source Items tab.
- Concurrency: chose 409-on-update only; queuing left as a
  future enhancement.
- WebUI: rewrote to a structured table editor with paste
  import, per-row validation, collapsed/expandable vector
  display, and explicit caps surfaced in-form.
- Updated implementation order to put the substrate
  migration and per-job timeout knob first, and added
  Gate 1 as step 0.
- Added an explicit "Crypto-review note" naming the
  affected token claims and AAD inputs.

## TenantEndpointConfig model contradiction

What was wrong, per audit finding 1: docs 08 / 10 / 11 had
moved to the current model (`implementation` as plaintext
content slot, lowerer assembles `{realm, impl, config}`)
but **docs 09 / 14 / 15 still described** the older
"whole-blob includes realm and impl, lowerer is a
byte-identical re-encryptor" model. This isn't cosmetic —
substrate privacy and crypto-bound payload semantics
differ.

What I changed:

- **Doc 09 (`philharmonic-policy`)**: rewrote
  `TenantEndpointConfig` entity definition to include the
  `implementation` content slot, rewrote "The encrypted
  blob" → "The encrypted config blob" (config-only),
  rewrote "The lowerer's policy consultation" as a 13-step
  payload-assembly sequence (read plaintext
  `implementation` → resolve realm via deployment's
  connector-router map → decrypt SCK blob → assemble
  `{realm, impl, config}` → COSE_Encrypt0 to realm KEM).
- **Doc 14 (open questions)**: changed the "Lowerer
  transformation: pure byte forwarding" entry to "payload
  assembly," with a back-reference to doc 09.
- **Doc 15 (v1-scope)**: updated the connector-layer
  "Ships with v1" to describe the lowerer as a payload
  assembler and updated `TenantEndpointConfig` minimal
  shape.

I left the historical ROADMAP Wave-plan text alone — it
records what we planned at that point in time and the
audit didn't ask for that to be edited.

## Workflow authoring guide (EN + JP)

What was wrong, per audit findings 2 + 3:

- **HTTP example**: `url_template` had a `{resource}`
  placeholder but no matching `url_param_specs.resource`
  entry — current validation rejects this.
- **LLM examples** (two places in EN, two in JP): called
  `llm_openai_compat` with only `model + messages` and
  read `response.body.choices[0].message`. The connector
  layer requires `output_schema` and returns a normalized
  `{output, stop_reason, usage}` shape. A user following
  the guide through the WebUI would build a workflow that
  fails connector validation or throws on a missing
  `choices` field.

What I changed:

- HTTP example (EN + JP): added
  `"url_param_specs": { "resource": {} }` and a one-line
  note explaining the rule.
- LLM examples (EN + JP, both occurrences each): added
  `output_schema`, switched to reading
  `response.body.output.<field>`, and added a callout
  distinguishing the transport envelope (`body / headers /
  status / ok`) from the LLM-normalized response shape.

The Japanese guide's full rewrite remains a Codex task per
HUMANS.md; what landed here is just the bug fix to the
specific examples called out in the audit. The submodule
commit is in `docs-jp` and the parent picks up the
submodule pointer.

## Design 06 (execution substrate)

What was wrong: opening said `mechanics-config` "doesn't
exist yet — extraction is pending," and the doc carried a
"migration plan" + "Status: extraction pending" tail.
Crate has been published for weeks.

What I changed: rewrote opening + "Schema extraction" +
"Status" sections as current-state. The split that landed
is described as shipped; the rationale for the extraction
is preserved as historical context, not as a TODO.

## Design 03 (crates and ownership) — full refresh

What was wrong: many old version pins (`v0.3.4`, etc.);
described `connector-client` as "the lowerer" (it isn't —
it's crypto primitives only); described `connector-service`
as hosting the `Implementation` registry (it doesn't —
dispatch lives in the deployment binary); listed
`philharmonic`, `philharmonic-api`, and the deferred
Tier 3 impl crates as "no crates.io presence" (they're
all on crates.io now).

What I changed: rewrote the whole crate-listing section as
versionless ("use `./scripts/crate-version.sh --all`"),
fixed the `connector-client` description (crypto
primitives, no policy/store deps, lowerer lives in API
server bin), fixed the `connector-service` description
(framework only, no Implementation registry), introduced
"Substantive vs Placeholder" terminology for the Tier 2/3
crates that exist as `0.0.x` placeholders, refreshed the
dependency graph to match current reality, and added a
"Naming history" subsection.

## Dropped design/00-index.md

You noticed it was invisible from the mdBook page (not in
SUMMARY.md). Two options: add to SUMMARY or merge into
`docs/README.md`. I chose merge.

Why: 00-index's "Files" section duplicated SUMMARY.md, its
"Status overview" duplicated the workspace `README.md` and
went stale fast (the audit's finding 14 caught two of
those drifts), and its "Status of the docs published at
github.io" section was a historical paragraph nobody is
acting on.

What I changed: rewrote `docs/README.md` to incorporate
the genuinely-useful "How to read this book" navigation
aid (file order for big-picture vs architectural-depth vs
crypto vs open-questions reading paths) and deleted
`docs/design/00-index.md`. References to 00-index in live
docs were already absent; remaining references are in
historical codex-reports / notes, which the audit
explicitly says not to touch.

## MSRV 1.89 exceptions

What was wrong: docs (CONTRIBUTING.md, README.md,
ROADMAP.md, design 03) all said "MSRV 1.88, mirrored in
each manifest" — but `inline-blob` and
`philharmonic-connector-impl-embed` declare 1.89.

What I changed: documented the exception explicitly in
all four places. Workspace baseline 1.88; two named
exceptions at 1.89; new crates default to 1.88.
Workspace-wide bump remains the right long-term answer.

## Phase / status language re placeholders

What was wrong: README.md said "no crates.io presence (no
0.0.0 placeholders were reserved)," but the deferred
Tier 3 crates *are* on crates.io as `0.0.x` placeholders.
ROADMAP §8 said "all 25 crates published at 0.1.0 or
higher" — the three placeholder crates are at `0.0.x`.

What I changed: introduced a clear two-term split in
README.md (substantive `0.1.0+` vs placeholder `0.0.x`),
updated Phase 9 status text accordingly, and rewrote
ROADMAP §8 v1 definition-of-done.

## Design 15 critical path

What was wrong: the file declared Phase 9 complete at the
top but then carried an imperative "Critical v1 path" with
items like "extract `mechanics-config`" and "claim
remaining crate names as 0.0.0 stubs" — long since done.
Reading it as live guidance was actively misleading.

What I changed: replaced the "Critical v1 path" section
with two sections: a short "Current state (Phase 9
complete, 2026-05-02)" summary that points at README.md /
ROADMAP.md as the authoritative status, and a
"Historical v1 implementation path" that preserves the
original sequencing as context with each step marked
done / deferred where applicable.

## Per-minting-authority rate limiting

What was wrong: docs 09 / 14 / 15 all said "deferred";
doc 10 said "in v1"; the code (`rate_limit.rs`) keys on
`(tenant, minting_authority)`. Internally inconsistent.

What I changed: docs 09 / 14 / 15 now describe per-minting-
authority rate limiting as in-v1 (matching the code +
doc 10), with code-file references for traceability.
Removed the now-incorrect entry from doc 14's
"Simplifications committed by deletion" list. Updated
doc 15's "Doesn't ship with v1" entry to be about
distributed (cross-node) rate limiting, which is the
actually-deferred piece.

## Live routes missing from doc 10 / 09

What was wrong: doc 10 didn't list `/v1/whoami` or the
`/v1/operator/tenants/*` routes; doc 09's permission-to-
endpoint table omitted the meta endpoints, `/v1/whoami`,
operator routes, and `PATCH /v1/workflows/templates/{id}`.

What I changed:

- Doc 10: added an "Identity introspection" section
  documenting `/v1/whoami` and a full operator-tenant
  routes block under "Tenant administration."
- Doc 09: extended the permission table with meta
  endpoints (marked "public — no auth"), `/v1/whoami`
  (marked "authenticated, no permission required"),
  `PATCH /v1/workflows/templates/{id}` (gated on
  `workflow:template_create`), and the five operator-
  tenant routes (gated on `deployment:tenant_manage`).

## Link rot

What was wrong (representative; full list in audit
finding 11):

- README.md pointed at `docs/01-project-overview.md`
  (file is `docs/design/01-project-overview.md`).
- AGENTS.md linked to `POSIX_CHECKLIST.md`; file is
  `docs/POSIX_CHECKLIST.md`.
- ROADMAP.md (which lives in `docs/`) linked to
  `CONTRIBUTING.md`, `docs/codex-prompts/...`, etc. — all
  resolving wrong from the `docs/` directory.
- Crypto proposal under `docs/crypto/proposals/` linked
  `../09-...` (resolves under `docs/crypto/`, not
  `docs/design/`).
- Test fixture READMEs linked `../../../ROADMAP.md`
  (resolves to `tests/ROADMAP.md`).

What I changed: fixed each one to a working relative path.
For ROADMAP.md, used a `sed` pass to bulk-rewrite the
recurring `docs/...` and `CONTRIBUTING.md` patterns.
Crypto proposal links now point at `../../design/0X-...`.
Test fixture READMEs and the openai-compat submodule's
fixtures README now use `../../../docs/ROADMAP.md` and
`tests/fixtures/upstream/` paths.

I did **not** edit historical codex-prompts or
notes-to-humans even where they contained stale paths —
the audit explicitly says to leave those alone (they're
records of what was true at the time).

## Misc

- **`TenantCredential` → `TenantEndpointConfig`** in doc
  12 (it referenced an entity name that has never existed
  in code).
- **Six → Seven layers** in ROADMAP §1: the section said
  "Six layers" then enumerated seven (1–7).
- **Version pins removed from docs 04, 07**: replaced with
  "use `./scripts/crate-version.sh --all`" — exact
  versions in component design docs go stale on every
  release and offer nothing useful in exchange.
- **Workflow guide note on `docs-jp` rewrite**: full
  re-write of the JP guide remains a Codex task per
  HUMANS.md. What landed is the bug fix to specific
  examples, not the rewrite.

## What's still pending (not addressed by this pass)

From audit finding 11 (link rot): I did not add an
automated Markdown link checker to CI. That's listed as a
recommendation but is its own task; flag if you want me to
take it.

From the embedding-datasets audit: nothing structural
remains in the design — the doc is now coherent and
implementable. Implementation itself is gated on the
crypto Gate 1 proposal (Approach A vs B) before any Codex
prompt is written.

From HUMANS.md (workflow authoring guide rewrite): still
deferred to Codex. The audit's spot-fixes to the broken
examples are now in, so users following the current guide
won't hit failing workflows in the meantime.
