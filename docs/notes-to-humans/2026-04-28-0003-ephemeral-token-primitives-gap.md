# Sub-phase B blocker — ephemeral API token primitives don't exist yet

**Author:** Claude Code · **Audience:** Yuka · **Date:** 2026-04-28 (Tue) JST late morning · **Severity:** Decision needed before sub-phase B can dispatch

## TL;DR

The 2026-04-28 morning approach-gate approval for Phase 8
(recorded in
[`2026-04-28-0001-phase-8-decisions-confirmed.md`](2026-04-28-0001-phase-8-decisions-confirmed.md))
states:

> Phase 8 introduces no new cryptography at the framework
> level. It consumes primitives that have already passed the
> workspace's two-gate crypto-review protocol during their
> original development:
> - **COSE_Sign1 mint + verify** for ephemeral tokens — wave A
>   primitives in `philharmonic-connector-common` and
>   `philharmonic-policy`. The API crate is a caller, not an
>   implementer.

I started drafting sub-phase B's prompt and verified this
assumption against the actual workspace code. **The
ephemeral-API-token primitives don't exist anywhere in the
workspace.** What exists is connector-token mint + verify
(different claim shape, payload-hash binding, realm binding) —
not reusable as-is for ephemeral API tokens, which need a
distinct claim shape, no payload-hash, an authority_epoch
binding, and a separate signing-key rotation story.

So sub-phase B isn't "just a caller". It needs primitives
introduced first — and **introducing crypto primitives is what
the crypto-review-protocol skill's Gate 1 was designed for**.
Your morning approval covered the consumer pattern, not the
primitives.

This blocks dispatch of sub-phase B until we settle a few
questions.

## What actually exists vs what doc 11 §"Token types" assumes

### Long-lived API tokens (`pht_`) — exist ✅

In [`philharmonic-policy/src/token.rs`](../../philharmonic-policy/src/token.rs):
- `parse_api_token(s) -> Result<TokenHash, PolicyError>` —
  validates `pht_` format, returns SHA-256 hash for substrate
  lookup.
- `generate_api_token() -> (Zeroizing<String>, TokenHash)` —
  mints fresh tokens.
- `TokenHash`, `TOKEN_PREFIX = "pht_"`, length constants.

Sub-phase B can use these directly. ✅

### Connector authorization tokens (lowerer→service) — exist ✅

In `philharmonic-connector-client/src/signing.rs` and
`philharmonic-connector-service/src/verify.rs`:
- `LowererSigningKey::mint_token(&ConnectorTokenClaims) ->
  ConnectorSignedToken` — Ed25519 + COSE_Sign1, hard-bound to
  `ConnectorTokenClaims` (which carries `realm`, `tenant`,
  `inst`, `step`, `config_uuid`, `payload_hash`, `kid`, `iss`,
  `exp`, `iat`).
- `verify_token(...) -> Result<ConnectorCallContext,
  TokenVerifyError>` — hard-bound to the same claim shape,
  enforces `payload_hash` constant-time match against a
  separately-supplied payload, enforces `realm` against the
  service's own realm.

These are **not generic**: the claim shape is fixed, the realm
binding is intrinsic to the verify path, and the payload-hash
check is mandatory in the verify code path. Reusing them for
ephemeral API tokens would mean either (a) generalizing them in
place — itself a wave-A re-design — or (b) duplicating the
COSE_Sign1 + Ed25519 + kid-lookup machinery with the right claim
shape.

### Ephemeral API tokens — do NOT exist ❌

