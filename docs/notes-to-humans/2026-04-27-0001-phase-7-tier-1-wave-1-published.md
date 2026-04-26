# Phase 7 Tier 1 wave 1 — published 2026-04-27

**Author:** Claude Code · **Audience:** Yuka · **Date:** 2026-04-27 (Mon) JST

Three of the four Tier 1 data-layer connectors shipped to
crates.io this morning, per Yuka's "publish parts of Tier-1
that don't need more attention" call. This resolves
ambiguity A from
[`2026-04-26-0001-phase-7-tier-1-next-steps-and-ambiguities.md`](2026-04-26-0001-phase-7-tier-1-next-steps-and-ambiguities.md)
in favor of split publishing (3 now, embed later) rather
than co-landing.

## What shipped

| crate | version | submodule tip | tag |
|---|---|---|---|
| `philharmonic-connector-impl-sql-postgres` | 0.1.0 | `cc4e991` | `v0.1.0` (signed, pushed) |
| `philharmonic-connector-impl-sql-mysql` | 0.1.0 | `5f8c59f` | `v0.1.0` (signed, pushed) |
| `philharmonic-connector-impl-vector-search` | 0.1.0 | `f75228e` | `v0.1.0` (signed, pushed) |

All three published via `./scripts/publish-crate.sh` through
the menhera-cooldown proxy. `verify-tag.sh` confirmed the
local tag, signature, and origin match for each.

These are first-time publishes — no `0.0.0` placeholder
crates.io entries existed for these names; the
`crates-io-versions` tool returned 404 prior to publish. (A
side effect: the docs that previously claimed "published as
0.0.0 placeholders" for the unpublished impl crates were
factually wrong; that section in
[`docs/design/03-crates-and-ownership.md`](../design/03-crates-and-ownership.md)
has been rewritten in the same commit as this note to drop
the placeholder-framing for crates that never had one.)

## Doc reconciliation in this commit

- [`README.md`](../../README.md) "Status" — 3 new lines in
  the published-crates list; Tier 1 paragraph rephrased
  from "publish held until embed lands" to "wave 1 shipped
  2026-04-27, wave 2 ships embed when the tract rewrite
  lands."
- [`ROADMAP.md`](../../ROADMAP.md) Phase 7 Tier 1 —
  per-crate state flipped to **published**; wave-1/wave-2
  framing introduced.
- [`docs/design/01-project-overview.md`](../design/01-project-overview.md)
  connector-layer status — wave 1 published; embed in
  wave 2.
- [`docs/design/03-crates-and-ownership.md`](../design/03-crates-and-ownership.md)
  — new "Phase 7 Tier 1 — published 2026-04-27 (wave 1)"
  section listing the three; previous "Published as 0.0.0
  placeholders (Phase 7+)" header dropped (no placeholders
  ever existed); replaced with "Pending publish" header.
- [`docs/design/08-connector-architecture.md`](../design/08-connector-architecture.md)
  vector-search line — "publish held" → "published".
- [`docs/design/14-open-questions.md`](../design/14-open-questions.md)
  — `sql_query` and `vector_search` settled-bullets carry
  the publish date instead of the locally-ready note.

## What's still open

The next-step plan from
[`2026-04-26-0001-phase-7-tier-1-next-steps-and-ambiguities.md`](2026-04-26-0001-phase-7-tier-1-next-steps-and-ambiguities.md)
is largely intact. The split-publish call (ambiguity A)
is now resolved; the other five remain:

- **B.** Embed reference model for the in-tree test
  fixture.
- **C.** Tract op-coverage verification strategy
  (early-fail vs. iterate).
- **D.** Codex round shape — clean rewrite vs. incremental
  migration.
- **E.** Tract test vectors — value-equality vs. invariants
  (and which reference-vector source).
- **F.** Manual vs. scripted publish wrapper for
  Tier 1 wave 2 + later tiers.

Recommendations on each remain in the 2026-04-26-0001
note. The next-session-first action is still: draft the
embed tract Codex prompt, archive it, then dispatch.
