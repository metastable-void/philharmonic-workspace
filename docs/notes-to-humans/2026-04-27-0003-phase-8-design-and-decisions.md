# Phase 8 — `philharmonic-api`: design + decisions to settle

**Author:** Claude Code · **Audience:** Yuka · **Date:** 2026-04-27 (Mon) JST

Phase 7 Tier 1 landed end-to-end this evening. Per the ROADMAP
macro sequence, Phase 8 (`philharmonic-api`) is next, with
implementation starting on or after 2026-05-07 post-Golden-Week.
This note surfaces what's already settled, what needs Yuka's
calls before the first Codex dispatch, and a suggested sub-phase
ordering. Recommendations included so most items are
read-and-confirm rather than open design.

## Where Phase 8 starts from

[`docs/design/10-api-layer.md`](../design/10-api-layer.md) is
the authoritative spec — substantially settled, status §523
explicitly says so. Endpoint surface, auth/authz model,
ephemeral-vs-long-lived token mechanics, routing-as-deployment-
concern, rate-limiting policy, observability scope are all
already pinned down.

[`ROADMAP.md` §Phase 8](../../ROADMAP.md#phase-8--philharmonic-api)
sketches the implementation order: axum → subdomain routing →
auth middleware → authz middleware → endpoint handlers → rate
limit → audit → error envelope → pagination → observability.

What's NOT in either doc: how to *stage* the implementation
(it's a big phase — 30+ endpoints), what test strategy to use,
and a few smaller plumbing decisions.

## What needs Yuka's call (with recommendations)

### Decisions for tomorrow (2026-04-28 Tue, last day before GW)

These should be settled in the morning so Claude/Codex can
start prep work before the GW pause and dispatch sub-phase A
on 2026-05-07.

**1. Sub-phase structure for Phase 8.** The phase is too big
   for one Codex round. **Recommendation: A → H below.**
   Yuka confirms or revises the cuts. Each sub-phase is a
   discrete Codex round with its own pre-landing-green
   target.

**2. Substrate test backend.** The handlers exercise a real
   substrate via `philharmonic-store`. Three options:
   - **(a) testcontainers-MySQL.** Mirrors the SQL
     connector-impl pattern; same `serial_test`
     `#[file_serial(docker)]` discipline; we already have
     the muscle memory. **Recommended.** Slower but
     production-faithful.
   - (b) In-memory mock store (`#[cfg(test)]`-only impl of
     the store traits). Faster; tests less faithful.
   - (c) New SQLite-backed store crate
     (`philharmonic-store-sqlx-sqlite`). Useful long-term
     for self-hosted single-tenant deployments anyway, but
     out-of-scope for Phase 8.

**3. Crypto-review-protocol approach gate.** Phase 8 mints
   COSE_Sign1 tokens, signs them with the API signing key,
   and encrypts endpoint configs under the substrate
   credential key (SCK). Both are crypto-sensitive paths
   per CLAUDE.md and the
   [`crypto-review-protocol`](../../.claude/skills/crypto-review-protocol/SKILL.md)
   skill, which **mandates Yuka's personal review at the
   approach-design stage BEFORE any code is written**.
   Tomorrow's approach review needs to confirm:
   - The COSE_Sign1 mint flow uses the existing
     `philharmonic-connector-common` token-mint primitives
     (no new crypto in `philharmonic-api`).
   - The SCK encrypt/decrypt of `TenantEndpointConfig`
     uses the existing primitives from the wave-A/B
     crypto work.
   - The kid-rotation story for the API signing key is
     pinned (which key store, who rotates, how
     `authority_epoch` interacts).

   **Recommendation: 30 minutes Tuesday morning to walk
   the approach + a checklist before Claude drafts any
   sub-phase prompt.**

### Decisions that can wait until post-GW (2026-05-07+)

These don't block tomorrow's prep work; settle them as
sub-phase A's Codex prompt is drafted.

**4. Exact axum version + ecosystem pins.** axum is fast-
   moving (0.7 / 0.8). Pick the latest cooldown-clear at
   prompt time. Auxiliary crates: `tower`, `tower-http`,
   `governor` (rate limiting), `serde`, `tracing`,
   `tracing-subscriber`. Codex Phase 0 verifies cooldowns.

**5. Substrate schema migrations.** Phase 8 introduces
   several entity tables (`Principal`, `Role`,
   `RoleMembership`, `MintingAuthority`, `AuditEvent`,
   `WorkflowTemplate`, `WorkflowInstance`, etc. — see
   `09-policy-and-tenancy.md`). These are NEW tables that
   `philharmonic-store-sqlx-mysql` doesn't have yet.
   Schema-migration strategy options:
   - **(a) Embedded `sqlx::migrate!` macro** with
     `migrations/` directory in `philharmonic-store-
     sqlx-mysql`. Sqlx's standard pattern; idiomatic.
   - (b) Separate `philharmonic-substrate-migrate` xtask
     bin. More tooling.
   - **Recommendation: (a)** unless ergonomics surface a
     gap.

**6. Pagination cursor format.** Spec says "opaque cursor
   string". Recommendation: base64url-encoded JSON
   `{"after_id":"<uuid>","after_seq":<u64>}` or similar
   tuple matching the entity's stable order key. Pin
   the exact shape in sub-phase A.

**7. Rate-limit storage.** v1 is single-node token buckets
   (already settled in doc 10). Implementation choice:
   `governor` crate (de-facto standard) vs hand-rolled.
   **Recommendation: `governor`.**

**8. Operator-endpoint scope for v1.** Doc 10 mentions
   tenant creation/suspension lives on a separate ingress
   for operators. Question: ship those endpoints in
   Phase 8 or defer? **Recommendation: ship the minimum
   set needed to bring up a tenant for testing —
   `POST /v1/tenants`, `POST /v1/tenants/{id}/suspend`,
   `POST /v1/tenants/{id}/unsuspend`. Defer richer
   operator surfaces.**

**9. Atomic create-instance-and-mint endpoint** (the
   single open question in doc 10 §517). **Recommendation:
   defer per doc 10's own steer ("simpler to defer and
   let clients make two calls"). Add post-v1 if a real
   consumer needs it.**

**10. Web UI.** Out of scope for Phase 8 (the crate ships
    JSON-only HTTP). Open question for *separately*: who
    builds the Web UI, when, against what stack. Worth a
    sentence in the ROADMAP that acknowledges this isn't
    Phase 8's problem.

## Suggested sub-phase ordering

Each line is one Codex round (or one Claude housekeeping
step). Pre-landing green at each cut.

- **Sub-phase A — Skeleton.** axum app, subdomain routing,
  tenant resolver trait, middleware chain plumbing (no
  auth yet — placeholder), error envelope shape, request
  context type, observability middleware (correlation ID
  + structured logging). Crate at 0.0.0; not publishable
  yet.
- **Sub-phase B — Auth (long-lived + ephemeral).** Bearer
  parsing, `pht_` lookup, COSE_Sign1 verification,
  `AuthContext` enum, kid resolver wiring. Hits the
  crypto-review gate. Tests: round-trip mint→verify, all
  rejection paths (expired, wrong kid, retired authority,
  suspended tenant, etc.).
- **Sub-phase C — Authz + tenant scope.** Permission-atom
  evaluation against `AuthContext`, instance-scope check
  for ephemeral, tenant-match check across the board.
  Tests: matrix of (Principal × required-permission ×
  granted-permission), (Ephemeral × in-envelope ×
  out-of-envelope), instance-scope mismatch.
- **Sub-phase D — Workflow management endpoints.** Full
  surface from doc 10 §228. Wires `WorkflowEngine` into
  handlers. Tests: happy path + at least one
  authz-failure path per endpoint, per acceptance
  criteria.
- **Sub-phase E — Endpoint config management.** Full
  surface from doc 10 §270. **Hits the crypto-review
  gate** (SCK encrypt/decrypt). Tests: happy path,
  decrypted-read permission boundary, retire-then-call
  failure, rotation correctly versioning.
- **Sub-phase F — Principal + role + minting-authority
  CRUD.** Full surface from doc 10 §309-§354. Long-lived
  token generation (returns once, never persisted in
  plaintext). Tests: token-once-only behavior,
  retirement, rotation, role-membership grant + revoke.
- **Sub-phase G — Token minting endpoint.** Doc 10 §355.
  **Hits the crypto-review gate** (token signing).
  Includes audit recording + permission clipping. Tests:
  envelope clipping, instance-scope binding, lifetime
  cap, claim-size cap, reserved-claim rejection.
- **Sub-phase H — Audit + rate limit + tenant-admin
  endpoints.** Doc 10 §418, §430, §436. Operator
  endpoints (the minimum from decision 8). Tests: 429
  with `Retry-After` header, audit events round-trip
  through the substrate.
- **Sub-phase I — Publish.** `0.1.0` to crates.io via
  `./scripts/publish-crate.sh`. Doc reconciliation.
  ROADMAP marks Phase 8 done.

Sub-phases B, E, G are crypto-gated.

## Timeline placement

- **Tomorrow (2026-04-28 Tue):** decisions 1, 2, 3 above.
  Claude drafts sub-phase A's Codex prompt skeleton (no
  crypto yet, OK to prep without the approach review).
  Possibly dispatch A late afternoon if the prompt is
  ready and Yuka greenlights — A is non-crypto, so it
  can run during GW if you want it to (Codex just needs
  to be left running; no human review of crypto required
  at this stage).
- **GW (2026-04-29 Wed → 2026-05-06 Wed):** if A was
  dispatched, review the result early in the window
  (Tuesday 04-29 morning JST?) and queue B prep. GW is
  for rest; nothing more dispatched.
- **2026-05-07 Thu:** sub-phase B's crypto-review approach
  gate, then B dispatch.
- **~ two weeks of B–H** at one sub-phase per Codex round
  with Claude review between, depending on iteration
  count.
- **Sub-phase I publish:** mid-to-late May, contingent on
  no major surprises.

This timing assumes serial dispatch. Sub-phases F (CRUD)
and H (audit/rate-limit) are non-crypto and could be
parallelized with crypto-gated rounds — but only if
Yuka's review bandwidth holds. Default to serial.

## What this doc does NOT decide

- The exact wire shape of any individual endpoint's request
  / response JSON. Doc 10 specifies the surface; sub-phase
  prompts pin the bytes.
- The mechanics-executor stub for tests. That's a sub-phase
  D / I detail; can be a small in-tree mock.
- Whether `philharmonic-api` runs as one binary or splits
  later. Doc 10 §475 says one binary for v1; not
  re-litigating.
- Anything about Phase 9 (integration + reference
  deployment). That's its own design pass, post-Phase-8.

## Where to settle the decisions

Tomorrow morning JST: Yuka reads this note, marks the
recommendations she accepts, flags any she wants to
revise. Claude commits the decisions (either inline edits
to this note, or a follow-up note) and updates ROADMAP
§Phase 8 to reflect the sub-phase plan.
