# Design Principles

These are cross-cutting commitments that shape decisions across the
whole system. They're settled; they act as constraints on all design
work.

## Append-only storage

Storage operations add data; they never modify or delete. No
`update`, no `delete`, no `upsert`. Soft-delete is expressed as a new
revision with a deletion scalar, not as removal.

**Why.** Collapses concurrency (no two writers conflict on existing
rows — only on PK collisions, which become semantic errors). Makes
every entity auditable by default. Simplifies replication. Enables
replay in principle.

**Cost.** Storage grows monotonically. Garbage collection of
genuinely orphaned data is out-of-band.

## Content-addressed

Anything storable as bytes and identifiable by content is keyed by
SHA-256 hash. JSON content is canonicalized (RFC 8785, JCS) before
hashing. Same bytes → same hash → deduped automatically.

Workflow scripts, configs, contexts, inputs, outputs are all
content-addressed. References to content travel as hashes.

## Backend-agnostic interfaces

Storage traits in `philharmonic-store` have no SQL. SQL
implementation is a separate crate. The workflow engine doesn't know
which backend. Same principle for the executor (trait, not concrete
HTTP client) and the connector lowerer (trait, not concrete policy
implementation).

## Vocabulary collapses misuse paths

Types in the cornerstone are narrow. `ScalarType` has only `Bool`
and `I64` — no `Str`. Strings belong in content blobs, i64 enum
encodings, or entity references, never as ad-hoc scalar columns.

The substrate refuses to bless patterns that lead to bad designs,
even when the broader type would be technically useful.

## LCD MySQL

SQL uses only features common to MySQL 8, MariaDB 10.5+, Aurora
MySQL, and TiDB. No JSON columns, no vendor-specific operators, no
declared foreign keys. `BIGINT` for timestamps, `BINARY(16)` for
UUIDs, `BINARY(32)` for hashes, `MEDIUMBLOB` for content.

## Statelessness in execution

JavaScript workers maintain no state across jobs. Each job runs in a
fresh Boa realm. `globalThis` mutations don't persist. Workers are
fungible; load balancers use simple algorithms.

Same principle for connector services: no cross-request state beyond
operational state (connection pools, realm private keys, static
implementation registry built at startup). Credentials arrive
per-request via the encrypted payload; the service does not cache
credentials.

## Cornerstone as workspace anchor

`philharmonic-types` is the single source of truth for shared
vocabulary types (`Uuid`, `JsonValue`, `Sha256`, `EntityId<T>`, etc.).
Downstream crates re-export from the cornerstone to prevent version
skew.

## Errors carry meaning, not stack traces

Error types partition the failure space by what the caller should do,
not by what went wrong technically. `StoreError::RevisionConflict` is
a retryable-concurrency-outcome; `StoreError::Decode` is a
probably-a-bug semantic violation. Callers pattern-match to decide
behavior.

## Ergonomic typed surfaces, object-safe base traits

Substrate traits are object-safe (take UUIDs and bytes). Extension
traits provide typed methods via blanket impls. Implementors work
with the small base; consumers work with the typed extensions.

## Layered ignorance

Each layer doesn't know about layers above. Substrate doesn't know
about workflows. Executor doesn't know about persistence or
orchestration. Connector services don't know about workflow state.

This is enforced by making the information structurally unavailable:
the substrate has no vocabulary for "workflow instance"; the
executor receives jobs with no instance ID in the protocol.

## Implementation uniformity

All connector implementations are the same kind of thing: Rust code
satisfying the `Implementation` trait, speaking one category of
external service. They differ only in the external protocol they
speak and the domain knowledge they encode. No category is
privileged at the framework, capability, or token level.

**LLM connectors are not first-class citizens.** `llm_openai_compat`
is structurally identical to `http_forward`; both are HTTP clients
with provider-specific domain knowledge. The framework has no
LLM-specific code paths, vocabulary, or entity kinds. If a feature
seems to require LLM awareness at the framework or token level, the
design has probably gone wrong — either the feature belongs inside
an implementation's domain knowledge, or the capability's wire
protocol should be extended in a way that applies to any
implementation.

This keeps the system shaped toward general-purpose orchestration
with LLM use as one application among many, rather than an
LLM-centric platform that sometimes orchestrates other things.

## Defer until concrete

Features land when there's a real consumer needing them. Designing
speculatively produces APIs shaped by imagined needs. The system is
shaped to *not foreclose* deferred features (append-only enables
future replay; stateless workers enable future determinism; generic
lowerer interface enables future alternate implementations) without
*implementing* them.

See `12-deferred-decisions.md` for specifics.

## Capability via signed tokens and encrypted payloads

Script-to-connector authorization is per-step signed tokens,
short-lived, binding one `TenantEndpointConfig` to one step via its
UUID and a payload hash. External-service credentials and
capability-bearing URLs live inside per-tenant encrypted config
blobs, re-encrypted at step time to per-realm KEM public keys and
decryptable only by the connector services of the destination
realm. The executor is an untrusted intermediary; a Boa exploit
yields ciphertext, not credentials.

This is a security principle as well as a design principle: the
threat model assumes the executor may be compromised and designs the
cryptographic boundaries accordingly.
