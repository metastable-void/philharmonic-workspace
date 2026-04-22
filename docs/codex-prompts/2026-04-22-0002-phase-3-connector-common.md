# Phase 3 — `philharmonic-connector-common`

**Date:** 2026-04-22
**Slug:** `phase-3-connector-common`
**Round:** 01
**Subagent:** `codex:codex-rescue`

## Motivation

Kicks off Phase 3 of the v1 roadmap: implement the shared
vocabulary crate for the connector layer. This is a types-only
crate — structs, enums, thin type-safe wrappers over the `coset`
crate's COSE types, serde impls, and a small realm-registry
lookup helper. No crypto primitives are invoked here; the actual
COSE_Sign1 signing/verification and COSE_Encrypt0 hybrid KEM
encryption live in Phase 5 (`philharmonic-connector-client` and
`philharmonic-connector-service`).

Phase 2 (`philharmonic-policy`) has landed and is published as
`0.1.0` with the two-gate crypto review cycle completed. Phase 3
sits between policy and the crypto-bearing connector crates: it
defines the data shapes those later crates will sign/encrypt,
without touching the cryptographic construction itself.

## References

- `ROADMAP.md` §Phase 3 (lines 620–692) — scope, struct
  definitions, acceptance criteria. Source of truth.
- `docs/design/08-connector-architecture.md` — connector-layer
  architecture; `ConnectorCallContext` and `Implementation`
  trait definitions; realm-model narrative.
- `docs/design/11-security-and-cryptography.md` — COSE framing,
  claim-set definition for the connector authorization token,
  key identifiers. Authoritative where ROADMAP is ambiguous.
- `docs/design/13-conventions.md` — workspace conventions:
  MSRV 1.88, edition 2024, POSIX-sh scripts, cargo-wrapper
  discipline, no-panic rule, naming rules.
- `philharmonic-types` crate (v0.3.4) — cornerstone vocabulary
  providing `Uuid`, `UnixMillis`, `Sha256`, `ContentHash<T>`,
  `CanonicalJson`. Use these via the cornerstone re-export, not
  direct upstream crates.

## Scope

### In scope

Implement every item in ROADMAP §Phase 3 tasks 1–5:

1. **`ConnectorTokenClaims`** — COSE_Sign1 payload shape:

   ```rust
   pub struct ConnectorTokenClaims {
       pub iss: String,          // issuer, minting-authority identifier
       pub exp: UnixMillis,      // expiry
       pub kid: String,          // realm key ID
       pub realm: String,        // target realm identifier
       pub tenant: Uuid,         // tenant UUID (internal or public — match design doc 11)
       pub inst: Uuid,           // workflow instance UUID
       pub step: u64,            // step sequence number
       pub config_uuid: Uuid,    // tenant endpoint config UUID
       pub payload_hash: Sha256, // SHA-256 of the encrypted payload
   }
   ```

   Notes:
   - ROADMAP shows `exp: u64` shorthand; use `UnixMillis` from
     `philharmonic-types` — the cornerstone's canonical millis
     type. `issued_at` / `expires_at` in design doc 08 are
     `UnixMillis`. Consistency matters.
   - ROADMAP shows `payload_hash: [u8; 32]`; use the cornerstone
     `Sha256` newtype (which IS a 32-byte array under the hood
     with the right serde wire format). The newtype prevents
     "any 32 bytes" conflation with other hashes.
   - Use plain `Uuid` (not `EntityId<T>` or `InternalId<T>` /
     `PublicId<T>`), per design doc 08 line ~481: keeps this
     crate free of `philharmonic-policy` / `philharmonic-workflow`
     dependencies.
   - If design doc 11 pins tenant as "public UUID" vs "internal
     UUID" for wire use, match that explicitly and call it out
     in a code comment. If the design doc is ambiguous, flag in
     the final summary and default to whatever `ConnectorCallContext`
     in design doc 08 uses.

2. **`ConnectorCallContext`** — verified-claim bundle the
   framework hands to `Implementation::execute`:

   ```rust
   pub struct ConnectorCallContext {
       pub tenant_id: Uuid,
       pub instance_id: Uuid,
       pub step_seq: u64,
       pub config_uuid: Uuid,
       pub issued_at: UnixMillis,
       pub expires_at: UnixMillis,
   }
   ```

   Plain `Uuid` for the same reason as above.

