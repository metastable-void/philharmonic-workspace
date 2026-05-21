# Philharmonic Implementation Roadmap

**Audience**: Claude Code, working in the `philharmonic-workspace`
repo.

**Purpose**: The authoritative implementation plan for Philharmonic.
V1 is complete; active work lives in the post-v1 dispatch plan
(§3 below). This file describes the **current state and what is
needed next** — past dispatches and closed arcs are no longer
enumerated here. Per-arc done-state snapshots live under
[`docs/archive/`](archive/) and per-crate release notes live in
each crate's `CHANGELOG.md`.

## Current status

- **v1 path: complete through Phase 9.** Reference deployment
  operational since 2026-05-02; the RAG-grounded chat use-case
  (embedding-dataset + `embed` + `vector_search` + `sql_postgres`
  + `llm_openai_compat`) is verified end-to-end against it.
- **Post-v1 internal work closed.** Embedding-datasets, WebUI
  infrastructure, connector enhancements, runtime semantics,
  module-surface refactor, HTTP-client transport + TLS posture,
  workspace tooling, HTTP/3 client + server, production-security
  dep cleanup, `mechanics-dns` extraction, the §3.K Audit &
  refactor sweep, and the §3.B Tier-2 connector batch (D7 SMTP
  + D19 DNS, both at `0.1.0` as of 2026-05-18) have all landed.
- **§3.M Production Chat UI: landed 2026-05-21** as the
  in-tree `bins/philharmonic-chat/` single-bin HTTPS+H3 axum
  server with an embedded React+Redux frontend bundle.
  Backend body + full frontend dispatched in one Codex round
  (commit `64a1fe6`); subsequent fix-forward commits closed
  `/mint-ephemeral` agent-token gating. The bin's local
  `README.md` is the design home; this section's residual
  text below is kept as historical context for the design
  choices, not as an open work item.
- **Open (post-MVP, deferred)**: §3.B Tier-3 LLM connector
  implementations — D8 Anthropic and D9 Gemini. **Not
  required for MVP deployment** (`llm_openai_compat` covers
  the OpenAI / vLLM / compatible-gateway shape that the MVP
  needs). Specs in §3.B below are kept for whenever they
  become MVP+1 priorities; no dispatch is planned in the
  current cycle.
- **Deferred (not yet scoped)**: a crypto-review-aware slice
  for the `lowerer.rs` / `embed_job.rs` extraction that was
  originally considered under §3.K but moved out because it
  touches SCK encrypt/decrypt and endpoint-payload handling.
  Will be sequenced once its Gate-1 proposal is ready.

Authoritative sources for things this file cross-references:

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

## 1. v1 milestone archive (pointer)

