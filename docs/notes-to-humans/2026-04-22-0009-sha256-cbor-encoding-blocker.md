# Blocker found while preparing Wave A Codex dispatch: `Sha256` CBOR encoding mismatch

**Date:** 2026-04-22
**Context:** Post-Gate-1 prep for Phase 5 Wave A (COSE_Sign1 tokens).

## What I found

While drafting the Python reference-vector generator for Wave A,
I went to encode the `payload_hash` field of
`ConnectorTokenClaims` and discovered the proposal's claim about
the wire encoding doesn't match the actual type.

- `docs/design/crypto-proposals/2026-04-22-phase-5-wave-a-cose-sign1-tokens.md`
  (r2/r3) asserts: "`Sha256` serializes as a 32-byte byte string
  (major type 2)."
- `philharmonic-types/src/sha256.rs:4-6` declares:
  ```rust
  #[derive(..., Serialize, Deserialize)]
  #[serde(transparent)]
  pub struct Sha256(#[serde(with = "hex_bytes")] [u8; 32]);
  ```
  and `hex_bytes::serialize` (`sha256.rs:57-62`) calls
  `s.serialize_str(&hex::encode(bytes))` — a hex-encoded tstr,
  unconditionally, regardless of the serializer's
  `is_human_readable` flag.
- `Uuid` is fine: `uuid 1.23` with the `serde` feature is
  human-readable-aware (JSON → string, CBOR → 16-byte bstr) out
  of the box.

So under the current type, `ConnectorTokenClaims` encoded via
`ciborium` would put the 32-byte hash as a 64-char hex **text
string** in CBOR. That's ~2× the wire size of a proper bstr and
unusual for a COSE-adjacent payload.

## Why this is a Gate-1 blocker

The proposal pinned the CBOR encoding as the Wave A wire form.
Dispatching Codex on the proposal as-written would produce tokens
where `payload_hash` is a hex tstr in the signed payload — then
we'd either have to live with that once tokens start flowing (1.0
breaking change later) or bump it before it ships (Wave B wire
break, plus recomputing every Wave A test vector).

I haven't dispatched Codex yet; the prompt is not archived. I
caught this while trying to produce the pycose reference bytes
and realized I needed to know exactly how `Sha256` encodes.

## Options

**(A) Fix `philharmonic-types 0.3.5` to mirror uuid's
human-readable-aware pattern.** `Sha256` emits:
- hex tstr when `serializer.is_human_readable()` (JSON — no
  behavior change for existing consumers), and
- 32-byte bstr when `!is_human_readable()` (CBOR — the COSE-right
  encoding).

Additive, non-breaking for JSON consumers, fixes CBOR. Needs a
philharmonic-types 0.3.5 release. `philharmonic-policy`,
`philharmonic-store`, `philharmonic-store-sqlx-mysql`, and
`philharmonic-connector-common` would need Cargo.toml bumps in
their `0.3.x` pins — which they already have from the 0.3.3 →
0.3.4 sweep.

Recommended.

**(B) Update the Wave A proposal to match the current type.**
`payload_hash` ships as a 64-char hex tstr in CBOR. Cheaper now,
more expensive forever (2× wire size, inconsistent with how
proper hashes encode in COSE).

Not recommended.

**(C) Define a separate `Sha256Bytes` newtype in
`philharmonic-connector-common` for CBOR-context use.** Keeps
`philharmonic-types` stable but introduces a parallel hash type.
Two types doing the same thing is a smell.

Not recommended.

## What I need from you

A pick between (A) and (B) — (C) is listed for completeness but I
don't think it earns its keep.

If (A): I'll
1. Patch `philharmonic-types/src/sha256.rs` to use a
   human-readable-aware serde, add tests covering both JSON and
   CBOR (ciborium) shapes, bump to 0.3.5, publish.
2. Bump the `philharmonic-types` pin in the three consumer crates
   that ship in v1 (policy, store, store-sqlx-mysql,
   connector-common) to `0.3.5`.
3. Revise the Wave A proposal r3 → r4 to reference the now-true
   encoding claim (no design change; just the text gets verified).
4. Resume the pycose reference-vector generation with the fixed
   encoding and dispatch Codex.

If (B): I revise the proposal r3 → r4 to match the hex-tstr
reality, regenerate the pycose reference using hex-tstr, and
dispatch.

Until picked, the Codex prompt stays un-archived and un-sent.

## Why I'm writing this as a note and not just a chat message

Per CLAUDE.md §notes-to-humans, substantive findings go in the
repo in addition to the chat. This one qualifies — wire-format
encoding for a token payload is exactly the kind of "quiet
invariant change that affects every downstream crate" I'd want a
record of if we look back at this in a month.