3. **Realm model:**

   ```rust
   pub struct RealmId(String);          // newtype over String
   pub struct RealmPublicKey { ... }    // ML-KEM-768 + X25519 hybrid public key
   pub struct RealmRegistry {           // kid → RealmPublicKey map
       by_kid: HashMap<String, RealmPublicKey>,
   }
   ```

   `RealmPublicKey` is a data carrier; no key-generation, no
   encryption. Layout suggestion (match design doc 11 if it
   pins field names; otherwise this is the canonical shape):

   ```rust
   pub struct RealmPublicKey {
       pub kid: String,
       pub realm: RealmId,
       pub mlkem_public: Vec<u8>,   // ML-KEM-768 public key bytes (1184 bytes fixed)
       pub x25519_public: [u8; 32], // X25519 public key
       pub not_before: UnixMillis,
       pub not_after:  UnixMillis,
   }
   ```

   If ML-KEM-768 public key size is known (1184 bytes), prefer
   `[u8; 1184]` over `Vec<u8>` for type discipline — but only if
   `coset` / `serde` serialize it cleanly. If not, fall back to
   `Vec<u8>` with a validated-length invariant enforced at
   construction. Either is defensible; flag the choice.

   `RealmRegistry` needs at minimum `.lookup(kid: &str) ->
   Option<&RealmPublicKey>`; `.insert(key: RealmPublicKey)` with
   kid-uniqueness enforcement is a reasonable constructor
   addition.

4. **COSE_Sign1 and COSE_Encrypt0 wrapper types.** Use the
   `coset` crate from crates.io. Thin type-safe newtypes;
   NO signing, NO verification, NO encryption, NO decryption in
   this crate. Proposed shape:

   ```rust
   pub struct ConnectorSignedToken(pub coset::CoseSign1);
   pub struct ConnectorEncryptedPayload(pub coset::CoseEncrypt0);
   ```

   Or: thin `Deref`/`AsRef` wrappers. Goal is (a) compile-time
   distinction from raw `CoseSign1` / `CoseEncrypt0` elsewhere,
   (b) forcing callers through typed constructors that apply
   `coset` validation. If the minimal win doesn't justify a
   wrapper, just re-export `coset::CoseSign1` /
   `CoseEncrypt0` under crate-local type aliases with a
   doc-comment pinning semantic meaning — flag that choice.

5. **`ImplementationError` enum** with variants:

   ```rust
   pub enum ImplementationError {
       InvalidConfig { detail: String },          // couldn't deserialize impl config
       UpstreamError { status: u16, body: String }, // 4xx/5xx from external
       UpstreamUnreachable { detail: String },    // network failure
       UpstreamTimeout,
       SchemaValidationFailed { detail: String }, // LLM structured-output mismatch
       ResponseTooLarge { limit: usize, actual: usize },
       InvalidRequest { detail: String },         // script sent malformed request
       Internal { detail: String },               // catch-all
   }
   ```

   Use `thiserror` for `#[derive(Error)]` + `#[from]` wiring.
   Field names are a suggestion; match ROADMAP + design doc 08
   §"Mapped to `ImplementationError` variants" if they pin
   specific names.

### Out of scope (explicitly)

- **COSE signing / verification** — Phase 5
  (`philharmonic-connector-client`, `philharmonic-connector-service`).
- **COSE encryption / decryption** — Phase 5 (same crates).
- **ML-KEM-768 / X25519 / HKDF / AES-256-GCM primitive calls**
  — Phase 5.
- **Payload-hash computation** (the actual hashing of the
  encrypted payload for token binding) — Phase 5. Phase 3 only
  declares the claim shape that will carry the hash.
- **Realm key rotation** — Phase 5 or later.
- **`philharmonic-policy` integration** — Phase 5 / 8.
- **The `Implementation` trait itself** — lives in
  `philharmonic-connector-service` per design doc 08. Phase 3
  defines the types `Implementation::execute` takes and returns;
  the trait definition comes later.

## Dependencies to add

Add to `philharmonic-connector-common/Cargo.toml`:

- `philharmonic-types = "0.3"` — cornerstone (`Uuid`,
  `UnixMillis`, `Sha256`, `CanonicalJson`). Pin to `"0.3"` (minor)
  to pick up patch-level additions automatically. Workspace
  `[patch.crates-io]` redirects to the local submodule.