The full pre-trim plans, definition-of-done, and completed-
crate inventory for Phases 0–9 (workspace setup through
reference deployment) are preserved verbatim at
[`docs/archive/2026-05-10-readme-roadmap-trim.md`](archive/2026-05-10-readme-roadmap-trim.md)
(under "Pre-trim `docs/ROADMAP.md`" → §4 "Completed v1 Milestone
Archive" and §8 "Definition of done for v1"). Historical
implementation detail also lives in dated
`docs/codex-prompts/`, `docs/codex-reports/`,
`docs/notes-to-humans/`, and `docs/crypto/{proposals,approvals}/`.

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

Each numbered item below is one Codex dispatch with its own
archived prompt under `docs/codex-prompts/` (see
[`.claude/skills/codex-prompt-archive/SKILL.md`](../.claude/skills/codex-prompt-archive/SKILL.md)).

**Closed arcs** (§3.A, C–L): per-arc done-state snapshots live
under [`docs/archive/`](archive/) — per-day archives span
2026-05-10 through 2026-05-18. They are not enumerated here.

### B. Phase 7 Tier-3 LLM connectors (2 dispatches — deferred out of MVP)

Tier-2 batch (D7 SMTP + D19 DNS) shipped 2026-05-18 at
`0.1.0` and is recorded with the other closed arcs. What
remains here is **Tier-3 only**: D8 Anthropic + D9 Gemini.

**MVP status (decided 2026-05-18):** Neither D8 nor D9 is
required for MVP deployment. `llm_openai_compat` already
covers the OpenAI / vLLM / compatible-gateway shape that
the first use case (customer-support chat, §3.M) needs.
The specs below are preserved verbatim so that whenever
D8 / D9 become MVP+1 priorities the dispatch can proceed
without re-deriving the shape — they are **not** queued
for the current cycle. Neither touches the crypto path;
both take an existing `0.0.x` placeholder to a `0.1.0`
substantive impl by implementing the
`Implementation` trait against an already-decrypted
`config` + `request`.

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

D8 and D9 are independent of one another; safe to run in
parallel whenever they get queued. Neither was a §3.M
prerequisite (which has already landed; see below).

### M. Production Chat UI (landed 2026-05-21)

The first production deployment use case is a
**customer-support chat** product, distinct from the
`philharmonic/webui/` chat surface that exists for
workflow-author end-to-end testing.

**Architectural constraint (decided in HUMANS.md, confirmed
2026-05-18):** the framework is **not** a chat app. Chats
are workflow knowledge (per
[§02 *Layered ignorance*](design/02-design-principles.md#layered-ignorance));
the framework's crate family must not gain chat-app
concepts. The production Chat UI therefore lives in its own
top-of-tree codebase: **`bins/philharmonic-chat/`**, in-tree
single-bin, that consumes `philharmonic` via the
`server-https` feature like any other consumer. Framework
crates know nothing about chat-app concepts (sessions,
conversation history UI, agent personas, customer-tenant
mapping, support-queue routing, etc.); the chat bin consumes
the framework's primitives (workflows, instances,
`llm_generate` via `llm_openai_compat`, the embedding-dataset
path for RAG grounding) and adds its own concerns on top.

**Shape that landed (2026-05-21):**

- In-tree single-bin musl-buildable HTTPS+H3 axum server
  (mirrors `bins/philharmonic-api-server`'s `https` feature
  pattern).
- Embedded React+Redux frontend bundle from
  `bins/philharmonic-chat/dist/`, served at `/` with SPA
  fallback. Build wrapper:
  `scripts/philharmonic-chat-build.sh --production`.
- HTTP surface: `/` (static assets), `/config` (returns
  `{ api_url, notify_instance_uuid }`), `/sign-in`
  (constant-time `agent_token` challenge),
  `/mint-ephemeral` (creates a chat-template instance and
  mints an instance-scoped ephemeral token; requires
  `Authorization: Bearer <agent_token>` since 2026-05-21),
  `/version`.
- Three tokens in `[chat]` config: `agent_token` (support
  agent's sign-in token), `service_token` (Principal token
  the bin uses to create instances), `minting_token`
  (MintingAuthority token the bin uses to mint
  ephemerals). The earlier `admin_token` naming is retired.
- Release-build wrapper: `scripts/chat-release-build.sh`
  mirrors `scripts/release-build.sh` for the chat bin alone
  (single binary, separate `philharmonic-chat-<sha>.tar.*`
  archive name).
- Design home: the bin's own
  [`README.md`](../bins/philharmonic-chat/README.md), not
  `docs/design/`.

**Existing testing-grade Chat UI** at `philharmonic/webui/`
stays in place as the workflow-author end-to-end test
surface. The two coexist.

**Out of scope for v0 (recorded in the bin's README and
Codex round-01 report at
[`docs/codex-reports/2026-05-21-0002-philharmonic-chat-bin-backend-and-frontend.md`](codex-reports/2026-05-21-0002-philharmonic-chat-bin-backend-and-frontend.md)):**
rate-limiting / abuse mitigation on `/mint-ephemeral`,
multi-tenancy (one bin = one tenant), agent assignment /
claim / presence indicators, HUMAN → AI transition, the
future EC embed widget, i18n. These are open follow-ups,
not regressions.

### Suggested sequencing

**Next dispatchable**: open. §3.M's in-tree
`bins/philharmonic-chat/` round 01 has shipped (2026-05-21);
follow-on rounds (rate-limiting on `/mint-ephemeral`,
HUMAN → AI transition, EC embed widget, multi-tenancy) are
deferred and not yet scoped. The crypto-review-aware
`lowerer.rs` / `embed_job.rs` slice noted in §"Current
status" / Deferred remains queued; D8 / D9 are post-MVP.

**Post-MVP, no dispatch queued**: D8 + D9 (§3.B Tier-3 LLM
connectors). When queued, both are independent and
parallel-safe.

- **D8** (`philharmonic-connector-impl-llm-anthropic`,
  Tier 3) — fully spec'd from
  [`docs/design/08-connector-architecture.md` §llm_anthropic](design/08-connector-architecture.md#llm_anthropic--config);
  prompt draft ready.
- **D9** (`philharmonic-connector-impl-llm-gemini`, Tier 3)
  — dual-mode AI Studio + Vertex AI; Claude proposes the
  discriminator field, Vertex-mode field names, and OAuth2
  access-token caching strategy in the prompt; Yuka
  overrides at prompt-review time if she has a preference.

### Dispatch discipline reminder

Per [`.claude/skills/codex-prompt-archive/SKILL.md`](../.claude/skills/codex-prompt-archive/SKILL.md)
and [`CONTRIBUTING.md §15.2`](../CONTRIBUTING.md#152-codex-prompt-archive):
every Codex prompt is archived to
`docs/codex-prompts/YYYY-MM-DD-NNNN-<slug>.md` and committed
**before** the spawn. Each dispatch above corresponds to one
archived prompt; multi-round work shares the daily-sequence
`NNNN` and increments the trailing `-NN`.
