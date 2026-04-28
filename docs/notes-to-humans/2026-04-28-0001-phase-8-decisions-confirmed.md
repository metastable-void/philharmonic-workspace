# Phase 8 — decisions confirmed (2026-04-28 Tue morning JST)

**Author:** Claude Code · **Audience:** Yuka · **Date:** 2026-04-28 (Tue) JST

Following up on
[`2026-04-27-0003-phase-8-design-and-decisions.md`](2026-04-27-0003-phase-8-design-and-decisions.md)
and the deployment-neutrality sweep (commit `74c82ce`). Yuka's
calls this morning, recorded for durability.

## Decisions accepted

**A — Sub-phase ordering A→I.** Accepted as recommended in the
design note §"Suggested sub-phase ordering". The plan moves
from suggestion to the canonical implementation order; ROADMAP
§Phase 8 updated in this commit.

**B — Substrate test backend: testcontainers-MySQL.** Approved.
Mirrors the SQL connector-impl pattern; uses
`serial_test`'s `#[file_serial(docker)]` to keep concurrent
container starts from OOMing the host. Same mental model and
muscle memory as Tier 1.

**C — Crypto-review-protocol approach gate.** Approved. See
§"Crypto-approach approval" below for the durable record.

**D — `RequestScope` trait shape.** Accepted as proposed:
- Single async method returning an enum (option a from the
  list).
- Plugged via `Arc<dyn RequestScopeResolver>` at app
  construction (option c — idiomatic axum).
- Sub-phase A's prompt pins the exact signature.

## Crypto-approach approval

**Yuka's call on 2026-04-28**: the approach gate for Phase 8's
crypto-touching sub-phases (B Auth, E Endpoint-config
encryption, G Token mint) is **APPROVED**.

**Reason for approval at the approach stage**: Phase 8
introduces **no new cryptography** at the framework level. It
consumes primitives that have already passed the workspace's
two-gate crypto-review protocol during their original
development:

- **COSE_Sign1 mint + verify** for ephemeral tokens — wave A
  primitives in `philharmonic-connector-common` and
  `philharmonic-policy`. The API crate is a caller, not an
  implementer.
- **Substrate credential key (SCK) encrypt + decrypt** of
  `TenantEndpointConfig` — wave B primitives. Same pattern.
- **API signing key kid rotation** — already specified in
  [`docs/design/11-security-and-cryptography.md` §"Key
  identifiers"](../design/11-security-and-cryptography.md#key-identifiers)
  as a 5-step procedure (generate → distribute → switch →
  overlap ≥ max token lifetime → retire). Phase 8 implements
  the procedure; it does not redesign it.

**What's still gated**: the code-level review for each
crypto-touching sub-phase (B, E, G) still applies before any
publish. That second gate is non-waivable and stays on the
calendar:

- **Sub-phase B code review** before B's Codex output is
  merged into main: review the mint-flow caller code for
  envelope-clipping correctness, ensure no key material
  leaks through error messages or logs, verify the kid
  resolver correctly reads from the deployment-supplied
  configuration.
- **Sub-phase E code review**: review the
  `TenantEndpointConfig` encrypt/decrypt call sites; verify
  the `endpoint:read_decrypted` permission is the only path
  to plaintext; verify ciphertext is never logged.
- **Sub-phase G code review**: review the token-minting
  endpoint's claim-clipping logic, the `authority_epoch`
  inclusion, the audit-record content (subject + authority
  only, not full claims).

**No new crypto allowed in Phase 8 without re-opening the
approach gate.** If a Codex round surfaces a need to
introduce new crypto (e.g. a different signing scheme,
custom KDF, anything not already in
`philharmonic-connector-common` or `philharmonic-policy`),
that is a hard-stop signal — Yuka picks the next move,
including possibly a fresh approach-gate cycle.

## Forward-looking: post-Phase-8 test infrastructure

Yuka's note this morning: **after Phase 8 lands, build a test
WebUI and binary targets to exercise the framework
end-to-end.**

Framing: these are *test artifacts*, not framework
prescriptions. Per the deployment-neutrality discipline, the
framework crates stay shape-agnostic. The test WebUI + binary
targets exist to:

- Stress-test every crate's contract end-to-end (API +
  workflow engine + connector layer + substrate + executor)
  against a known-good integration scenario.
- Demonstrate one concrete deployment shape so future
  consumers have a worked example to reference (without it
  being privileged).
- Validate the ergonomics of the trait surfaces from a
  consumer's perspective (i.e. "if I were building a
  deployment, would the trait surface make sense to me?").

**Where they live**: in-tree non-published crates, same
pattern as `xtask/`. A reasonable layout (sub-decision worth
confirming when we get there):

```
test-deployment/                 # in-tree, non-published
├── webui/                       # small browser-facing test UI
│   └── …
├── api-host/                    # binary that hosts philharmonic-api
│   └── …
├── connector-host/              # binary that hosts connector-service
│   └── …
└── README.md                    # how the pieces wire up; ONE example shape
```

Or alternatively under `examples/` (Cargo's conventional
location for example crates) — that's a naming decision for
when we get there.

**Where they sit on the roadmap**: ROADMAP §Phase 9
("Integration and reference deployment") is the natural home;
this commit updates §Phase 9 to mention the test WebUI +
binary targets explicitly. They're additive to what was
already there (testcontainers integration tests, reference
deployment); not a structural change.

## What's next today (2026-04-28)

In rough order:

1. **Now**: this note + ROADMAP updates landed (single commit).
2. **Then**: I draft sub-phase A's Codex prompt. Sub-phase A is
   non-crypto (skeleton + RequestScope trait + middleware
   plumbing + observability + error envelope), so the crypto
   gate doesn't fire on it.
3. **Then (your call)**: dispatch A late morning / early
   afternoon, OR archive the prompt and hold dispatch until
   2026-05-07 post-GW. Either is fine — sub-phase A is a
   long-running Codex round (axum scaffolding + multiple
   trait surfaces), and if we dispatch today it can finish
   in the background during GW with no further human attention
   needed (the work is fully non-crypto and Codex is
   instructed to stop on any architectural surprise).

## Where to read more

- [`2026-04-27-0003-phase-8-design-and-decisions.md`](2026-04-27-0003-phase-8-design-and-decisions.md)
  — original design note with sub-phase rationale.
- [`docs/design/10-api-layer.md`](../design/10-api-layer.md) —
  endpoint surface, auth/authz, hosting (now updated to
  framework-neutral framing).
- [`ROADMAP.md` §Phase 8](../../ROADMAP.md) — task list (this
  commit makes A→I canonical).
- [`docs/design/11-security-and-cryptography.md`](../design/11-security-and-cryptography.md)
  — key management, kid rotation procedure (referenced by
  the crypto approval).
