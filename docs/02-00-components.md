# Philharmonic — Components

This document enumerates the system's concrete components: what each
one does, what crate (or crates) implement it, and how they depend on
each other. The conceptual framing lives in `00-overview.md`; this
document is the canonical reference for "what crates exist and how do
they fit together."

The deep design rationale for each component lives in its own
document, listed at the end of each component's section.

## The map

The system has five components in active design or use, plus several
future layers reserved as crate names. Each component sits at a
definite layer of the stack:

```
                    Workflow (philharmonic-workflow)
                              |
            +-----------------+-----------------+
            |                 |                 |
        Storage          Execution          Connectors
   (philharmonic-     (mechanics-core,    (philharmonic-
    store +            mechanics)          connector)
    backends)
            |                 |                 |
            +-----------------+-----------------+
                              |
                  Cornerstone (philharmonic-types)
```

Cornerstone vocabulary at the bottom. Three independent middle layers:
storage (persistence), execution (JS computation), and connectors
(capability-mediated I/O). Workflow at the top, integrating all three.
The arrows are dependency directions: workflow depends on the three
middle layers; each middle layer depends on the cornerstone; nothing
in the middle layer depends on its peers at the crate level.

The connector layer's relationship to execution deserves a note. The
connector services receive HTTP calls *from* the executor's runtime
when scripts invoke configured capabilities, but the connector layer's
crate doesn't depend on `mechanics-core`. The runtime-to-service
relationship is wire-protocol, not crate-dependency.

## Cornerstone

**`philharmonic-types`** — the workspace's shared vocabulary crate.
SHA-256 hashes, phantom-typed UUID identities, content-addressed JSON,
millisecond timestamps, the `Entity` trait and slot declarations. No
runtime, no I/O, no async. Acts as the version anchor: types like
`Uuid` and `JsonValue` are re-exported from here so that downstream
crates share one canonical definition.

Status: published, in active use.

Deep dive: `02-01-cornerstone.md`.

## Storage substrate

Two crates in active use, with more backends possible.

**`philharmonic-store`** — substrate trait definitions: `ContentStore`,
`IdentityStore`, `EntityStore`, plus typed extension traits and an
umbrella convenience trait. No SQL, no async runtime, no database
driver dependencies. Crates that want to be backend-agnostic depend
only on this.

Status: published, in active use.

**`philharmonic-store-sqlx-mysql`** — the canonical SQL implementation,
backing the substrate onto MySQL-family databases (MySQL 8, MariaDB,
Aurora MySQL, TiDB) via `sqlx`. Uses LCD-compatible SQL only; no
vendor-specific features.

Status: published, in active use.

Future backend implementations (in-memory for testing, alternative SQL
flavors) would be sibling crates implementing the same trait surface.
The trait crate's API is stable enough that adding a backend doesn't
require trait changes.

Deep dive: `02-02-storage.md`.

## Execution substrate

Two crates, mirroring the storage split.

**`mechanics-core`** — the JavaScript execution library. Wraps Boa
runtimes in a worker pool, accepts jobs as `(module_source, arg,
config)`, returns JSON results or stringified errors. Stateless per
job; no cross-job mutable state. Defines the `MechanicsConfig` schema
that captures the runtime capabilities a script is permitted.

Status: published, in active use.

**`mechanics`** — the HTTP service exposing `mechanics-core` over the
network. Worker nodes run this binary.

Status: name claimed; implementation in progress.

The orchestrator never depends on `mechanics-core` directly. It talks
to `mechanics` instances over HTTP, with the `MechanicsConfig` shape
as the wire-level data contract.

Deep dive: `02-03-execution.md`.

## Connector layer

The connector layer is in design; crates exist as defensive name
claims. The component is described here for completeness; specifics
are subject to refinement as implementation progresses.

**`philharmonic-connector`** — defines the abstract capability schema
that template configs use, the `ConfigLowerer` trait that produces
runtime configs from abstract ones, and (likely) reference
implementations of common lowerer patterns. Depends on the concrete
config types from the execution substrate (either via `mechanics-core`
directly, or via an extracted schema crate; decision pending).

Status: name reserved; implementation not started.

**`philharmonic-realm`** — defines realm vocabulary: networking
scope, isolation domain, deployment-target labels for connector
services. Whether this becomes a separate crate or folds into
`philharmonic-connector` is undecided.