Per
[`docs/design/11-security-and-cryptography.md` §"Ephemeral API tokens"](../design/11-security-and-cryptography.md#ephemeral-api-tokens)
and §"Ephemeral API token signing key rotation":

> **Distinct from the lowerer's signing key; same rotation
> pattern**

So even the design doc expects this to be its own implementation,
just sharing the rotation pattern. The claim shape per
[`docs/design/09-policy-and-tenancy.md`'s ephemeral-token spec](../design/09-policy-and-tenancy.md):

```
{ iss, exp, sub, tenant, authority, authority_epoch,
  optional instance, permissions, injected_claims, kid }
```

Greppable evidence:
- Zero occurrences of `EphemeralApiTokenClaims` (or any
  similar struct name) workspace-wide.
- Zero occurrences of `authority_epoch` in `.rs` files
  workspace-wide.
- One occurrence of `injected_claims` — in
  `philharmonic-api/src/auth.rs` as a runtime-context field,
  not a token-claim struct.

Sub-phase A landed `AuthContext::Ephemeral { ... }` as a
runtime-context enum, but the *signed-and-verified bytes that
populate it* have no implementation.

## Why this matters for the gate

The crypto-review-protocol skill is unambiguous:

> **Gate 1: pre-approval of approach, before coding.** Before
> Claude writes any of the primitives above — and before any
> Codex prompt that implements them — produce a short written
> proposal and get Yuka's sign-off.

Adding ephemeral-API-token mint+verify is *exactly* the kind of
primitive that needs a Gate 1 proposal under
`docs/design/crypto-proposals/<date>-<topic>.md`. The
specific things Gate 1 wants pinned:

- Exact primitives and library versions (Ed25519 via
  `ed25519-dalek`, COSE_Sign1 via `coset`, claim serialization
  via `ciborium` — all already in the workspace, but the
  versions need to be pinned in the proposal).
- Claim struct field-by-field (lifted from doc 09).
- Algorithm pinning at the COSE protected-header level
  (EdDSA-only, reject others — same pattern as connector verify).
- Kid binding rules (protected-header kid must match
  payload-claim kid, same pattern).
- Zeroization story (the signing seed, any plaintext the
  process holds).
- Test-vector plan (canonical claim → canonical signed bytes,
  validates against known-good).
- Rotation story: where the API signing key lives, how
  deployments supply it, how `kid` lookup happens at verify
  time.

Your morning approval explicitly says "Phase 8 introduces no
new cryptography at the framework level". That framing was
given on the assumption that the primitives already existed.
Since they don't, the framing doesn't apply — adding the
primitives **is** new cryptography at the framework level (or at
the policy-crate level, depending on placement).

## Decisions you need to make

These can't be deferred into the Codex prompt — they shape
where the code lives, what crate version bumps are involved, and
whether sub-phase B is one round or two.

### D1. Where do the ephemeral-token primitives live?

Three options:

**(a) `philharmonic-policy`**
- Pros: Sits next to `Sck`, `sck_encrypt`, `sck_decrypt`,
  `parse_api_token`, `generate_api_token` — already the
  "auth/identity primitives" crate. One crate, one cohesive
  surface.
- Cons: `philharmonic-policy` 0.1.0 already published; this is
  a meaningful additive surface, so a 0.2.0 bump (or 0.1.1 if
  we treat it as additive) is needed. The crate gains a
  `coset` dep it doesn't currently have.

**(b) New crate `philharmonic-api-token`** (or similar name)
- Pros: Clean separation; the API token system is conceptually
  its own protocol. Gate 1 proposal is scoped tightly.
- Cons: One more crate to publish + maintain.
  `philharmonic-policy` already carries connection-adjacent
  primitives so the boundary feels artificial.

**(c) Inside `philharmonic-api` itself**
- Pros: No version bumps to other crates; everything stays
  in-scope for sub-phase B.
- Cons: Anyone else who needs to verify ephemeral tokens (e.g.,
  workflow engine if it ever needs to inspect the original
  token) would have to depend on `philharmonic-api`, which
  inverts the dep direction. **Not recommended.**

My instinct: **(a) `philharmonic-policy`**, mirroring how
`Sck` sits there — but it's your call.

### D2. Sub-phase B scope split

Two shapes:

**(a) Sub-phase B as one round.** B introduces the primitives
*and* the consumer middleware in one Codex round. Crypto
review fires on the whole round.

**(b) Sub-phase B0 (primitives) + B1 (consumer middleware) as
two rounds.** B0 is the primitives only — Gate 1 proposal,
crypto review, publish a 0.x version of the host crate. B1
consumes them. Each round is reviewed under its own gate.

My instinct: **(b) split**, because B0's deliverable is small
and crypto-precise (claim struct + sign + verify + kid
registry trait + tests), while B1 is mostly plumbing
(middleware that calls B0). Splitting lets the crypto review
happen on a tight, reviewable surface without code-review
fatigue on adjacent middleware plumbing.

### D3. Is this the same approach gate, or a new one?

The 2026-04-28 morning approval was based on a wrong
assumption. Two options:

**(a) Re-open and supersede.** Treat the morning approval as
covering only the consumer-side framing. Write a fresh Gate 1
proposal under `docs/design/crypto-proposals/` for the new
primitives, get sign-off, then proceed. Cleaner audit trail.

**(b) Amend the existing note.** Add a section to
`2026-04-28-0001-phase-8-decisions-confirmed.md` clarifying
that the primitives need to be built and that's still
considered "approved at the approach level". Less paperwork
but blurs the gate-record.

My instinct: **(a) fresh Gate 1 proposal**, because the
discipline of producing the proposal is what catches misuse —
and the existing note's "no new crypto" sentence is what the
proposal will actually disagree with, so superseding is the
honest move.

### D4. Impact on the 5/2 target

Adding B0 as its own round adds (on the order of) one Codex
round + your code-level review. Realistic numbers:

- Gate 1 proposal: I write a draft today (~couple hours of my
  time, no Codex), you review.
- B0 dispatch: Codex implements primitives + tests + crate
  version bump (~30-60 min round). Code-level crypto review
  fires.
- B1 dispatch: Codex implements the auth middleware on top
  (~30 min round).

Net delta vs. the original "B is one round" plan: roughly +1
round + the proposal-writing time + Yuka's proposal-review
time. Plausible to absorb into 4/30 if proposal sign-off lands
today and B0 dispatches today/tomorrow. If proposal sign-off
slips past today, the 5/2 target gets harder; the cut-list
(see
[`2026-04-28-0002-pre-gw-target-may-2-end-to-end.md`](2026-04-28-0002-pre-gw-target-may-2-end-to-end.md))
moves up.

## What I'm asking you to do

In rough order:

1. **Tell me your calls on D1, D2, D3.**
2. If D3 = (a), I'll draft the Gate 1 crypto proposal under
   `docs/design/crypto-proposals/2026-04-28-ephemeral-api-token-primitives.md`,
   commit, and surface it for your sign-off. The proposal will
   reference the design docs (10 + 11 + 09) for the
   specification, pin specific dep versions and construction
   choices, and propose test vectors.
3. Once D1/D2/D3 are settled and (if applicable) the Gate 1
   proposal is approved, I draft the sub-phase B (or B0)
   Codex prompt and dispatch.

Sub-phase B does **not** dispatch in the meantime. The
auth_placeholder middleware in
[`philharmonic-api/src/middleware/auth_placeholder.rs`](../../philharmonic-api/src/middleware/auth_placeholder.rs)
stays a no-op until the gate's done.

## Notes

- This is exactly the kind of question the morning Codex
  prompt's "Missing-context gating" section was designed to
  catch (the prompt's gating list said: "If any
  architecturally-significant surprise: STOP and flag"), and
  the gating language was inherited correctly from the prior
  rounds. Sub-phase A doesn't trip the gate because A is
  non-crypto; this round would have, but I'm catching it
  during prompt drafting before any Codex run.
- Out-of-hours commentary: JST currently inside regular hours
  (Tue ~11:30), so this is fine to surface and act on
  synchronously today.
