# Deferred Decisions

Features considered and deliberately not in v1. Each has a
rationale for deferral and a note on how the architecture keeps
the door open.

Organized by concern for navigability. The list is meant to stay
short and visible, not hide features from view. Each entry was
considered and judged not yet worth implementing, with a path
for when implementation becomes needed.

The deferral discipline is uniform: the crates ship with the
smallest surface that meets concrete, current consumer need.
Features justified only by "some future consumer might want it"
stay out until a concrete consumer materializes — shipping
speculatively locks in choices that first adopters should drive.

## Execution and workflow

### Determinism in execution

**What it would be**: seeded `Math.random`, frozen `Date.now`,
I/O interception. Combined with append-only storage, enables
replay.

**Why deferred**: substantial implementation work. Append-only
storage already captures full history for post-hoc diagnosis;
replay isn't needed for that. Replay becomes important for
deterministic testing, A/B comparison of script changes,
forensic recreation — none pressing yet.

**Door stays open**: `mechanics-core` can grow optional job
parameters (`rand_seed`, `time_freeze`) without breaking existing
callers. The workflow engine can record them in step records.
Replay tooling reconstructs jobs with recorded values.

### Multi-step orchestration in the engine

**What it would be**: engine automatically advances multi-step
workflows without external triggering.

**Why deferred**: coupling the engine to completion semantics.
Current single-step model lets external triggers (API requests,
schedulers) drive the loop.

**Door stays open**: a future scheduler crate can sit above the
engine, driving `execute_step` based on deployment-specific
conventions. No engine changes needed. Such a scheduler should
itself stay vendor-neutral — pluggable against external systems
(cron, Temporal, Airflow, cloud scheduler services) rather than
tied to any one of them.

### Retry policy

**What it would be**: engine automatically retries failed steps.

**Why deferred**: retry policy is deeply context-dependent
(transient network vs. script bug, idempotent vs. side-
effecting, rate limits). Baking one policy in is wrong; making
it configurable adds complexity.

**Door stays open**: engine returns structured errors; callers
(or wrapping crates, or a future policy layer) implement their
own retry logic.

### Auto-advancement conventions

**What it would be**: workflow engine detects "advance me"
signals from scripts (e.g., `next_input` field in step output)
and automatically continues.

**Why deferred**: a scheduler concern, above the engine. Keeping
engine narrow.

**Door stays open**: scheduler crate can layer on top, using
existing `execute_step` plus inspection of step results.

### Cross-instance coordination

**What it would be**: workflow-engine support for waiting on
other instances, fanout, join.

**Why deferred**: coordination patterns vary; designing one
speculatively constrains consumers.

**Door stays open**: possibly connectors for spawning workflows
recursively.

## Storage substrate

### Tenant scoping in the substrate

**What it would be**: first-class tenant awareness in storage
traits.

**Why deferred**: tenancy is policy, not storage. Adding tenant
fields to substrate methods would entangle layers.

**Door stays open**: tenant references as entity slots on
workflow entities (`WorkflowTemplate.tenant`,
`WorkflowInstance.tenant`); substrate's existing
`find_by_scalar` and `list_revisions_referencing` support
tenant-filtering queries.

**Status note**: the substrate stays tenant-naive; the
`philharmonic-policy` crate ships with v1 and owns the tenant
vocabulary. See `09-policy-and-tenancy.md`.

### In-memory storage backend

**What it would be**: `philharmonic-store-mem` for testing
without MySQL.

**Why deferred**: SQL backend works; testcontainers-based
testing is functional if slow. Mem backend is quality-of-life,
not blocker.

**Door stays open**: substrate trait surface is backend-agnostic;
mem backend is a sibling crate. Name is reserved.

### Postgres backend

**What it would be**: `philharmonic-store-sqlx-pgsql`.

**Why deferred**: no current consumer asking for it. LCD MySQL
covers four MySQL-family targets. A second SQL backend doubles
maintenance surface for marginal reach absent a concrete driver.

**Door stays open**: trait surface is unchanged; new backend is
a sibling crate.

### Universal tombstone flag

**What it would be**: built-in `is_deleted` scalar on every
entity with substrate-enforced deletion semantics.

**Why deferred**: different entity kinds have different deletion
semantics. Templates retire; instances cancel; credentials
revoke; minting authorities have epochs — each different.
Universal flag would impose wrong model on some.