Status: name reserved; implementation not started.

Connector service binaries are deployment artifacts, not philharmonic
crates per se. The reference deployment uses a single static binary
per realm, containing all connector kinds the realm supports.
Connector router behavior — signature verification, dispatch to
services within a realm — is part of the same binary or a sibling
service.

Deep dive: `02-04-connectors.md`.

## Workflow orchestration

**`philharmonic-workflow`** — the orchestration layer. Defines
workflow templates, instances, and step records as entity kinds.
Implements the lifecycle state machine. Bridges storage (via
`philharmonic-store` traits, generic over backend), execution (via a
`StepExecutor` trait, generic over transport), and connectors (via a
`ConfigLowerer` trait, generic over policy implementation). Holds none
of these concrete types directly; everything is plugged in at
construction.

Status: in design; implementation not started.

Deep dive: `02-05-workflow.md`.

## Future layers

Crates that exist as defensive name claims with no current
implementation, planned for when use cases materialize.

**`philharmonic-policy`** — tenants, principals, permissions,
authorization decisions. Defines policy entity kinds and evaluation
logic. Likely consumed by both the API layer (for request
authorization) and the connector lowerer (for capability
authorization).

**`philharmonic-api`** — public HTTP API for external consumers.
Translates between API requests and workflow operations. Mints
capability tokens for the lowerer's use; consults the policy layer for
authorization decisions.

These will be designed and built when their use cases become concrete.
They do not exist yet beyond name reservation.

## Meta-crate

**`philharmonic`** — name placeholder on crates.io, currently
published as 0.0.0 with empty contents. May eventually become a
convenience re-export crate; the decision is deferred until there's a
clear convenience worth providing.

## Crate ownership status

Currently real (published with substantive content):

- `philharmonic-types`
- `philharmonic-store`
- `philharmonic-store-sqlx-mysql`
- `mechanics-core`

Currently claimed (name owned, content TBD):

- `philharmonic` (0.0.0 placeholder)
- `mechanics` (in development)

Reserved but not yet claimed:

- `philharmonic-workflow`
- `philharmonic-connector`
- `philharmonic-realm`
- `philharmonic-policy`
- `philharmonic-api`

The defensive-claim policy and the rationale for early name
reservation are documented in `05-conventions.md`.

## Dependency graph in detail

The dependencies between crates, omitting standard upstream
dependencies (serde, tokio, etc.):

```
philharmonic-types               (no philharmonic deps)
philharmonic-store               → philharmonic-types
philharmonic-store-sqlx-mysql    → philharmonic-store, philharmonic-types
mechanics-core                   → (no philharmonic deps)
mechanics                        → mechanics-core
philharmonic-connector           → philharmonic-types,
                                   mechanics-core (or extracted schema)
philharmonic-workflow            → philharmonic-types,
                                   philharmonic-store,
                                   philharmonic-connector
```

`philharmonic-workflow` does not depend on `mechanics-core` or
`mechanics`. It depends on `philharmonic-connector`'s `ConfigLowerer`
trait and on its own `StepExecutor` trait; implementations of those
traits depend on the executor's wire format, not the workflow crate
itself.

## Layer compositions

The components compose into running services in different ways
depending on deployment topology. A few common patterns:

**Single-binary deployment.** All philharmonic and mechanics components
in one process. The binary embeds an HTTP listener for the API,
in-process workflow engine, in-process storage backend (or HTTP client
to a separate database), in-process executor (or HTTP client to a
separate worker fleet). Suitable for development, testing, and small
deployments.

**Separated services.** API server, workflow engine, executor fleet,
and connector services as separate binaries, possibly on separate
hosts. The API server holds the workflow engine; the workflow engine
talks to MySQL for storage and to mechanics over HTTP for execution;
the executor's runtime talks to connector services over HTTP.
Suitable for production deployments wanting independent scaling of
each tier.

**Multi-region.** Multiple API + workflow tiers per region; shared
MySQL with appropriate replication; per-region executor and connector
fleets. Realm assignment becomes geographic. The substrate's LCD
MySQL discipline supports this naturally; the connector layer's
realm-as-deployment-target model fits.

The crate boundaries don't dictate deployment topology. A single
binary can hold every crate; separated services pick subsets per host.
The trait-based interfaces (storage, execution, lowerer) make the
seams swappable without code changes.
