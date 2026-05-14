# D24 — workspace-wide `default-features = false` audit (round 03)

**Date:** 2026-05-14 (JST)
**Slug:** `d24-default-features-audit`
**Round:** 03 — supersedes round 02. Round 02 completed tier 1
edits but attempted `./scripts/commit-all.sh` and hit a sandbox
filesystem blocker (or, equivalently, the script's codex-guard
that aborts under any Codex ancestor process). Per `CLAUDE.md`:
**Codex itself never runs `commit-all.sh`** — gitwrite operations
are forbidden for Codex.
**Subagent:** `codex:codex-rescue`

## Round 03 = round 02 minus the commit instructions

Round 03 inherits **everything** from round 02
(`2026-05-14-0001-d24-default-features-audit-02.md`):

- **Decision rule per direct dep** (Step 1 / Step 2 / Step 3 —
  drop unused upstream defaults, keep only what grep attests as
  used OR what's re-exposed through the crate's `pub` API OR
  what's runtime-required and grep-invisible OR what would pull
  a banned dep).
- **Worked examples** (`boa_engine`, `uuid`, `hyper`, `rand`,
  `tokio-rustls`).
- **Heuristics for common heavy crates** (`tokio`, `axum`, `clap`,
  `tracing-subscriber`, `serde`, `serde_json`, `boa_engine`,
  `uuid`, `chrono`, `aes-gcm`, `ed25519-dalek`, `rand`).
- **Tier ordering** (7 tiers, leaf → root).
- **Per-crate version-bump policy** + **CHANGELOG template**.
- **Non-goals** (no new deps, no version pin bumps to deps, no
  src/ behaviour change, no `[profile.release]` edits, no
  `[features]` redesign in published crates, no HTTP-client /
  TLS-backend changes, no publishing, no raw git).
- **action_safety**, **missing_context_gating**,
  **completeness_contract**, **default_follow_through_policy**,
  **structured_output_contract** — same.

The **only delta** between round 02 and round 03 is the
commit-discipline.

## Commit discipline — Codex does NOT commit (binding)

Per `CLAUDE.md`:

> Codex itself never runs `commit-all.sh` (including
> `--dry-run` and `--exclude`); the codex-guard in the script
> aborts under any Codex ancestor process.

So for **every** workspace edit Codex makes in round 03:

1. Edit the file in the working tree.
2. Run the **per-crate `cargo check`** after each crate's edit:
   `CARGO_TARGET_DIR=target-main cargo check -p <crate>
   --all-targets`. This is the only verification Codex runs
   mid-flight. It MUST pass before moving to the next crate. If
   it fails, the trim was too aggressive — revert the specific
   feature drop and try again.
3. **Leave everything in the dirty working tree.** Do NOT call
   `./scripts/commit-all.sh`. Do NOT call `git commit`,
   `git add`, `git push`, `git stash`, or any other gitwrite.
   Read-only `git` calls (`git status`, `git diff`, `git log`)
   are fine.
4. Continue to the next crate.
5. At the end of the round, **DO NOT** run `pre-landing.sh`
   either. Claude runs `pre-landing.sh` after committing,
   because pre-landing.sh's behavior under a Codex ancestor
   process is undefined (and the script triggers commits via
   the hooks in a way that may collide with the guard).

The commit + push + pre-landing.sh runs are Claude's
post-Codex responsibility. Codex's responsibility ends at:

- Tier 2–7 edits applied to working tree.
- Tier 2–7 per-crate `cargo check -p <crate> --all-targets`
  passing for every touched crate.
- CHANGELOG entries added to each patch-bumped crate's
  `CHANGELOG.md`.
- Tier 5/6/7 stub crates (empty deps) explicitly noted as
  no-ops in the structured output.
- `./scripts/status.sh` showing the dirty tree (Codex SHOULD
  run this read-only command at the end to confirm scope).

## Tier 1 already landed — Codex picks up at tier 2

Round 02's tier 1 edits landed via Claude as:

- Parent commit: `fda23d2` "D24 audit tier 1: default-features
  = false sweep on leaf crates"
- Submodule commits: `philharmonic-types` `67740c2`,
  `philharmonic-store` `d426627`, `mechanics-config` (commit
  SHA visible via `git -C mechanics-config log -1 --oneline`).

The three tier-1 crates are now at:

- `philharmonic-types` 0.3.7
- `philharmonic-store` 0.1.3
- `mechanics-config` 0.1.2

CHANGELOG entries are in place.

**Tier 2 onward is the round-03 scope.** Specifically:

- **Tier 2** (first non-leaf foundations):
  - `mechanics-http-client` — already mostly disciplined;
    re-audit per the round-02 decision rule and trim more if
    grep allows.
  - `mechanics-http-server` — already mostly disciplined;
    same.
  - `mechanics-core`
  - `philharmonic-connector-common`
  - `philharmonic-connector-impl-api`
  - `dockerlet` — already disciplined; verify no edit needed.
- **Tier 3**:
  - `mechanics`
  - `philharmonic-policy`
  - `philharmonic-workflow`
  - `philharmonic-store-sqlx-mysql`
  - `philharmonic-connector-client`
  - `philharmonic-connector-service`
  - `philharmonic-connector-router`
- **Tier 4** (connector impls):
  - `philharmonic-connector-impl-http-forward`
  - `philharmonic-connector-impl-llm-openai-compat`
  - `philharmonic-connector-impl-llm-anthropic` (stub — verify)
  - `philharmonic-connector-impl-llm-gemini` (stub — verify)
  - `philharmonic-connector-impl-sql-postgres`
  - `philharmonic-connector-impl-sql-mysql`
  - `philharmonic-connector-impl-dns` (stub — verify)
  - `philharmonic-connector-impl-email-smtp` (stub — verify)
  - `philharmonic-connector-impl-embed`
  - `philharmonic-connector-impl-vector-search`
- **Tier 5** (top tier):
  - `philharmonic` (meta-crate; `[features]` block off-limits)
  - `philharmonic-api`
- **Tier 6** (bins, non-published):
  - `bins/mechanics-worker`
  - `bins/philharmonic-connector`
  - `bins/philharmonic-api-server`
- **Tier 7** (xtask, separate target dir):
  - `xtask`

## At the end of round 03

Return the structured output (per round 02's inherited
`<structured_output_contract>`):

1. Summary (2-3 sentences).
2. Touched files grouped by tier (tiers 2–7).
3. Patch-bumps issued (`crate@old → crate@new`) for every
   published crate whose Cargo.toml changed.
4. No-op crates (the four stubs + dockerlet if untouched).
5. Residual risks (features kept due to banned-dep risk,
   features kept as "runtime-required but grep-invisible"
   with their inline-comment reasons).
6. Verification results: per-crate `cargo check -p <crate>
   --all-targets` PASS for every touched crate. **Do not** run
   `pre-landing.sh` (Claude runs it).
7. Git state: report `./scripts/status.sh` output showing the
   dirty tree. **No commits made.**
8. Outcome paragraph for this `-03.md` file.

Claude will then:

- Review the dirty tree.
- Commit per-tier (or all-at-once, Claude's discretion).
- Push.
- Run `./scripts/pre-landing.sh` + `./scripts/pre-landing.sh
  --xtask` + `cargo deny check bans` + banned-dep cargo-tree
  inverts.
- Fix-forward if any of those break (with another round if
  the fixes are substantive).
- Update this prompt's `## Outcome` section with the final
  state.

## Outcome

Pending — will be updated after Codex round 03 run and Claude's
post-Codex verification.

---

<task>
Workspace-wide `default-features = false` audit per ROADMAP
§3.J D24 — round 03, tiers 2–7. Round 02's tier-1 work
already landed at parent commit `fda23d2`; do not re-edit
tier-1 crates (philharmonic-types, philharmonic-store,
mechanics-config).

**Reference docs (authoritative if anything in this prompt
contradicts them):**

1. `docs/ROADMAP.md` §3.J **D24**.
2. `docs/codex-prompts/2026-05-14-0001-d24-default-features-audit-02.md`
   — round 02 prompt. **Read in full.** Round 03 inherits the
   decision rule, worked examples, heuristics, tier ordering,
   version-bump policy, CHANGELOG template, non-goals,
   action_safety, missing_context_gating,
   completeness_contract, default_follow_through_policy, and
   structured_output_contract verbatim. The only delta is
   commit discipline (see below).
3. `docs/codex-prompts/2026-05-14-0001-d24-default-features-audit-01.md`
   — round 01 prompt for the original tier ordering and
   reference shape.
4. `CONTRIBUTING.md` §§3.1, 4, 5, 10.9, 11.
5. `deny.toml`.
6. `CLAUDE.md` — specifically the "Codex itself never runs
   `commit-all.sh`" paragraph.

**Commit discipline (binding — DELTA from round 02):**

- **Codex does NOT commit.** Do NOT call
  `./scripts/commit-all.sh`. Do NOT call `git commit`,
  `git add`, `git push`, `git stash`, or any other gitwrite.
- Read-only `git` calls (`git status`, `git diff`, `git log`)
  are fine.
- Leave everything in the dirty working tree.
- Do NOT run `./scripts/pre-landing.sh` either — Claude runs
  it post-commit.
- Per-crate `CARGO_TARGET_DIR=target-main cargo check -p
  <crate> --all-targets` IS Codex's responsibility, run after
  each crate's edits.
- At end of round: run `./scripts/status.sh` (read-only),
  capture the dirty tree summary, and return the structured
  output.

**Goal (binding — same as round 02):**

For every direct dep in every workspace Cargo.toml across
tiers 2–7:

1. Set `default-features = false`.
2. Compute active feature set (upstream defaults ∪ existing
   explicit list).
3. Apply Step-2 grep check per feature.
4. Write new explicit `features = [...]` list containing only
   features that survive Step-3's keep-conditions:
   (a) grep-attested as used, OR
   (b) gates a `pub` API the depending crate re-exports, OR
   (c) runtime-required but grep-invisible (with inline
       `# kept: <reason>` comment), OR
   (d) dropping would pull a banned dep (with inline
       `# kept: trim pulls <banned-dep> via <chain>` comment).
   Otherwise: drop.
5. If the final list is empty → `features = []` (explicit
   empty array, not omitted).

Apply the same rule to internal workspace deps that have a
`[features]` block. For internal deps without a `[features]`
block, `default-features = false` is a no-op syntactically —
skip the edit.

**Per-crate version-bump policy (same as round 02):**

Patch-bump every published crate whose Cargo.toml is changed.
CHANGELOG entry per the template (round 01's preamble has the
verbatim template; round 02's tier-1 commits show the landed
shape). No bumps for xtask, bins/*, stub crates with empty
deps, or unchanged crates.

**Tier ordering & per-crate cargo check (same as round 02):**

Process tiers 2 → 7 in order. After each crate's edits:

```sh
CARGO_TARGET_DIR=target-main cargo check -p <crate> --all-targets
```

Must pass before moving on. If it fails, the trim was too
aggressive — revert the specific feature drop and re-check.

**Non-goals (same as round 02):**

No new deps; no version pin bumps to deps; no src/ behaviour
change; no `[profile.release]` edits; no `[features]`
redesign in published crates; no HTTP-client / TLS-backend
changes; no publishing; no gitwrite.

**Action-safety reminders:**

- Every cargo invocation needs `CARGO_TARGET_DIR=target-main`
  (or `target-xtask` for xtask/).
- POSIX-ish host: no bash-only constructs in shell.
- Run `./scripts/xtask.sh calendar-jp` at session start and
  again before returning, to ground the JST timestamp.
- If JST is outside regular hours (10:00–19:00, ext 21:00),
  add a one-line "(JST now HH:MM <day> — outside regular
  hours; proceeding.)" note in the final reply.

**Follow-through:**

Run the full audit through tiers 2 → 7 in this single round.
Don't stop after tier 4 and report "halfway done". The
`<default_follow_through_policy>` inherited from round 02 /
01 is binding.

If a single crate cannot complete (a dep's defaults expand to
a feature that crashes per-crate check, requiring non-trivial
investigation), skip that crate, note it in residual risks,
and continue with the rest. Don't block the whole sweep on
one crate.

**Structured output (inherited):**

At end of round 03, return:

1. Summary (2-3 sentences).
2. Touched files grouped by tier.
3. Patch-bumps issued (`crate@old → crate@new`).
4. No-op crates.
5. Residual risks (kept-features with reasons).
6. Verification results — per-crate `cargo check` PASS list.
   `pre-landing.sh` says "not run (Claude responsibility)".
7. Git state — `./scripts/status.sh` output showing dirty
   tree. **NO commits made.**
8. Outcome paragraph for `-03.md` file's `## Outcome` section.
</task>
