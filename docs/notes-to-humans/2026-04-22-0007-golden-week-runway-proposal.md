# Proposal: Golden Week runway (what to ship before 2026-04-29)

**Date:** 2026-04-22

## Deadline context

Japanese Golden Week 2026: April 29 (Wed, Shōwa Day) through
May 6 (Wed, substitute for May 3). Effective "shop closed"
window starts **April 29** — seven calendar days from today
including today.

Picking the anchor as 2026-04-29 means **~7 working days** of
Claude-available time, with the constraint that Yuka's review
time shrinks as the deadline approaches.

## Where we are

Published on crates.io (as of today):

- Phase 0 (workspace setup) — done.
- Phase 1 (`mechanics-config` extraction) — done. `mechanics-
  config 0.1.0`, `mechanics-core 0.3.0`, `mechanics 0.3.0`.
- Phase 2 (`philharmonic-policy`) — done. `philharmonic-policy
  0.1.0` today. Two-gate crypto review cycle completed.

Not started: Phases 3–9. The v1 MVP definition in
ROADMAP §8 is roughly "reference deployment serving real
workflows" — not shippable in one week. The realistic ask for
the Golden Week deadline is **how much of Phases 3–9 can
land with confidence**, with the rest landing after the
break.

## Proposed plan for the week

Ordered by dependency, not by wall-clock. Each is a
Codex-dispatch candidate unless otherwise noted.

### Days 1–2: Phase 3 — `philharmonic-connector-common`

**Size estimate**: small-to-medium. Mostly type definitions
(`ConnectorTokenClaims`, `ConnectorCallContext`, `RealmId`,
`RealmPublicKey`, `RealmRegistry`, `ImplementationError` enum,
thin COSE wrappers over the `coset` crate).

**Crypto-review status**: **not a Gate-1/2 trigger.** Phase 3
defines thin type-safe wrappers over `coset`'s COSE types; the
actual COSE signing/verification lives in Phase 5
(`philharmonic-connector-client` and `-service`). The wrappers
are shell types, not crypto construction. I'll flag if any
borderline choice comes up during implementation.

**Deliverables**: the type catalogue, serde round-trip tests,
realm-registry lookup tests. No crypto primitives exercised
yet — those arrive in Phase 5.

