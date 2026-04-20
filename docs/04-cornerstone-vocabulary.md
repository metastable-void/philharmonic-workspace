# Cornerstone Vocabulary (`philharmonic-types`)

The workspace's shared vocabulary crate. Stable, narrow, depended on
by everything else. Currently at v0.3.3.

## What's in it

### Hash and content addressing

- **`Sha256`** — 32-byte digest. Constructed via `Sha256::of(bytes)`
  or `Sha256::from_bytes_unchecked(arr)` for trusted sources.
- **`HashFunction`** trait — abstracts over hash algorithms.
  `Sha256` is the only shipping implementation.
- **`ContentHash<T, F = Sha256>`** — typed hash, parameterized by the
  content type `T`. Compile-time distinction between
  `ContentHash<Script>` and `ContentHash<Context>`.
- **`ContentValue`** — untyped bytes-plus-hash with the invariant
  `hash == Sha256::of(bytes)` enforced at construction.
- **`Content`** trait — declares a type as content-addressable.
  Requires deterministic encode/decode.
- **`CanonicalJson`** — JSON canonicalized per RFC 8785 (JCS).
  Semantically-equal JSON produces byte-equal `CanonicalJson`.

### Identity

- **`Identity`** — untyped pair of `(internal: Uuid, public: Uuid)`.
  Internal is UUIDv7 (time-ordered), public is UUIDv4 (opaque).
- **`Id<T, KIND: u8>`** — typed wrapper with const generic for
  distinguishing internal from public at the type level.
- **`InternalId<T>`** and **`PublicId<T>`** — type aliases.
- **`EntityId<T>`** — typed identity pair.
- **`Identity::typed::<T>()`** — validates UUID versions and
  produces `EntityId<T>`.

The phantom-type pattern (`PhantomData<fn() -> T>`) distinguishes
entity types at compile time with zero runtime cost.

### Entity declarations

- **`Entity`** trait — implemented by marker types to declare
  themselves as entity kinds. Associated constants: `KIND: Uuid`,
  `NAME: &'static str`, `CONTENT_SLOTS`, `ENTITY_SLOTS`,
  `SCALAR_SLOTS`.
- **`ContentSlot::new(name)`** — declares a content-hash slot.
- **`EntitySlot::of::<T>(name, pinning)`** — declares a reference to
  another entity. `T: Entity` bound at the declaration.
- **`ScalarSlot::new(name, ty, indexed)`** — declares a small typed
  scalar.
- **`SlotPinning::Pinned`** vs. **`SlotPinning::Latest`** —
  references can pin to a specific revision or track latest.
- **`ScalarType`** — enum with variants `Bool` and `I64` only. **No
  `Str` variant** (deliberate constraint; see principles).
- **`ScalarValue`** — runtime form: `Bool(bool)` or `I64(i64)`.

### Time and JSON

- **`UnixMillis`** — wraps `i64` milliseconds since Unix epoch.
  Constructor `UnixMillis::now()`; bare field constructor for
  existing values.
- **`JsonValue`**, **`JsonMap`** — re-exports of
  `serde_json::Value` and `serde_json::Map` for cross-crate
  consistency.

### Errors

- **`ContentDecodeError`** — decoding bytes into a content type
  failed.
- **`IdKindError`** — UUID version didn't match expectation.
- **`IdentityKindError`** — compound error for `Identity::typed`.
- **`CanonError`** — malformed JSON input to `CanonicalJson`.

These flow into `philharmonic-store`'s `StoreError` via `#[from]`.

## What's deliberately not in it

- Specific entity kinds (live in their domain crates).
- Storage interfaces (live in `philharmonic-store`).
- Async runtime or I/O.
- Serde-arbitrary trait bounds on types that don't need them.
- Business-logic types (tenants, principals, etc.).
- General-purpose utilities.

Inclusion rule: a type goes in the cornerstone if it appears in
multiple crates' public APIs. Otherwise it doesn't.

## Re-export discipline

Downstream crates importing upstream types (`Uuid` from `uuid`,
`JsonValue` from `serde_json`) should use the cornerstone's
re-exports:

```rust
use philharmonic_types::{Uuid, JsonValue};  // ✓
use uuid::Uuid;                              // ✗
```

This pins one version of each upstream type across the workspace.

## Versioning discipline

Cornerstone is on the strict end. Patch releases for additions;
minor releases (breaking) are bundled and announced; major releases
are ecosystem events.

Downstream crates pin to minor version (`philharmonic-types =
"0.3"`) to pick up patch-level additions automatically while
protecting against minor-version breakage.

## Status

Published at crates.io and docs.rs. 99%+ documented. Stable enough
that patch-level additions are the current mode of evolution.

A 1.0 release waits until the workflow layer and at least one upper
layer (policy or API) have validated the vocabulary's fitness.
