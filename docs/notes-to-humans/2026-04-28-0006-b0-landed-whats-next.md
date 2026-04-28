# Sub-phase B0 landed — what's next toward 2 May

**Author:** Claude Code · **Audience:** Yuka ·
**Date:** 2026-04-28 (Tue) JST 13:40

## Where we are

Sub-phase B0 (ephemeral API token primitives in
`philharmonic-policy`) is fully done: Gate-1 approved,
code implemented, Codex audit findings fixed, Gate-2
approved (`ea43e24`). The crate is at 0.2.0 locally
(`[patch.crates-io]` bridges until Phase 8 close).

Sub-phase A (skeleton in `philharmonic-api`) landed this
morning (`51de953`).

**Two of the planned sub-phases are done on the first
working day.** Calendar to 5/2:

```
Tue 4/28 — today, afternoon remaining
Wed 4/29 — 昭和の日 holiday
Thu 4/30 — working day
Fri 5/1  — working day
Sat 5/2  — target date
```

## What's immediately next: sub-phase B1

**B1 — auth middleware in `philharmonic-api`.**

This is the consumer of B0's primitives. It replaces the
`auth_placeholder` middleware with real authentication:

1. Parse `Authorization: Bearer <token>` header.
2. Route on prefix:
   - `pht_`-prefixed → long-lived token: `parse_api_token`
     → SHA-256 hash → substrate lookup of `Principal` or
     `MintingAuthority` by credential hash → check not
     retired, tenant not suspended → build
     `AuthContext::Principal`.
   - Otherwise → ephemeral token:
     `verify_ephemeral_api_token` (B0 primitive) → authority
     lookup → authority-tenant binding check →
     authority-epoch check → build `AuthContext::Ephemeral`.
3. Populate `RequestContext.auth = Some(auth_context)`.
4. On any verify/lookup failure → generic HTTP 401 with
   `{"error":{"code":"unauthenticated",...}}` — no
   kid/expiry/signature details leaked externally.

**B1 is crypto-touching** (it calls `verify_ephemeral_api_token`),
so the code-review gate fires before merge. But B1 doesn't
introduce new crypto — it's a caller of already-reviewed
primitives. The review scope is smaller: correct call-site
usage, authority-tenant binding enforcement, error collapsing,
no key material in logs.

**B1 needs substrate access.** This is the first sub-phase
that pulls in `philharmonic-store` / `philharmonic-store-sqlx-mysql`
as dependencies on `philharmonic-api`. The builder pattern
from sub-phase A gains a store handle dependency.

**B1 Codex prompt** can be drafted and dispatched this
afternoon. Estimated Codex time: ~30-60 min. Yuka review
of B1 can happen tonight or tomorrow.

## Full remaining sequence to 5/2

| Sub-phase | Description | Crypto? | Blocker |
|-----------|-------------|---------|---------|
| **B1** | Auth middleware (consumer of B0) | Yes (caller) | None — dispatch now |
| **C** | Authz + tenant-scope enforcement | No | B1 landed |
| **D** | Workflow management endpoints | No | C landed |
| **E** | Endpoint-config CRUD + SCK decrypt | Yes (caller) | D landed |
| **F** | Principal/role/authority CRUD + token gen | No | E landed |
| **G** | Token minting endpoint | Yes (caller) | F landed |
| **H** | Audit + rate limit + admin/operator | No | G landed |
| **I** | Publish `philharmonic-api` 0.1.0 + `philharmonic-policy` 0.2.0 | No | H landed |

Plus Phase 9 tasks (test WebUI + binary targets + e2e
testcontainers scenario) which can start in parallel once
enough of Phase 8 is in for the bin targets to compile
(probably after F).

**Crypto-review gates remaining:** B1 (caller review), E
(SCK decrypt call site), G (mint call site). These are
lighter than B0's Gate-1 because B0 already passed the
approach + code review for the underlying primitives. B1/E/G
reviews scope to: correct call-site usage, no key material
leakage, correct error handling.

## Bottleneck analysis

- **Codex throughput:** Each sub-phase is ~30-60 min Codex
  time. If dispatched back-to-back, B1 through H could all
  land in one long working day (~6-8 hours of Codex time +
  Yuka review between rounds).
- **Yuka review time:** The real bottleneck. Three crypto
  call-site reviews (B1/E/G) plus normal code review on
  C/D/F/H. If Yuka can review one sub-phase every ~2 hours,
  the pipeline stays flowing.
- **GW:** Tomorrow is 昭和の日 (holiday). If B1 dispatches
  this afternoon and Yuka reviews tonight or on 4/30 morning,
  the pipeline picks up at C on 4/30.

## Recommended action right now

1. **I draft and dispatch B1** this afternoon. The prompt
   references the B1 handoff contract in the B0 proposal
   (authority-tenant binding, error collapsing, epoch check,
   `i64→u64` conversion rule).
2. **Yuka reviews B1 output** when it lands (tonight or 4/30
   morning).
3. **C dispatches as soon as B1 is approved.**
4. Pipeline continues back-to-back through the week.

Say "go" and I'll start drafting B1.