**Claude-side prep** (I'll do today or tomorrow):
- Archive the Codex prompt under `docs/codex-prompts/`.
- Spawn via `codex:rescue`.

**Release**: `philharmonic-connector-common 0.1.0`.

### Days 3–5: Phase 4 — `philharmonic-workflow`

**Size estimate**: medium-large. Workflow engine with three
entity kinds (`WorkflowTemplate`, `WorkflowInstance`,
`StepRecord`), status transitions, nine-step execution
sequence, `SubjectContext` propagation, `StepExecutor` +
`ConfigLowerer` trait surfaces.

**Crypto-review status**: **not a Gate-1/2 trigger.** No
crypto primitives — but touches the audit boundary
(`StepRecord` subject content). The ROADMAP §6 pitfall "Step
record subject content records identifier and authority only,
never full injected claims" needs a clear test.

**Deliverables**: engine with `create_instance`, `execute_step`,
`complete`, `cancel`; full state-machine coverage; terminal-
state immutability; integration tests against
`philharmonic-store-sqlx-mysql` + `MockStore`.

**Release**: `philharmonic-workflow 0.1.0`.

**Dependency on policy**: needs `Tenant` / `MintingAuthority`
markers, which are now shipped at policy `0.1.0`. Clean.

### Days 5–7: Phase 5 Gate-1 proposal draft (no implementation)

**Recommendation**: don't start Phase 5 code before Golden
Week. It's the heaviest crypto phase — ML-KEM-768 + X25519
hybrid KEM, HKDF key derivation, AES-256-GCM with specific
AAD construction, COSE_Sign1 signing, COSE_Encrypt0
encryption, payload-hash binding. Two-gate review per the
crypto-review-protocol. Gate-1 + Gate-2 cycles took several
rounds for Phase 2's simpler SCK + `pht_` scope; Phase 5 is
strictly more surface.

**What I can do pre-break**: draft the Gate-1 crypto proposal
at
`docs/design/crypto-proposals/2026-04-Nx-phase-5-connector-triangle.md`
— primitives, construction order, HKDF inputs, AEAD AADs,
nonce scheme, zeroization points, key-rotation story, COSE
algorithm identifiers, test-vector plan. You review when
you're back; Gate-1 approval unblocks Codex dispatch.

**Not at risk**: the Gate-1 draft is a design document. If
you're away during Golden Week, no coding happens until you
sign off — exactly the semantics the protocol is designed
for.

### Parallel: ROADMAP.md kept current

Per HUMANS.md short-term TODO #1. I'll continue updating
ROADMAP after each phase lands, not as a follow-up.

## What I'd NOT start before the break

- **Phase 5 implementation** — blocked on Gate-1 approval,
  which requires your review time.
- **Phase 6 impls** — open questions in
  `14-open-questions.md §Per-implementation wire-protocol
  details` (LLM dialect selectors, SQL parameter binding,
  email submission shape, embed/vector-search result shapes)
  should ideally resolve before the first impl is written.
  Plus they depend on Phase 5.
- **Phase 7 parallel impls** — same blocker.
- **Phase 8 (`philharmonic-api`)** — depends on policy +
  workflow + connectors being in place.
- **Phase 9 (reference deployment)** — endgame.
- **Post-MVP refactor** (HUMANS.md item 3 — AI-coding
  best-practices template) — confirmed not-blocking per your
  note.

## Realistic finish line

At the end of Golden Week (May 6), with the plan above,
we'd be roughly:

- **Shipped**: Phases 0, 1, 2, 3, 4 (five of nine).
- **Gate-1 proposed**: Phase 5.
- **Blocked/queued**: Phases 5 (pending Gate-1), 6–9 (serial
  dependencies).
- **Reference deployment**: well out of scope; v1 MVP
  still requires the full chain through Phase 9.

The v1 MVP definition isn't something one pre-Golden-Week
week can reach. What it can do is set up the post-break
sprint well: Phase 5 approach signed off and ready to
dispatch as soon as you're back.

## Open questions for you

1. **Dispatch pacing.** Phase 2 took four submodule commits
   over two sessions plus review cycles. Phases 3 + 4 in a
   week is aggressive — comfortable with that, or prefer I
   slow down to one phase with polish rather than rush two?
2. **Phase 3 vs Phase 4 priority.** If only one makes it
   pre-break, which do you prefer? My lean: Phase 3 (types
   crate is smaller, higher-leverage — unblocks Phase 5 type
   definitions and any pre-MVP Phase 4 drafting).
3. **Gate-1 draft scope for Phase 5.** Should I draft the
   full connector-triangle construction in one proposal, or
   split into sub-waves (e.g. Wave A: COSE_Sign1 tokens,
   Wave B: hybrid KEM for COSE_Encrypt0)? Phase 2 used waves
   to shrink each review surface; same pattern may work here.
4. **Immediate next action.** If you're comfortable with
   the plan, I can start Phase 3 today: draft the prompt,
   archive it, dispatch `codex:rescue`. Just need a nod. If
   you want me to wait on all three questions above first,
   I'll hold.

## Not in this proposal

- Post-MVP workspace refactor (AI-coding best-practices →
  reusable template / crate). Your HUMANS.md notes this is
  explicitly not-blocking; queuing for the post-MVP phase.
- Bug triage on the design docs. Phase 9 has a
  "Documentation reconciliation" pass baked in; the
  conventions + terminology work we've done this week
  already absorbs most of it. Nothing urgent surfaced during
  the Phase 2 work.
- Open-question resolution for Phase 6 impls. Better handled
  when we know concrete consumers.

Awaiting your direction on the four questions above.
