# Phase 5 Wave B hybrid KEM + COSE_Encrypt0 design security review

**Date:** 2026-04-22
**Prompt:** docs/codex-prompts/2026-04-22-0005-phase-5-wave-b-hybrid-kem-cose-encrypt0.md *(not present in workspace; this review was requested directly in-session)*

## Scope

Reviewed:

- `docs/design/crypto-proposals/2026-04-22-phase-5-wave-b-hybrid-kem-cose-encrypt0.md`

Focus: design-level security properties for the Wave B encryption/decryption construction and its composition with Wave A token verification.

## Findings

### 1) High: COSE `Enc_structure` is not specified, so AEAD auth does not cover protected headers at the content layer

The proposal defines COSE_Encrypt0 structure and protected headers (`...hybrid-kem-cose-encrypt0.md`:315-333), but decryption is specified as raw AES-GCM over `aad_bytes` derived from claim fields (`...md`:286-305).

In RFC 9052 COSE_Encrypt0, AEAD AAD is the serialized `Enc_structure = ["Encrypt0", protected, external_aad]`, not just an application-defined `external_aad` digest. As written, the content-layer AEAD check does not authenticate protected headers unless Wave A payload-hash verification ran first and correctly.

Impact:

- Security layering is fragile under refactors that reorder or split verification/decryption paths.
- Interop with standards-compliant COSE implementations is likely to fail or diverge.

Recommended design fix:

- Keep your current claim-binding value as `external_aad`, but feed AEAD with canonical CBOR of full COSE `Enc_structure`.
- State this explicitly in both encrypt and decrypt algorithms.

### 2) High: key-registry oracle leakage via distinct Step-13 failures

Step 13 returns separate errors for unknown key IDs and out-of-window keys (`...md`:388), and negative vectors assert those distinct outcomes (`...md`:529-534).

Impact:

- An attacker can probe `kid` namespace validity and key activation windows if these variants are reflected at API boundaries (status code/body/timing).
- This materially helps targeted cryptographic abuse and operational reconnaissance.

Recommended design fix:

- Keep detailed internal error variants for logs/tests, but require a single indistinguishable external failure surface for all crypto/key-selection failures (Step 13 + Step 14).
- Explicitly define response uniformity requirements (status/body/latency budget).

### 3) High: insufficient binding between claims and the decryption key actually used

Design uses protected-header `kid` for private-key lookup (`...md`:388), while AAD includes `claims.kid` (`...md`:286-294). No explicit equality check (`protected_kid == claims.kid`) is specified, and registry shape is keyed only by `kid` (`...md`:429-437). Step 15 checks plaintext realm vs claim realm, but not key realm vs claim realm (`...md`:390).

Impact:

- Acceptable messages can exist where claims and actual decryption key selection diverge (especially under implementation bugs or mixed-tenant key-distribution mistakes).
- Weakens auditability and policy enforcement around realm/key ownership.

Recommended design fix:

- Make decryption lookup keyed by `(realm, kid)` instead of `kid` alone.
- Add mandatory checks: `protected_kid == claims.kid` and `registry_entry.realm == claims.realm` before decapsulation.

### 4) Medium: replay remains accepted and now amplifies CPU-exhaustion risk

Replay is explicitly unchanged from Wave A (`...md`:117-122). In Wave B, each accepted replay drives ML-KEM decapsulation + X25519 + HKDF + AEAD decrypt (`...md`:389).

Impact:

- A single captured valid `(token, ciphertext)` inside `exp` can be replayed to force expensive cryptographic work repeatedly.
- This shifts replay from integrity risk only to an availability pressure point.

Recommended design fix:

- Define anti-replay semantics for Wave B dispatch path (for example, bounded replay cache keyed by signed token identifier/payload hash and `exp` TTL).
- At minimum, specify per-tenant/per-instance rate-limits for decrypt attempts.

### 5) Medium: payload-size and structure bounds are underspecified

The design parses attacker-controlled COSE payload bytes at Step 12 (`...md`:387) and carries arbitrary ciphertext bstr (`...md`:321) but does not set explicit hard limits for total payload size, header-map size, or parse allocations.

Impact:

- Memory and CPU pressure from oversized payloads/headers is not normatively constrained.
- Security behavior can drift across crates if each layer invents its own limits.

Recommended design fix:

- Specify hard numeric limits in the design: max total encrypted payload size, max ciphertext size, max protected-header byte length/map entries.
- Require exact fixed lengths for `kem_ct` (1088), `ecdh_eph_pk` (32), and nonce (12) before expensive operations.

### 6) Medium: mandatory algorithm/parameter validation is not explicit in the decrypt sequence

The protected header table defines required parameters (`alg`, `kid`, `IV`, `kem_ct`, `ecdh_eph_pk`) (`...md`:327-333), but the service algorithm only states generic parse then decrypt (`...md`:387-390).

Impact:

- Parser edge cases (missing headers, duplicate keys, unexpected extra headers, wrong lengths/types) may be handled inconsistently and create downgrade/confusion risks.

Recommended design fix:

- Add an explicit validation step requiring:
  - `alg == 3` exactly.
  - `unprotected` map empty.
  - required headers present exactly once with exact type/length.
  - duplicate-label rejection.

## Positive notes

- The token-to-ciphertext commitment (`payload_hash`) plus AAD context binding is a strong compositional direction (`...md`:26-29, 126-134, 280-305).
- Secret-material lifecycle is documented unusually well for a proposal draft (`...md`:447-466).
- Negative-path vector planning is thorough and should catch many regressions (`...md`:523-555).

## Gate recommendation

Do not approve Gate-1 unchanged. Items 1-3 are security-critical and should be made normative in the design before implementation. Items 4-6 should be resolved as explicit requirements so crate implementations cannot diverge on security behavior.
