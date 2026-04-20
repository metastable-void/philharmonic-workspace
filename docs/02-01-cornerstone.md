# Philharmonic — Vocabulary

This document covers `philharmonic-types`, the cornerstone crate that
defines the shared vocabulary used across the workspace. The crate
exists for one purpose: to be the single source of truth for types
that appear in multiple crates' APIs.

## What the cornerstone is for

A workspace of cooperating crates needs shared types. Workflow code
constructs `EntityId<WorkflowInstance>` values and passes them to
storage code, which compares them, hashes them, and writes them to
the database. For that to work, "the workspace's `EntityId`" has to
mean exactly one type, regardless of which crate the value came from
or which crate consumes it.

Two ways to arrange this. The crates could each depend on `uuid`
directly and use `uuid::Uuid` everywhere — but then a version skew
between crates produces silent type mismatches. Or one crate could own
the canonical definitions and the others re-export from it — which
gives the workspace one version of every shared type, set in one
place, propagating through the dependency graph.

The cornerstone is the second arrangement. `philharmonic-types` owns
the workspace's vocabulary. Other crates depend on it and re-export
from it where appropriate. Upstream version pins live in the
cornerstone's `Cargo.toml`; downstream crates inherit them transitively.

## What's in the cornerstone

The contents fall into a few categories.

### Hash and content addressing

`Sha256` wraps a 32-byte digest. Construction goes through
`Sha256::of(bytes)` (computes the hash) or
`Sha256::from_bytes_unchecked(arr)` (asserts the bytes are a real
digest from a trusted source — used by storage backends decoding hash
columns, never by ad-hoc callers).

The `Content` trait declares what it means for a type to be
content-addressable: a method to encode the value to canonical bytes,
and a method to decode bytes back to the value. The encoding must be
deterministic — the same value must always produce the same bytes —
because content addresses depend on it.

The `HashFunction` trait abstracts over hash algorithms.
`HashFunction::digest(bytes) -> Output` is the function shape.
`Sha256` is the only implementation that ships with the cornerstone;
the trait exists so that `ContentHash<T, F>` can be parameterized over
hash functions if a future need arises (a different algorithm for a
specific use case, perhaps), but the default is `Sha256`.

`ContentHash<T, F = Sha256>` is a typed hash: a digest tagged with
what content type it's a hash *of*. A
`ContentHash<CanonicalJson>` cannot be passed where a
`ContentHash<JsScript>` is expected, even though both wrap the same
32-byte digest. This catches at compile time the class of bugs where
the wrong content type is fetched for a hash.

`ContentValue` is the untyped form: bytes plus their hash, together,
with the invariant `hash == Sha256::of(bytes)` enforced at
construction. Storage backends that shuttle bytes around without
caring about the type use this; consumers that have a typed `T:
Content` use the typed APIs in `philharmonic-store`.

`CanonicalJson` is JSON canonicalized per RFC 8785 (JCS). The bytes
inside are guaranteed canonical: keys sorted at every level, numbers
formatted per ECMA-262, strings escaped per RFC 8259, no insignificant
whitespace. This means semantically-equal JSON values produce
byte-equal `CanonicalJson` values, which produce identical content
hashes. The deserialization impl runs JCS canonicalization on input,
so a `CanonicalJson` received over the wire is always canonical
regardless of how the sender serialized it. Implements `Content`,
making it the natural choice for any JSON-shaped content the system
stores.

### Identity

`Identity` is the untyped pair: an `internal: Uuid` (UUIDv7,
time-ordered, used for internal storage and indexing) and a `public:
Uuid` (UUIDv4, opaque, used for external references). This is the
storage-boundary form — what the substrate reads and writes.

`Id<T, KIND: u8>` is the typed wrapper. The `KIND` const generic
distinguishes internal IDs (`KIND_INTERNAL`) from public IDs
(`KIND_PUBLIC`). Type aliases `InternalId<T>` and `PublicId<T>` are
what consumers actually use; the bare `Id<T, KIND>` is an
implementation detail that lets both share construction and
formatting code.

`InternalId<T>::from_uuid` and `PublicId<T>::from_uuid` validate that
the UUID has the expected version (7 or 4 respectively) and return
`IdKindError` if not. This enforces the invariant at the boundary
where untrusted UUIDs become typed IDs. Constructors that don't
validate (`from_uuid_unchecked`) exist for the trusted-source case
and are named accordingly.