- `mechanics-config = "0.1"` — referenced by design doc 08 as a
  common dep; include if `ImplementationError` / other types
  need it. Skip if nothing in Phase 3 scope uses it; flag the
  choice.
- `coset = "0.3"` — COSE structures. Look up the current version
  via `./scripts/xtask.sh crates-io-versions -- coset` and pin
  to the latest-minor.
- `serde = { version = "1", features = ["derive"] }` — serde
  derives for the claim types.
- `serde_json = "1"` — if tests need JSON round-trips; optional
  otherwise.
- `thiserror = "2"` — error derives. Look up latest via
  `./scripts/xtask.sh crates-io-versions -- thiserror`.

**Crate version lookup rule:** never recall a crate's published
version from memory. Always probe via
`./scripts/xtask.sh crates-io-versions -- <crate>`. Model-
training data is months stale; prior-session memory is frozen
in time. This applies even if you "remember" what the right
version is.

Do NOT add these yet:
- `aead`, `aes-gcm`, `hkdf`, `x25519-dalek`, `ml-kem` — Phase 5.
- `zeroize` / `secrecy` — Phase 3 doesn't hold secret key
  material. Phase 5 introduces these.

## Tests required

Unit tests colocated (`#[cfg(test)] mod tests`) covering:

1. **`ConnectorTokenClaims` serde round-trip** — serialize to
   JSON (or CBOR if that's what the wire format will use; check
   design doc 11), deserialize, assert equality. Include at
   least one claim set with realistic values plus edge cases
   (empty strings where allowed, `0`-valued integers where
   allowed, wrapper types correctly round-tripping).

2. **`ConnectorCallContext` serde round-trip** — same pattern.

3. **`RealmRegistry` lookup tests:**
   - `lookup(kid)` returns `Some(&realm_public_key)` for an
     inserted key.
   - `lookup(kid)` returns `None` for a missing key.
   - Duplicate-kid insertion — either error or replace; pin the
     behavior with a test either way, and flag in the final
     summary so Claude can confirm the semantics.

4. **`RealmPublicKey` length validation** — if you chose
   `Vec<u8>` for `mlkem_public`, construction must reject
   wrong-length inputs. Test both the happy path and a
   wrong-length rejection.

5. **`ImplementationError` serde round-trip** — ensure the enum
   serializes cleanly (useful for including errors in API
   responses later).

6. **`coset` wrapper types** — minimal smoke test that a
   wrapped `CoseSign1` / `CoseEncrypt0` can be constructed from
   a sample `coset` value. No crypto.

No integration tests for Phase 3 — there are no external
substrates to integrate with yet. No `#[ignore]`-gated tests.

## Crypto-sensitivity

Phase 3 is **not a Gate-1 crypto-review trigger** — there is no
cryptographic construction. It *is* the moment where the wire-
format claim schema lands as code, and if a field is named
wrong or typed wrong here, it cascades into Phase 5. So:

- Match ROADMAP and design doc 11 exactly for claim-set field
  names and types. If either spec is ambiguous, **stop and flag
  in the final summary** rather than inventing a shape.
- If implementation nudges into primitive calls, key material
  handling, or payload-hash *computation* (not just declaration
  of the hash field), **stop immediately** and flag — that's
  Phase 5 territory requiring Yuka's Gate-1 sign-off.

Gate-2 (pre-publish code review) **does** apply before the
`philharmonic-connector-common 0.1.0` release, even though
there's no crypto construction here — the claim schema is
crypto-adjacent infrastructure and Yuka reviews it.

## Workspace conventions (authoritative: `docs/design/13-conventions.md`)

- **Edition 2024, MSRV 1.88.** Match in `Cargo.toml`.
- **License `Apache-2.0 OR MPL-2.0`.** No per-file copyright
  headers; `LICENSE-APACHE` and `LICENSE-MPL` already exist at
  the crate root.
- **Errors via `thiserror`** — no `anyhow` in library crates.
  Partition error variants by what a caller does with them.
