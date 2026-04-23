# Phase 5 connector triangle — published

**Date:** 2026-04-23

## tl;dr

All four Wave B-gated crates are live on crates.io with signed
release tags. Publish sequence ran cleanly in dep order; every
`verify-tag.sh` reports local-signed-pushed-all-ok. ROADMAP
marked done.

| Crate | Version | Tag (short) | crates.io |
|---|---|---|---|
| `philharmonic-connector-common` | **0.2.0** | `v0.2.0` (4d50487) | published |
| `philharmonic-connector-client` | **0.1.0** | `v0.1.0` (b362f32) | published |
| `philharmonic-connector-service` | **0.1.0** | `v0.1.0` (c89b89b) | published |
| `philharmonic-connector-router` | **0.1.0** | `v0.1.0` (42d13bf) | published |

## Sequence

1. **Pre-flight** — `pre-landing.sh` green, all four submodules
   clean / on main / tracking origin, `crates-io-versions` run
   on each: common had 0.1.0; client/service/router HTTP 404
   (names never reserved). That means 0.1.0 is the *initial*
   published version for the three triangle crates, not a
   reservation bump.
2. **`check-api-breakage.sh philharmonic-connector-common`** —
   0.1.0 → 0.2.0 is pre-1.0 major, semver-checks reported "no
   semver update required" (0 pass, 252 skip — all checks
   skipped because the bump itself allows breakage). Expected.
3. **Prep commit** `e28e41c` (parent) +
   `efddde9`/`b6960a9`/`a81c778` (client/service/router): bumped
   Cargo.toml versions, moved each CHANGELOG `[Unreleased]` →
   `[0.1.0] - 2026-04-23` / `[0.2.0] - 2026-04-22` (common was
   already there), dropped the aspirational `## [0.0.0]` "name
   reservation" entries for the three triangle crates (those
   never happened), bumped `connector-service` dev-dep on
   `connector-client` from `"0.0.0"` (nonexistent on crates.io)
   → `"0.1"` so `cargo publish`'s dev-dep check resolves
   cleanly against the about-to-be-published 0.1.0. Pushed.
4. **Publish in dep order:**
   - `publish-crate.sh philharmonic-connector-common` →
     `Published 0.2.0 at registry crates-io`, signed tag
     `v0.2.0` created.
   - `publish-crate.sh philharmonic-connector-client` →
     `Published 0.1.0`, signed tag.
   - `publish-crate.sh philharmonic-connector-service` →
     `Published 0.1.0`, signed tag. The prior dev-dep bump
     meant dry-run resolved cleanly; no sequence retry needed.
   - `publish-crate.sh philharmonic-connector-router` →
     `Published 0.1.0`, signed tag. No inter-crate deps so
     this one could have gone any time after the others.
5. **`push-all.sh`** pushed the four new tags to origin (the
   branches were already caught up from step 3).
6. **`verify-tag.sh`** × 4 — all report *"local tag ok, signed
   ok, origin ok — signed and pushed"*.

## Decisions made during the run

- **Dropped the `[0.0.0]` CHANGELOG entries** for client /
  service / router. They said "Name reservation on crates.io.
  No functional content yet." but that event never actually
  happened — the names were cold on crates.io. Keeping the
  entries would have been misleading historical fiction, so
  they went.
- **`connector-service` dev-dep bumped in the prep commit**
  (before client was published). Works because the workspace's
  `[patch.crates-io]` table redirects `philharmonic-connector-client`
  to the local path, so in-workspace resolution still succeeds
  against the local `v0.1.0` member; `cargo publish` picks the
  crates.io version at publish time (by then published).
- **No `cargo audit` fix in this round.** The three pre-existing
  advisories flagged in the Round-02 audit note
  (`RUSTSEC-2023-0071` rsa via sqlx-mysql,
  `RUSTSEC-2026-0104` rustls-webpki via ureq/testcontainers,
  `RUSTSEC-2024-0436` paste via boa/mechanics) are not on Wave B
  crypto paths — per your earlier instruction, we cannot do
  anything on those right now, so I didn't try to block this
  publish on them.

## Artefacts

- ROADMAP.md updated in two passes: first marked Wave B
  Gate-2-approved + publish-in-progress (prep commit
  `e28e41c`), then marked publish done in the final commit
  alongside this note.
- Archive notes trail:
  `docs/design/crypto-approvals/2026-04-23-0001-phase-5-wave-b-codex-dispatch-complete.md`
  (Yuka's Gate-2),
  `docs/codex-prompts/2026-04-22-0005-*.md` (Wave B main dispatch),
  `docs/codex-prompts/2026-04-23-0001-*.md` (zeroization
  follow-up), `docs/notes-to-humans/2026-04-23-000{1,2}-*.md`
  (Claude's two audit passes).

## What's next

With Phase 5 closed, the ROADMAP advances to Phase 6
(first connector Implementation crates — the `philharmonic-
connector-impl-*` series). None of that is crypto-sensitive;
normal Codex dispatch workflow applies (no two-gate review).
Waiting on your direction for phase sequencing.