**Door stays open**: per-kind deletion scalars (`is_retired`,
`status: Cancelled`, etc.) are already the convention. Shared
patterns can be pulled into a common helper later if many kinds
converge.

### Read replicas / connection routing

**What it would be**: substrate support for primary/replica
routing.

**Why deferred**: `ConnectionProvider` trait already supports
this in principle. Actual implementation requires deployment-
specific logic (session affinity for read-your-own-writes),
which belongs in the consumer's `ConnectionProvider` impl, not
in the substrate crate.

**Door stays open**: custom `ConnectionProvider` implementations
can route however they need. The trait split (`acquire_read`
vs. `acquire_write`) is the extension point.

### Pagination on substrate queries

**What it would be**: cursor-paginated query methods.

**Why deferred**: current queries return small result sets
(scoped by tenant, template, or instance). Pagination adds
complexity without current need.

**Door stays open**: new paginated methods are pure additions.
The trait surface is extensible.

### Streaming query results

**What it would be**: query methods returning streams.

**Why deferred**: result sets are small; streaming complicates
error handling and lifetime management.

**Door stays open**: new streaming methods are pure additions.

### Garbage collection

**What it would be**: periodic cleanup of orphaned substrate
data.

**Why deferred**: append-only design is structurally at odds
with deletion. Storage cost acceptable for current workloads.

**Door stays open**: GC is operational, not substrate-level.
Future tooling can run against the substrate; substrate would
need `delete_*` methods (breaking the append-only discipline)
if it participated directly.

## Connector and capabilities

### Tool calling in LLM connectors

**What it would be**: scripts declare tools; LLM requests tool
execution; connector service dispatches tools; results feed
back.

**Why deferred**: explicit architectural choice. Structured
output is the primary interface; agentic loops are composed in
JavaScript using structured output at each iteration. This
keeps control flow in the script (deterministic, auditable,
testable) rather than delegated to the LLM.

**NOT planned.**

### Streaming LLM responses

**What it would be**: LLM connector returns a stream instead of
a complete response.

**Why deferred**: streaming complicates the wire protocol (event
streams, partial parsing). Complete responses cover all
workflow-authoring patterns currently in view.

Not planned unless other (non-LLM) use cases for streaming
arise. LLMs alone don't justify it since they're deliberately
not a first-class citizen of the architecture.

### Upsert capabilities for stateful stores

**What it would be**: capabilities for populating vector stores
and modifying SQL schemas from within workflows. Lets consumers
ingest data into indexed stores via Philharmonic workflows
rather than out-of-band tooling.

**Why deferred**: the v1 position is that state population and
mutation for vector stores and SQL schemas happens out-of-band
using each store's native tooling. Query-shaped capabilities
are sufficient for the capability set shipping in v1.

**Door stays open**: upsert capabilities are pure additions to
the capability registry; they require no architectural changes.
Add them when a consumer has a concrete population-via-workflow
need worth the maintenance cost.

### Connector routing decisions in the API layer

**What it would be**: the API layer (or the workflow engine)
selects connector destinations based on runtime conditions.

**Why deferred**: with the capability-and-realm model, routing
decisions happen at lowering time (the lowerer picks the realm
based on capability definition and tenant configuration) and at
router level (the router load-balances within the realm). The
API layer doesn't need to care.

**Door stays open**: the `ConfigLowerer` trait is generic;
custom lowerers can implement arbitrary routing logic.

## Policy, tenancy, and authentication

### Per-tenant credentials

**Not deferred — ships in v1.** See `09-policy-and-tenancy.md`.

The earlier framing of "shared credentials at v1, per-tenant
later" was revised. Credential scoping is present from v1 in the
policy crate: retrofitting scoping onto an unscoped credential
store would force every consumer through a data migration, which
is worse than shipping the scoping up front. Single-scope
deployments use the scoping trivially (one tenant entity);
multi-scope deployments get isolation by construction.

### Hierarchical tenancy

**What it would be**: tenants containing sub-tenants, with
inherited or overridable credentials, capability authorizations,
and role definitions. Allows enterprise-style account
hierarchies where organization-level settings propagate to
departments or projects.

**Why deferred**: substantial design and evaluation-logic scope
with no current consumer requesting it. Customizable roles ship
in v1 and carry much of the flexibility that hierarchy would
otherwise provide for grouping permissions, which covers
anticipated near-term needs.