`EntityId<T>` is the typed identity pair: an `InternalId<T>` and a
`PublicId<T>` together, parameterized by the entity kind. This is
what application code holds when it knows what kind of entity it's
working with. Consumers go from `Identity` to `EntityId<T>` via
`Identity::typed::<T>()`, which validates UUID versions and returns
`IdentityKindError` on failure. Going the other way is `EntityId<T>::untyped()`,
which is infallible.

The phantom-type pattern (`PhantomData<fn() -> T>`) provides type
distinction without runtime cost. An `EntityId<WorkflowInstance>`
and an `EntityId<WorkflowTemplate>` are distinct types at compile
time but identical bytes at runtime.

### Entity declarations

The `Entity` trait is what types implement to declare themselves as
entity kinds. The trait has no methods, only associated constants:

- `KIND: Uuid` — the globally-unique identifier for this entity kind,
  generated once at type-authoring time as a UUIDv4 and never changed.
- `NAME: &'static str` — a human-readable name for debug output and
  tooling. Not used for identity.
- `CONTENT_SLOTS`, `ENTITY_SLOTS`, `SCALAR_SLOTS` — static slices
  declaring the slots the entity kind uses.

Implementors are typically zero-sized marker structs:

```
struct WorkflowTemplate;
impl Entity for WorkflowTemplate {
    const KIND: Uuid = /* ... */;
    const NAME: &'static str = "workflow_template";
    /* slot declarations */
}
```

The marker struct exists only to give the type system something to
parameterize over. It's never instantiated.

`ContentSlot::new(name)` declares a content-hash slot. The slot has
a name (scoped to the entity kind) and nothing else; the content
type isn't recorded because content is type-erased at the storage
layer. Consumers know what type to decode each slot as because they
authored the kind.

`EntitySlot::of::<T>(name, pinning)` declares a reference to another
entity. The `T: Entity` bound at the declaration site catches typos
and renames (the slot won't compile if `T` doesn't exist or doesn't
implement `Entity`). The recorded `target_kind` is `T::KIND`. The
`pinning` field is `SlotPinning::Pinned` (reference includes a
specific revision sequence) or `SlotPinning::Latest` (reference
tracks whatever revision is current).

`ScalarSlot::new(name, ty, indexed)` declares a small typed scalar.
The `ty` is `ScalarType::Bool` or `ScalarType::I64` (no `Str`; see
the principles document). The `indexed` flag is a hint to the
storage backend that queries on this attribute should be efficient.

`ScalarValue` is the runtime form: `Bool(bool)` or `I64(i64)`. This
is what gets stored and retrieved. Mismatched types between
`ScalarSlot::ty` and `ScalarValue::ty()` are caught at the storage
layer.

### Timestamps and JSON

`UnixMillis` wraps `i64` milliseconds since the Unix epoch. The
substrate's `created_at` and similar fields use this. Constructors
are `UnixMillis::now()` and the public field constructor; the type
deliberately doesn't try to be a full date-time abstraction. For
formatting, conversion to higher-level date types, or arithmetic,
consumers convert at the boundary using whatever date library they
prefer (`chrono`, `time`, or just doing the math).

`JsonValue` and `JsonMap` are re-exports of `serde_json::Value` and
`serde_json::Map`. They appear here so that downstream crates can
depend on the cornerstone for JSON types rather than depending on
`serde_json` directly. This keeps the workspace's `Value` type pinned
to one version.

### Errors

`ContentDecodeError` is what `Content::from_content_bytes` returns
when bytes don't decode as the expected type. Variants cover the
common cases (invalid UTF-8, invalid JSON) plus a `Custom(String)`
escape hatch for type-specific decode failures.

`IdKindError` is what `from_uuid` constructors return when a UUID's
version doesn't match expectations. Carries the expected and actual
version numbers.

`IdentityKindError` is the compound version of `IdKindError` for
`Identity::typed::<T>()`, distinguishing whether the internal or the
public UUID was wrong.

`CanonError` is what `CanonicalJson` constructors return when input
JSON is malformed.

These errors flow into the storage substrate's `StoreError` via
`#[from]` impls, so consumer code can use `?` cleanly when these
errors arise from cornerstone operations.

## What's not in the cornerstone

The exclusion list is deliberate and worth being explicit about.

**No specific entity kinds.** `WorkflowTemplate`, `WorkflowInstance`,
`StepRecord`, and any future entity kinds belong in the crates that
own the corresponding domain (`philharmonic-workflow`, etc.). The
cornerstone provides the `Entity` trait; specific kinds implement it
elsewhere.

