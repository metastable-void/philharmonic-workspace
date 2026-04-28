# Sub-phase B0 — Claude assessment of Codex code-audit findings

**Author:** Claude Code · **Audience:** Yuka (supplements
the Claude code-review note `2026-04-28-0004`) ·
**Date:** 2026-04-28 (Tue) JST afternoon

Codex ran an independent code audit of the B0 implementation
against the Gate-1 r2 proposal. Report at
[`docs/codex-reports/2026-04-28-0002-phase-8-b0-api-token-code-audit.md`](../codex-reports/2026-04-28-0002-phase-8-b0-api-token-code-audit.md).
Two findings. Here's my read on each and what I'm doing about
them.

## Finding 1: Claim-payload decoding accepts trailing bytes

**Codex severity:** low/medium.
**My assessment:** agree, low/medium. **Fix before Gate-2.**

The verify path uses `ciborium::de::from_reader(&mut
claims_reader)` which decodes one CBOR value and stops.
If the signed payload is `<valid claims CBOR> || <extra
bytes>`, the extra bytes are inside the COSE_Sign1
signature envelope (so they're authenticated) but silently
dropped by the verifier. The returned
`EphemeralApiTokenClaims` doesn't represent the full signed
payload.

**Why this matters (even though it's not an attack vector):**

- Audit tooling that hashes or records raw token payload
  bytes would see data the verifier ignores. A compliance
  auditor comparing "what was signed" vs "what was accepted"
  would see a discrepancy.
- If a future minting bug appends garbage, the verifier
  wouldn't catch it — the token would be accepted with
  stale claims.
- The proposal r2's intent is that the signed payload IS the
  canonical claims CBOR, byte-for-byte. Trailing bytes
  violate that invariant even if they're signature-covered.

**Why it's NOT an attack vector:** An unsigned attacker
can't exploit this. Changing any trailing byte invalidates
the Ed25519 signature. Only a holder of the signing key can
produce a token with trailing bytes, and they could equally
produce a token with any claims they want. The risk is
operational drift / audit confusion, not compromise.

**Fix:** After each `ciborium::de::from_reader` call, check
that the reader slice is exhausted. If not, reject as
`Malformed`. Two sites: the main claims decode (verify step
9) and the `raw_injected_claims_text` re-parse. One-line
check each.

I'm implementing this fix now, in the same commit as this
note.

## Finding 2: Unknown signed claim fields accepted and stripped

**Codex severity:** low.
**My assessment:** agree, low. **Fix before Gate-2.**

The inner `ClaimsHelper` struct used for deserialization
doesn't carry `#[serde(deny_unknown_fields)]`. A signed
CBOR map with extra fields beyond the 11 we expect would
deserialize successfully — the extra fields would be
silently dropped from the returned `EphemeralApiTokenClaims`.

**Why this matters:**

- Profile stability. The claim struct is a fixed-shape
  protocol. If we accept tokens with extra fields today,
  upgrading the struct later (to consume a new field) changes
  the semantics of tokens that were previously accepted under
  the old struct. That's a forward-compatibility trap.
- Audit stability. The signed payload contains data the
  verifier ignores. Same audit-discrepancy concern as
  Finding 1.

**Why it's also not an attack vector:** Same reasoning as
Finding 1 — only a holder of the signing key can produce
tokens with extra fields. The verifier returning the correct
subset of fields doesn't create a privilege escalation or
bypass.

**Fix:** Add `#[serde(deny_unknown_fields)]` to the inner
`ClaimsHelper` struct. One attribute.

I'm implementing this fix now, in the same commit.

## Combined assessment

Both findings are about CBOR profile strictness, not about
signature or authentication bypass. They're the kind of
issue where a security-conscious system should be strict
because the cost of strictness is near-zero (one line per
fix) and the cost of leniency is audit confusion plus a
forward-compatibility trap. The Codex auditor was right to
flag them, and I missed both in my initial review — my
trace through the 14 verify steps confirmed the step
ordering but didn't catch the "what happens after the
step?" leniency in ciborium's decoder behavior.

After these two fixes land, the verify path enforces:
- No trailing bytes after the claims CBOR (Finding 1).
- No unknown fields in the claims CBOR map (Finding 2).
- Re-canonicalization check on the `claims` text field
  (already in place from step 10).

Together these mean the signed payload bytes ARE the
canonical claims CBOR, byte-for-byte, field-for-field.