**Door stays open**: adding a parent entity slot to `Tenant` is
an additive change; evaluation logic walking up the tree is new
behavior layered on existing queries, not a restructuring.
Credential and capability inheritance become new resolution
rules in the lowerer.

### Session + access-token authentication pattern

**What it would be**: long-lived sessions referenced by opaque
tokens, exchanged for short-lived access tokens on demand.
Standard OAuth 2.0 refresh-token shape.

**Why deferred**: bearer ephemeral tokens with up-to-24h
lifetimes and epoch-based mass revocation are adequate for the
delivery patterns in scope for v1 (browser sessions, job-run
tokens, partner integrations, CLI/desktop tools). The session
model's per-session revocation granularity isn't needed yet;
the simpler bearer-token model is adequate.

**Door stays open**: adding session entities and an
access-token-exchange endpoint is a new feature, not a
rework. Existing ephemeral tokens continue to work as-is.

### Schema-constrained subject claims

**What it would be**: minting authorities declare a JSON Schema
describing the shape of injected subject claims; the API layer
validates mint requests against the schema; workflow scripts can
assume claim shapes.

**Why deferred**: v1 treats injected claims as free-form. The
minting authority and the workflow script are coordinated by
the tenant's own application; they can agree on a schema
informally without Philharmonic's mediation.

**Door stays open**: add an optional `claim_schema` content slot
on `MintingAuthority`; validate mint requests against it when
present. No existing behavior changes.

### Per-ephemeral-token revocation lists

**What it would be**: persistent revocation lists that let
individual ephemeral tokens be invalidated before natural
expiry.

**Why deferred**: token lifetimes are short enough that natural
expiry handles ordinary turnover, and the minting-authority
epoch mechanism covers compromise cases for all delivery
patterns anticipated so far. Per-token revocation is
additive-only infrastructure that shipping unused would cost
substrate growth without earning its complexity.

**Door stays open**: add a revocation list consulted at token
verification time. Storage-backed; new additive behavior. Scale
concern (list growth) would need consideration but is tractable.

### Per-tenant encryption keys

**What it would be**: each tenant's credentials encrypted with
a tenant-specific key rather than a deployment-shared key.
Improves isolation (substrate compromise exposes less if keys
are distinct) at operational cost.

**Why deferred**: deployment-level shared keys are adequate for
the v1 threat model. Per-tenant keys add key-management
complexity (one key per tenant to rotate; per-tenant HSM slots
if HSM-backed).

**Door stays open**: the `key_version` scalar on
`TenantCredential` already generalizes to "which key encrypted
this." Extending to "which tenant-specific key" is a per-tenant
key registry plus per-key rotation procedures.

### OAuth 2.0 / OIDC federation

**What it would be**: authenticate API callers via external
identity providers using OAuth 2.0 or OpenID Connect.

**Why deferred**: API tokens are adequate for v1. Federation
adds substantial scope (IdP registration, callback flows, token
exchange, claim mapping) with no consumer currently requesting
it; shipping it unrequested locks in choices that first adopters
should drive.

**Door stays open**: new authentication methods are additive.
The `AuthContext` enum in the API layer can grow a federated
variant.

### Session-based authentication for Web UI

**What it would be**: session cookies for browser-based Web UI
access, distinct from the API-token mechanism used
programmatically.

**Why deferred**: the Web UI uses the same API-token mechanism
(or ephemeral tokens for user-scoped operations) for v1. Adds
no new concepts; simpler to ship. Consumers preferring session
cookies can layer them in front of the API in their own
deployment.

**Door stays open**: adding session cookies and session
management is standard web engineering; no architectural
change.

## API layer

### Outbound webhooks

**What it would be**: API notifies consumer-registered endpoints
on workflow instance transitions (completion, failure, specific
state changes).

**Why deferred**: useful but not essential for v1. Consumers can
poll the API; webhooks are ergonomic sugar.

**Door stays open**: webhook configuration is a new entity kind
plus a dispatcher component. Pure addition.

### Distributed rate limiting

**What it would be**: rate limits enforced across the API fleet
(Redis-backed token buckets or similar), rather than per-node.

**Why deferred**: single-node token buckets are simple and work
for v1. Per-node buckets have the obvious scale limitation
(total throughput is `nodes × bucket_rate`) which is acceptable
at v1 scale.

**Door stays open**: rate limiting is an API-layer concern
swappable with a different implementation without changes to
the rest of the system.