- **No panics in library code.** This is systems-programming
  infrastructure; panics are user-visible failure.
  - No `.unwrap()` / `.expect()` on `Result`/`Option`. Use `?`
    with a typed error variant, or `.ok_or_else(...)` /
    `.map_err(...)`.
  - No `panic!` / `unreachable!` / `todo!` / `unimplemented!` on
    reachable paths — model unreachability at the type level.
  - No unbounded indexing (`slice[i]`, `map[&k]`) — use
    `.get(...)` → `Option` and propagate.
  - No unchecked integer arithmetic. For Phase 3 most fields
    are `u64` / `usize` and simple comparisons; if arithmetic
    surfaces, use `checked_*` / `saturating_*` / `wrapping_*`
    to declare intent.
  - No lossy `as` casts on untrusted widths — use `TryFrom`.
  - Narrow exceptions require inline justification.
  Tests / `#[cfg(test)]` blocks can `.unwrap()` freely.
- **Re-export discipline.** Re-export types from direct
  dependencies that appear in this crate's public API
  (e.g. if `Sha256` or `UnixMillis` appear publicly, re-export
  them from the crate root). Don't re-export transitive deps.
- **Rustdoc** — aim for high coverage. Every public type gets a
  doc comment explaining what it is and when a caller would
  touch it. The cornerstone (`philharmonic-types`) is at 99%+;
  match the discipline.
- **POSIX-ish host.** This workspace assumes GNU/Linux (incl.
  WSL2), macOS, BSDs, or musl distros. Your runtime is already
  there; just noting the rule.
- **No raw `cargo` — use `./scripts/*.sh` wrappers.** Before
  concluding, run:

  ```sh
  ./scripts/pre-landing.sh philharmonic-connector-common
  ```

  Pre-landing runs `fmt --check` + `check` + `clippy -D warnings`
  + `test`. If fmt-check fails, run `cargo fmt -p
  philharmonic-connector-common` and re-run pre-landing.

## Git

You do NOT commit, push, or branch. Leave the working tree
dirty — Claude runs `./scripts/commit-all.sh` and
`./scripts/push-all.sh` after review. Same rule as always
(AGENTS.md §Git).

## Deliverables

1. `philharmonic-connector-common/src/lib.rs` (or split into
   `claims.rs`, `context.rs`, `realm.rs`, `cose.rs`,
   `error.rs` modules with a thin `lib.rs`) containing the
   types above.
2. `philharmonic-connector-common/Cargo.toml` with the
   dependencies enumerated above and the `version` left at
   `"0.0.0"` — Claude handles the bump to `0.1.0` at publish
   time.
3. `philharmonic-connector-common/README.md` — crate README
   covering purpose, quick example of constructing a claim set
   and a realm-registry lookup, and a "what's in / what's out"
   summary. Replace the existing stub.
4. `philharmonic-connector-common/CHANGELOG.md` — new
   `[Unreleased]` section listing the initial type inventory.
5. Passing `./scripts/pre-landing.sh philharmonic-connector-common`.

## Acceptance criteria

Match ROADMAP §Phase 3 acceptance:

- `philharmonic-connector-common` compiles with the minimal
  deps listed in §Dependencies above.
- Unit tests for token-claim serde round-trip and realm-
  registry lookup by kid are present and pass.
- `cargo clippy --all-targets -- -D warnings` is clean.
- Rustdoc coverage on public items.
- Publishing as `0.1.0` is deferred to Claude — you leave the
  version at `0.0.0`.

## When in doubt

- If ROADMAP and a design doc conflict on a specific field name
  or type, **design doc 11 wins for crypto-format fields**;
  **design doc 08 wins for API-shape fields**; **ROADMAP wins
  as a tie-breaker if both design docs are silent**. If all
  three disagree or are all silent, stop and flag in the final
  summary.
- If a test reveals the spec is under-constrained (e.g.
  duplicate-kid insertion semantics), pin your choice with a
  test and flag the decision in the final summary so Claude
  can adjudicate in review.
- If Phase 3 scope would require touching Phase 5 territory
  (primitive calls, key generation, payload-hash *computation*),
  STOP and flag. Don't improvise crypto.

Report in your final summary:
- Files changed (paths).
- Any ambiguities you resolved and the choice you made.
- Any anomalies where the submodule's own `[profile.release]`
  differs from the workspace `Cargo.toml`'s profile (Phase 3 is
  not the time to fix this, but flag it if noticed).
- `pre-landing.sh` result.
- Anything you would have written to
  `docs/codex-reports/YYYY-MM-DD-NNNN-<slug>.md` per
  `AGENTS.md §Reports` — write the report if the bar is met,
  skip otherwise.
