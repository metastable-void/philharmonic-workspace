# Philharmonic — Design Summary Index

This directory captures the design of the Philharmonic project as
it currently stands. Files are organized by topic. Each file mixes
**settled decisions** with **open questions** and explicitly marks
the difference.

## Files

- `01-project-overview.md` — what the system is, what problem it
  solves, the three-subsystem architecture (storage substrate,
  execution substrate, connector layer) plus the workflow layer
  that orchestrates them, and the deployment topology.
- `02-design-principles.md` — cross-cutting commitments:
  append-only, content-addressed, backend-agnostic, layered
  ignorance, defer until concrete, and others.
- `03-crates-and-ownership.md` — the published/claimed/to-be-
  claimed crate namespace, the dependency graph including the
  connector split into four crates and the `mechanics-config`
  extraction, licensing, versioning.
- `04-cornerstone-vocabulary.md` — `philharmonic-types`: the
  shared vocabulary crate anchoring the workspace.
- `05-storage-substrate.md` — `philharmonic-store` and
  `philharmonic-store-sqlx-mysql`: the append-only, EAV-shaped
  storage layer.
- `06-execution-substrate.md` — `mechanics-config`,
  `mechanics-core`, and `mechanics`: stateless JS execution via
  Boa, HTTP service, and the schema extraction keeping the
  lowerer Boa-free.
- `07-workflow-orchestration.md` — `philharmonic-workflow`:
  templates, instances, step records, the lifecycle state
  machine, the four-field script argument
  (`{context, args, input, subject}`), subject context threading
  through engine methods.
- `08-connector-architecture.md` — the connector layer split
  into `connector-common`, `connector-client` (lowerer),
  `connector-router` (pure dispatcher), and `connector-service`
  (framework), plus per-implementation crates. COSE-based wire
  format, realm-scoped hybrid PQC KEM encryption, the v1
  capability set.
- `09-policy-and-tenancy.md` — `philharmonic-policy`: tenants
  (flat for v1), principals, customizable roles and role
  memberships, tenant credentials encrypted at rest, tenant
  capability authorizations, minting authorities that issue
  ephemeral API tokens, audit events.
- `10-api-layer.md` — `philharmonic-api`: HTTP API, long-lived
  and ephemeral token authentication, the minting endpoint,
  workflow and credential management, principal and role
  management, rate limiting.
- `11-security-and-cryptography.md` — threat model,
  cryptographic primitives (ML-KEM-768 + X25519 + AES-256-GCM,
  Ed25519, SHA-256, COSE formats), the three token systems
  (connector authorization, ephemeral API, long-lived API),
  key management, rotation, blast-radius analysis.
- `12-deferred-decisions.md` — features explicitly out of scope
  for v1, with rationale for each deferral and notes on how the
  architecture keeps options open.
- `13-conventions.md` — workspace-wide practices: naming,
  versioning, Rust edition, MSRV, CI, licensing.
- `14-open-questions.md` — decisions still pending, organized by
  urgency. Resolved questions moved to the "already answered"
  section for reference.
- `15-v1-scope.md` — what a v1 release includes, what's deferred,
  the critical path with parallelism opportunities.

## How to read this

**For the big picture:** start with `01-project-overview.md`,
then `15-v1-scope.md` to see what shipping looks like.

**For architectural depth:** after the overview, read
`02-design-principles.md`, then the layer-by-layer docs
(`04` → `05` → `06` → `07` → `08` → `09` → `10`).

**For concrete open decisions:** `14-open-questions.md`
aggregates everything pending.

**For cryptographic specifics:** `11-security-and-cryptography.md`
consolidates the design across token systems, encryption
systems, and key management.

**For reference on a specific component:** each component has
its own topic file.

## Status overview

Published and stable:

- `philharmonic-types` (cornerstone)
- `philharmonic-store` (substrate traits)
- `philharmonic-store-sqlx-mysql` (SQL backend)
- `mechanics-core` (JS executor library)
- `mechanics` (JS executor HTTP service)

Designed, not yet implemented:

- `mechanics-config` (extraction pending)
- `philharmonic-policy`
- `philharmonic-workflow`
- `philharmonic-connector-common`
- `philharmonic-connector-client`
- `philharmonic-connector-router`
- `philharmonic-connector-service`
- Per-implementation crates (one crate each, named
  `philharmonic-connector-impl-<n>`)
- `philharmonic-api`

Remaining items blocking v1, per `14-open-questions.md`:

- Per-implementation wire-protocol details — the exact
  request/response JSON shapes for each v1 implementation
  (`llm_generate`, `http_forward`, `sql_query`, `email_send`,
  `embed`, `vector_search`), to be sketched against the first
  impl and iterated.

The permission atom vocabulary in `09-policy-and-tenancy.md`
is treated as closed for v1 and adjusted deliberately if
implementation reveals a concrete need.

Other items (multi-region, OTLP export, authoring patterns
documentation, stateful-connector concerns, admin UI form
rendering) are non-blocking for v1. See `14-open-questions.md`
for the full list and the resolved-questions archive.

## Status of the docs published at metastable-void.github.io/philharmonic

A separate set of public-facing docs was drafted previously and
published at `metastable-void.github.io/philharmonic`. That set
covers `00-overview.md`, `01-principles.md`, `02-vocabulary.md`
(renamed to `02-01-cornerstone.md`), `03-storage.md` (renamed
to `02-02-storage.md`), `04-execution.md` (renamed to
`02-03-execution.md`), `05-workflow.md` (renamed to
`02-05-workflow.md`), `06-boundaries.md`, `07-deferred.md`, and
`08-conventions.md`. A renumbering to a two-level scheme was
proposed (`02-00-components.md` as a new index, with per-
component docs as `02-NN-*.md`), and revised versions of
`00-overview.md` and new `02-00-components.md` were drafted.

The connector doc (`02-04-connectors.md`) was never drafted
because architectural decisions kept shifting. The summary
files in this directory now capture those architectural
decisions as settled, so the public connector doc can be
written based on `08-connector-architecture.md` plus the
related cryptographic and policy docs.