### Streaming API responses

**What it would be**: API endpoints that stream responses (e.g.,
tailing a workflow instance's step records as they happen).

**Why deferred**: polling covers the interaction patterns
shipping in v1. Streaming is additive when a concrete use case
arises.

**Door stays open**: streaming endpoints are additive.

### GraphQL or gRPC transport

**What it would be**: API exposed via GraphQL or gRPC alongside
or instead of REST.

**Why deferred**: REST + JSON is standard, widely understood,
adequate.

**Door stays open**: additional transports are parallel to the
REST API, not replacements. Each would be a separate endpoint
surface.

### Atomic create-instance-and-mint-token endpoint

**What it would be**: single API call that creates a workflow
instance and mints an ephemeral token scoped to it, for
patterns where a caller wants to do both in sequence (e.g.,
creating a session and immediately handing its scoped token to
a downstream caller).

**Why deferred**: clients make two separate calls (create
instance, then mint token). Two calls is fine; an ergonomic
combined endpoint is sugar.

**Door stays open**: add the combined endpoint when usage
patterns demonstrate it's worth the API-surface increase.

### API layer split from workflow engine

**What it would be**: the API layer as a separate service from
the workflow engine, communicating over an internal protocol.

**Why deferred**: co-locating the workflow engine in the API
process adds no network hop and no complexity. Splitting adds
both without benefit.

**Door stays open**: the `WorkflowEngine` is a Rust struct with
trait-abstracted dependencies; wrapping it in an RPC service is
straightforward if deployment scale later demands separation.

## Cryptography and security

### Hybrid post-quantum signing

**What it would be**: COSE_Sign1 tokens signed with a hybrid
construction combining ML-DSA and Ed25519, rather than Ed25519
alone.

**Why deferred**: token lifetimes are short (minutes for
connector tokens, up to 24h for ephemeral API tokens), limiting
harvest-now-forge-later exposure on signed tokens (as opposed
to encrypted payloads, where harvest-now-decrypt-later is a
real threat). ML-DSA + Ed25519 hybrid is not yet widely
implemented in mature Rust libraries.

**Door stays open**: COSE is algorithm-agile. Adopting hybrid
signing is a matter of registering a new algorithm identifier,
generating new keypairs, updating signing and verification
code. No protocol rework.

### Per-token revocation windows for connector tokens

**What it would be**: connector services reject tokens whose
`iat` is older than a deployment-configured threshold, enabling
faster incident response than natural expiry.

**Why deferred**: short token lifetime is the baseline
revocation mechanism. Adding a revocation window is optional
defense-in-depth, not v1-essential.

**Door stays open**: the connector router or service can be
extended with a "minimum issuance timestamp" check. Additive.

### Hardware security modules

**What it would be**: signing keys and the substrate credential
key held in HSMs or cloud KMS services rather than software
storage.

**Why deferred**: software-stored keys are acceptable for v1
default deployments. HSM integration is a deployment-level
choice rather than an architectural constraint.

**Door stays open**: the lowerer and API layer can be refactored
to call a key-management interface rather than reading key
material directly. Deployment operators choose software, HSM,
or KMS without affecting the architecture.

### Automated key rotation

**What it would be**: scheduled rotation procedures for signing
keys, KEM keys, and the substrate credential key, with
automated overlap management.

**Why deferred**: manual rotation with documented procedures
works for v1. Automation is operational tooling, not
architectural.

**Door stays open**: rotation procedures are deployment-level.
Tools can be built that orchestrate the steps; the architecture
already supports overlap periods via COSE `kid` fields and
`key_version` scalars.

## Summary of deferral discipline

Features land when there's a concrete consumer needing them. The
architecture keeps options open by:

- **Trait-based interfaces at every layer** — swap
  implementations without rewriting consumers.
- **Append-only storage** — preserve history for later use
  (replay, audit).
- **Opaque pass-through at the workflow engine** — don't bake
  assumptions into the core (abstract config, concrete config,
  subject context).
- **Extension points in traits** — new methods are additive;
  old consumers keep working.
- **Stateless services** — future state-adding additions don't
  break existing deployments.
- **Algorithm-agile cryptography via COSE** — swap primitives
  when the ecosystem moves.
- **Per-kind deletion semantics** — entity kinds get the
  deletion model that fits them; no universal constraint.

Each entry above was considered and judged not yet worth
implementing, with a path for when implementation becomes
needed.