**No storage interfaces.** `ContentStore`, `IdentityStore`,
`EntityStore`, and `StoreError` live in `philharmonic-store`. The
cornerstone has no concept of "storing" anything — it defines what
the things being stored *are*, not how they're stored.

**No async runtime, no I/O.** Every cornerstone type is pure data
plus pure functions. Consumers can use the cornerstone in any async
runtime, in synchronous code, in const contexts where applicable,
in WebAssembly. The lack of async is what makes the cornerstone
universally depend-on-able.

**No serde-arbitrary trait bounds.** Cornerstone types implement
`Serialize` and `Deserialize` where needed, but the cornerstone
doesn't define traits that *require* serde or take serde traits as
generic parameters. This keeps the cornerstone's surface clean for
consumers that don't use serde.

**No business-logic types.** `Tenant`, `Principal`, `Permission`, and
similar concepts that might appear in a future policy layer are not
cornerstone vocabulary. They would be entity kinds in
`philharmonic-policy` if and when that crate exists.

**No general-purpose utilities.** String helpers, collection
extensions, "missing" standard library functionality — none of
these. The cornerstone is shared vocabulary, not a toolkit.

The discipline: a type belongs in the cornerstone if it appears in
multiple crates' public APIs. Otherwise it doesn't.

## The re-export discipline

The cornerstone's role as workspace anchor depends on downstream
crates following a convention: when an upstream type appears in a
substrate-or-orchestration-layer API, import it from
`philharmonic-types`, not from the upstream crate.

Currently the re-exported upstream types are `Uuid` (from `uuid`)
and `JsonValue` / `JsonMap` (from `serde_json`). The convention is:

```
// In philharmonic-store, philharmonic-workflow, etc.:
use philharmonic_types::{Uuid, JsonValue};
// Not:
use uuid::Uuid;
use serde_json::Value as JsonValue;
```

The benefit is one canonical version per upstream type across the
workspace. If `philharmonic-types` updates to `uuid 2.x`, the whole
workspace updates together; if downstream crates depended on `uuid`
directly, they could lag or skew.

The convention applies whenever the upstream type is part of an API
that crosses crate boundaries. Implementation details that don't
cross boundaries (sqlx types inside the SQL backend, reqwest types
inside an HTTP executor) don't need to flow through the cornerstone.

When a new upstream type starts appearing in cross-crate APIs, the
right move is to add it to the cornerstone (a patch-bump addition,
non-breaking) and update consumers to use the cornerstone's version.
Doing this proactively at the moment of introduction is much cheaper
than retroactively when version skew has already produced bugs.

## Versioning and stability

`philharmonic-types` is on the strict end of the workspace's
versioning discipline. Because so many other crates depend on it,
breaking changes have outsized consequences — every dependent crate
needs to update.

Patch releases (0.x.y → 0.x.(y+1)) are for additions and
documentation: new types, new methods on existing types, new
re-exports, derive additions, doc improvements. These don't break
existing consumers.

Minor releases (0.x.y → 0.(x+1).0) are for changes to existing
types: signature changes, removed methods, semantic changes. These
break consumers and require coordinated updates across the workspace.
Avoided where possible; bundled when necessary.

The major version (currently 0.x) signals pre-1.0 status: API
changes are possible at minor-version increments. The promise is
that 0.x → 0.(x+1) changes will be documented, and that downstream
crates won't be left behind without paths to update.

A 1.0 release will happen when the cornerstone's API is stable
enough that breaking changes are genuinely rare, which probably
requires the workflow and policy layers to exist and validate the
vocabulary's fitness for those use cases.

## What this means for downstream crates

A crate depending on the cornerstone should:

- Pin to a specific minor version (`philharmonic-types = "0.3"`) to
  pick up patch-level additions automatically while protecting
  against minor-version breakage.
- Use cornerstone re-exports for upstream types that appear in its
  own API surface, rather than depending on those upstream crates
  directly.
- Treat the cornerstone's vocabulary as the workspace's lingua
  franca — when in doubt about how to model a cross-crate
  data shape, look for a cornerstone type that fits before inventing
  one locally.
- Push genuinely-shared types up to the cornerstone via a patch
  release rather than redefining them locally. The barrier to adding
  a re-export or a small type to the cornerstone should be low.

The cornerstone earns its place by being small, stable, and cheap to
depend on. The discipline that maintains those properties is
consistent application of the inclusion rule: "this type appears in
multiple crates' APIs, therefore it belongs here." When that's true,
addition is the right move. When it's not, local definition is fine.
