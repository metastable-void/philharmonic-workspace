# Phase 5 Wave A COSE_Sign1 token design security review

**Date:** 2026-04-22
**Prompt:** docs/codex-prompts/2026-04-22-0004-phase-5-wave-a-cose-sign1-tokens.md *(not present in workspace; this review was requested directly in-session)*

## Scope

Reviewed:

- `docs/design/crypto-proposals/2026-04-22-phase-5-wave-a-cose-sign1-tokens.md`

Focus: design-level security properties for Wave A signing and verification.

## Findings

### 1) High: replay resistance is not specified

The claim set has no nonce / token ID (`jti`) and verification is purely stateless (`signature`, `exp`, `payload_hash`, optional `realm`) (`...tokens.md`:64-74, 145-157).

If a valid `(token, payload)` pair is observed, an attacker can replay it until `exp` without any server-side replay detector. This is especially risky if connector actions are non-idempotent.

**Recommended design fix:** add explicit anti-replay semantics now (for example, `jti` + bounded replay cache keyed by issuer/kid, or strict one-time monotonic `step` validation tied to persisted workflow state).

### 2) High: key validity windows are defined but not in the verification algorithm

Key registry entries include `not_before` and `not_after` windows (`...tokens.md`:229-236), but the seven-step verification order does not include a window check (`...tokens.md`:137-159).

That mismatch creates a failure mode where operationally retired keys can still verify tokens unless manually removed, and future-dated keys may be accepted early.

**Recommended design fix:** make key-window enforcement a mandatory verification step immediately after `kid` lookup and before signature verification.

### 3) High: no audience binding, and weak issuer-key binding

Claims include `iss` but no `aud`/service identity (`...tokens.md`:64-74). Verification only optionally checks `realm` (`...tokens.md`:156-157). Registry lookup is by `kid` only (`...tokens.md`:217-224), and `kid` format is advisory/free-form (`...tokens.md`:240-244).

This permits token confusion across services/environments that share keys and claim schema, and lets any key holder mint for arbitrary `iss` unless additional constraints are enforced elsewhere.

**Recommended design fix:** add `aud` claim and require exact audience match; bind registry entries to expected issuer and enforce `claims.iss == registry_entry.issuer`.

### 4) Medium: algorithm pinning is not explicit on verify

The design says signer sets `alg = -8` (EdDSA) (`...tokens.md`:98), but verification steps do not explicitly reject non-EdDSA protected headers (`...tokens.md`:137-159).

Relying on library behavior is weaker than a hard policy check and can enable algorithm-confusion regressions if dependencies or call paths change.

**Recommended design fix:** explicitly require protected `alg == EdDSA` before signature verification.

### 5) Medium: signing-key file handling omits local hardening requirements

Minting keys are loaded from a configured file path, with format still undecided (`...tokens.md`:173-176). The design does not require file-permission/ownership validation.

On multi-user hosts or misconfigured deployments, world/group-readable key files become a straightforward secret-compromise path.

**Recommended design fix:** require strict permission checks (`0600`-class, expected owner) before loading secret key material; fail closed on mismatch.

### 6) Medium: unbounded payload hashing enables resource-exhaustion pressure

Verification hashes caller-supplied payload bytes (`...tokens.md`:149-150) but the design does not set any payload size ceilings or streaming/backpressure rules.

An attacker can force large hash workloads and memory pressure by repeatedly sending oversized payloads.

**Recommended design fix:** define hard payload size limits (or streaming hash with bounded body size at ingress) and rejection behavior.

### 7) Low: duplicated `kid` locations can drift without an explicit equality check

`kid` is placed in both protected header and payload (`...tokens.md`:99-101, 240-243), but the verify sequence does not specify a `claims.kid == protected_kid` check (`...tokens.md`:137-159).

This is primarily a consistency/auditability risk rather than cryptographic breakage, but it can produce confusing forensic records.

**Recommended design fix:** either remove `kid` from claims or enforce strict equality and fail on mismatch.

## Positive notes

- Signature-first verification before claim trust is correctly prioritized (`...tokens.md`:140-144, 359-361).
- Use of COSE protected headers for `alg` and `kid` is directionally strong (`...tokens.md`:96-104).
- Explicit negative vector planning for tamper paths is good and should catch many regressions (`...tokens.md`:315-333).

## Gate recommendation

Do not approve Gate-1 unchanged. The replay model, key-window enforcement, and audience/issuer binding should be made explicit and mandatory before implementation to avoid security model drift across crates.
