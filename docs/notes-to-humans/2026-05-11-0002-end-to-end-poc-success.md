# End-to-end PoC success — full chatbot RAG flow on the deployment

**Date**: 2026-05-11 (Mon) evening
**Author**: Claude Code, recording Yuka's report
**Trigger**: Yuka completed a full end-to-end chatbot use-case
flow against the live deployment and asked the docs/roadmap to be
brought up to date.

## TL;DR

Today the platform served a **complete real-world chatbot use
case** for the first time — not via integration tests, but as
a workflow created in the WebUI, executed through the production
API + mechanics + connector path, and answering a real prompt with
real retrieved context. The stack exercised:

- **Retrieval**: an `EmbeddingDataset` (indexed offline) +
  `embed` connector (BGE-M3, local ONNX via `tract` and
  `inline-blob` model bundling) + `vector_search` connector
  (stateless cosine over the dataset's corpus).
- **Relational data**: a `sql_postgres` connector endpoint
  pointed at a real Postgres instance.
- **LLM**: `llm_openai_compat` connector pointing at OVHCloud's
  Hugging Face Inference Provider endpoint, serving
  `Qwen/Qwen3-32B`. Uses D12's `custom_headers` knob for the
  HF `X-HF-Bill-To` billing header.

All in **one workflow**. All through the **production code
path** — API server → mechanics worker → connector router →
connector service → external upstreams.

## Why this matters

The platform's stated central use case (per README and design
docs) is RAG-grounded chat where:
- the vector index lives inside the platform (privacy / locality);
- a relational DB participates alongside the vector store;
- the LLM is run by the operator or a chosen partner.

Until today this had been **stated** and **integration-tested**
but never **demonstrated end-to-end against an actual user-style
prompt on a real deployment**. The PoC closes that gap. The
"local-or-partner-hosted RAG chatbot" pitch in `docs-jp/`
executive summaries is now describable in present tense.

It also serves as a regression anchor: any future change that
breaks this flow is breaking something we know real users would
care about.

## What the PoC also surfaced (already fixed)

The same session that validated the flow also produced four
deployment-time fixes earlier the same day, all already landed:

- **`mechanics-core` 0.3.2 → 0.4.0** (`5cbe72c` submodule +
  `6ed5ee2` parent) — runtime no longer overrides
  main-fulfilled success with engine-side "Unhandled promise
  rejection" errors. The PoC's workflow uses `try { await ... }
  catch { }` to fall back across providers; without 0.4.0 every
  caught rejection would have killed the step.
- **WebUI permission-aware nav + disabled non-actionable buttons
  + sticky sidebar footer** — surfaced because the PoC was being
  driven from a `Tenant Admin` role with a clipped envelope; the
  WebUI was showing buttons that would 403 on click. Fixed via
  `philharmonic-api` 0.1.8 `WhoamiResponse.permissions` +
  `usePermissions` hook + per-button `disabled` state.
- **Audit-log producer gap closed** — the PoC made the empty
  Audit Logs page very visible; producer wiring landed across 7
  route files with privacy-restricted token-mint payloads. See
  [`2026-05-11-0001-audit-event-producer-gap.md`](2026-05-11-0001-audit-event-producer-gap.md).
- **Connector body cap raised 2 MiB → 32 MiB**
  (`philharmonic-connector-router` 0.1.2) — the retrieved
  corpus chunks made the per-request payload exceed the old cap
  in some queries.

## What's next (not blocking)

The PoC working does not change the post-v1 dispatch plan:
**D7 / D8 / D9** (SMTP, Anthropic, Gemini connector
implementations) remain the named remaining Codex dispatches.
The PoC demonstrates the architecture is sound enough that
those three are straightforward extensions of the connector
pattern, not blocked by anything in the substrate.

What might be worth doing post-PoC, if Yuka wants:

- A **PoC walkthrough doc** that records the exact workflow
  script, the endpoint shapes, and the captured request/response
  cycle, as a reference recipe for partners. The workflow-authoring
  guide's "combined RAG recipe" is close but doesn't currently
  show OVHCloud-HF dialect specifics or `tool_choice` tuning.
- **HF dialect-selection subsection** in
  `docs/guide/workflow-authoring.md` (offered earlier; now there's
  concrete experience to pull from — Yuka noted "structured
  outputs works with openai_native for some provider/model
  combinations on HF Inference Providers, with the custom header
  support previously added, although some don't work" earlier
  today).

Neither is urgent. Surface here so they don't get lost.

## Memory and persistence

Per [`CLAUDE.md` §"Memory / persistence policy"](../../CLAUDE.md),
nothing in this note is going into machine-local memory — the
PoC outcome and what it implies for the workspace belongs in
the repo (this note, `README.md`'s Status section,
`ROADMAP.md`'s preamble, the docs-jp summary for today).
