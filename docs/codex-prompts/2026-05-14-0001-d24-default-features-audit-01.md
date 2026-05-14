# D24 — workspace-wide `default-features = false` audit

**Date:** 2026-05-14 (JST)
**Slug:** `d24-default-features-audit`
**Round:** 01 — initial single-round dispatch covering every
workspace crate's `Cargo.toml`.
**Subagent:** `codex:rescue`

## Motivation

D24 is the last remaining item in the ROADMAP §3.J
production-security dep-cleanup arc. D20 / D21 / D22-client +
D22-server-lib / D25 / D23 have all landed (2026-05-12 →
2026-05-13). After this dispatch, the §3.J arc closes and the
workspace returns to the originally queued Tier-2/3 connector
work (D7 SMTP, D8 Anthropic, D9 Gemini, D18 mechanics-core
refactor, D19 DNS).

The goal is **conservative**, **behaviour-preserving**, and
explicitly bounded by Yuka's 2026-05-14 directive:

> trimming unused features / forbidden deps; you must **not
> change behaviours** except removing **unused** or **forbidden**
> deps; not removing functional features or features enabled by
> default for production.

In other words: when in doubt, **keep the feature**. The audit
makes the workspace's feature surface **explicit**, not narrower
than what the code actually needs. The win is supply-chain
transparency, not microscopic compile-time savings.

## References (read in this order)

1. `docs/ROADMAP.md` §3.J → **D24** (the canonical spec — hard
   requirements, acceptance criteria, scope estimate).
2. `CONTRIBUTING.md`:
   - **§3.1** `[profile.release]` per-setting rationale — leave
     these blocks alone; this audit only touches `[dependencies]`
     / `[dev-dependencies]` / `[features]` (when a top-level
     feature gates a dep that's being trimmed).
   - **§4** Git workflow — `scripts/commit-all.sh` only; no raw
     `git commit`/`git push`/`git add`. **No `--no-verify`**.
   - **§5** Script wrappers — every cargo call goes through the
     appropriate wrapper. Never `cargo` without
     `CARGO_TARGET_DIR=target-main` (or via the wrappers, which
     set it).
   - **§10.9** HTTP-client stack split (`reqwest`+rustls-tls for
     runtime, `ureq`+rustls for `xtask/`). Don't change client
     choices here.
   - **§11** Pre-landing checks — run `./scripts/pre-landing.sh`
     once at the very end of the sweep.
3. `deny.toml` — the authoritative ban list. After this pass:
   - `pyo3`, `maturin`, `openssl-sys`, `native-tls`,
     `rustls-platform-verifier`, `rustls-native-certs` — all
     **no-wrapper full bans**. Must remain that way.
   - `ring` — **single wrapper retained**: `wrappers =
     ["quinn-proto"]` (the upstream `h3-quinn 0.0.10` feature-
     unification bug). Do **not** add new wrappers. If you find
     a feature whose trimming would introduce a new banned-dep
     path, **leave that feature alone** and note it in residual
     risks.
4. Recent exemplars of the discipline applied correctly — copy
   their shape rather than re-deriving:
   - `dockerlet/Cargo.toml` (0.1.0): `bollard`, `futures-util`,
     `tokio` all `default-features = false` + explicit features.
   - `mechanics-http-server/Cargo.toml`: `quinn`, `rustls`,
     `rcgen` (dev) all with explicit aws-lc-rs feature choices.
   - `mechanics-http-client/Cargo.toml`: `hyper-rustls`,
     `hickory-resolver` with explicit feature lists.
   - `mechanics-core/Cargo.toml`: `tokio = { version = "1",
     default-features = false, features = ["rt", "time"] }`.

## Context files pointed at

Every workspace Cargo.toml — 33 in total. Authoritative list from
`Cargo.toml`'s `[workspace] members`:

**Published submodule crates (24):**

- `philharmonic-types`
- `philharmonic-store`
- `philharmonic-store-sqlx-mysql`
- `mechanics-config`
- `mechanics-core`
- `mechanics`
- `mechanics-http-client`
- `mechanics-http-server`
- `philharmonic`
- `philharmonic-policy`
- `philharmonic-workflow`
- `philharmonic-connector-common`
- `philharmonic-connector-client`
- `philharmonic-connector-router`
- `philharmonic-connector-service`
- `philharmonic-connector-impl-api`
- `philharmonic-connector-impl-http-forward`
- `philharmonic-connector-impl-llm-openai-compat`
- `philharmonic-connector-impl-llm-anthropic` (currently stub
  `0.0.1`, deps block empty — likely a no-op for this audit;
  verify)
- `philharmonic-connector-impl-llm-gemini` (same — stub)
- `philharmonic-connector-impl-sql-postgres`
- `philharmonic-connector-impl-sql-mysql`
- `philharmonic-connector-impl-dns` (stub, `0.0.0`, empty deps —
  no-op; verify)
- `philharmonic-connector-impl-email-smtp` (stub, `0.0.1`, empty
  deps — no-op; verify)
- `philharmonic-connector-impl-embed`
- `philharmonic-connector-impl-vector-search`
- `philharmonic-api`
- `inline-blob`
- `dockerlet` (0.1.0 — already disciplined; verify nothing has
  drifted)

**In-tree, non-published (4):**

- `bins/mechanics-worker`
- `bins/philharmonic-connector`
- `bins/philharmonic-api-server`
- `xtask`

Plus the workspace root `Cargo.toml` (no direct deps in
`[workspace.dependencies]` today — verify).

## Goal

For every direct dep entry in every workspace `Cargo.toml`:

1. If the entry **already has** `default-features = false`,
   leave it alone unless its current explicit feature list is
   demonstrably wrong (missing a feature the crate's code
   actually calls into → add it; listing a feature the code
   never uses → keep it anyway, this is a **conservative**
   audit).
2. If the entry **does not** have `default-features = false`,
   decide:
   - Determine the upstream crate's default features at the
     version pin (look at `Cargo.lock` and/or the crate's
     `crates.io` page).
   - Grep the depending crate's `src/` (and `tests/`,
     `examples/`, `benches/` for dev-deps) for the APIs gated
     behind each default feature.
   - **For each default feature that is actually used** —
     re-list it explicitly in the new `features = [...]` array.
     The post-audit form is `default-features = false, features
     = [<every previously-defaulted feature that the code uses>,
     <every feature already in the existing explicit list>]`.
   - **For default features that are clearly unused** (no API
     call sites, no transitive enablement needed) — drop them.
   - **When unsure whether a default feature is used** — keep
     it. Add it to the explicit list. The audit's bias is
     **toward listing**, not toward trimming. This is the
     point Yuka emphasised this morning.
   - **For default features that would, if dropped, introduce
     a banned-dep path** (e.g. dropping a `rustls` feature that
     causes a fallback to `native-tls`) — leave the default
     feature on AND add an inline `# default-features kept:
     dropping <feat> pulls <banned-dep> via <chain>` comment.
3. Apply the same to **internal workspace deps**
   (philharmonic-* / mechanics-* / inline-blob / dockerlet
   referenced from another workspace crate). When crate A
   depends on crate B and B has a `[features]` block, A pins
   `default-features = false` on B and only enables the B
   features A actually needs. This is the cross-crate piece
   explicitly called out in ROADMAP D24:

   > "we'll trim … with `default-features = false` for our own
   > crates too."

   For internal crates **without** a `[features]` block (most of
   them), `default-features = false` is a no-op syntactically;
   skip the edit on those — adding it is noise.

4. **Workspace-internal crates with a `default = […]` feature
   set are sensitive.** `philharmonic` has a meta-crate default
   feature set enumerating connector impls; **do not change
   what's in `default = [...]`**. Only edit how downstream
   crates *consume* `philharmonic` (which is already
   `default-features = false` everywhere it's used — verify).

5. **xtask / bins/ stay non-published.** Their `default-features
   = false` edits are pure transparency wins (no version bump
   implications). xtask uses a separate target dir
   (`target-xtask/`) and gets validated with `pre-landing.sh
   --xtask`.

## Non-goals (explicit)

- **No new dep additions.** No new crates. No version bumps
  except the patch-bumps mandated by the per-crate-version-bump
  policy (below).
- **No version bumps to released-version pins** — e.g.
  `axum = "0.8.9"` stays `"0.8.9"`. Only the audited crate's
  *own* version bumps.
- **No code changes in `src/`** beyond what's strictly required
  to keep compile green after a feature trim (e.g. an unused
  import that lints under a trimmed feature). Functional behaviour
  is the contract. If a trim requires removing a `use` line,
  that's fine; if it requires changing what a function does, you
  trimmed wrong — revert and keep the feature.
- **No `[profile.release]` edits.** Those are per
  `CONTRIBUTING.md` §3.1; the canonical block is set and
  documented per-setting.
- **No `[features]` redesign in published crates** — only
  pure-consumption-shape edits to `[dependencies]` /
  `[dev-dependencies]`.
- **No `Cargo.lock` regen** as a separate task — `cargo build`
  will write it as needed; it lands as a side effect, not a
  goal.
- **No new lint suppressions, `#[allow(...)]` shims, or
  `cfg`-gate gymnastics.** If a trim trips clippy, the trim
  was wrong — keep the feature.
- **No HTTP-client / TLS-backend changes** — workspace stays on
  `reqwest`+rustls-aws-lc-rs / `ureq`+rustls for tooling /
  `hyper`+rustls-aws-lc-rs in mechanics-http-client. No native-tls.
  No ring. No platform-verifier. No native-certs.
- **No publishing.** Yuka publishes the patch-bumps via
  `scripts/publish-crate.sh` manually after reviewing the diff.
  Codex never invokes `publish-crate.sh`. Codex never invokes
  `push-all.sh` either — Claude pushes.

## Per-crate version-bump policy

For **each published crate** whose `Cargo.toml` is changed by
this audit (i.e. at least one `default-features = false` or
explicit-feature edit landed in `[dependencies]` /
`[dev-dependencies]`):

- **Patch-bump** the `version` field of that crate.
  - `philharmonic-api 0.1.9` → `0.1.10`
  - `philharmonic-types 0.3.X` → `0.3.X+1`
  - …etc., per `./scripts/crate-version.sh`.
  - Use `./scripts/xtask.sh crates-io-versions -- <crate>` to
    confirm published-vs-local versions and avoid colliding.
- **Add a CHANGELOG patch entry** in the crate's `CHANGELOG.md`
  describing the bump as a non-functional supply-chain
  transparency patch. Template:

  ```
  ## [<new-version>] - 2026-05-14

  ### Changed
  - Internal Cargo.toml audit: `default-features = false` set on
    direct dependencies with explicit feature lists for what the
    crate actually uses. No behaviour change. (D24)
  ```

  If a crate has no `CHANGELOG.md` today, create one with the
  standard header and this single entry. Don't backfill historical
  entries.

- **Internal workspace consumers of patch-bumped crates** (i.e.
  another workspace crate that depends on it) — **do NOT bump
  their dep-version pin**. The semver-compatible pin in
  `dependencies = "0.1"` covers the patch. The `[patch.crates-io]`
  block in the workspace root `Cargo.toml` already overrides to
  the local path for build purposes.

- **`xtask`, `bins/*`, `dockerlet 0.1.0`** — no version bump.
  - `xtask` and `bins/*` are `publish = false`.
  - `dockerlet 0.1.0` is already published with the canonical
    feature discipline; the audit is unlikely to touch its
    Cargo.toml, but if it does and the change is non-trivial,
    bump to `0.1.1` and CHANGELOG-entry it.
  - `inline-blob 0.1.0` — if Cargo.toml changes, bump to `0.1.1`.

- **Stub crates** (`philharmonic-connector-impl-dns`,
  `philharmonic-connector-impl-email-smtp`,
  `philharmonic-connector-impl-llm-anthropic`,
  `philharmonic-connector-impl-llm-gemini`) — if their `[dependencies]`
  blocks are empty (which they currently are), they are no-ops.
  No version bump. Note this in the Outcome section.

## Concrete tasks (sequencing)

Process crates in this dependency order (leaf → root) so that
intermediate `cargo check` runs catch breakage one crate at a
time, not as a cascading mess:

1. **Leaf foundations** (no internal workspace deps, or
   minimal):
   - `inline-blob`
   - `philharmonic-types`
   - `mechanics-config`
   - `philharmonic-store` (depends only on `philharmonic-types`)
2. **First tier**:
   - `mechanics-http-client` (foundational HTTP client; already
     mostly disciplined)
   - `mechanics-http-server` (already disciplined; verify)
   - `mechanics-core`
   - `philharmonic-connector-common`
   - `philharmonic-connector-impl-api`
   - `dockerlet` (verify only)
3. **Second tier**:
   - `mechanics`
   - `philharmonic-policy`
   - `philharmonic-workflow`
   - `philharmonic-store-sqlx-mysql`
   - `philharmonic-connector-client`
   - `philharmonic-connector-service`
   - `philharmonic-connector-router`
4. **Connector impls** (one at a time; the stubs are no-ops):
   - `philharmonic-connector-impl-http-forward`
   - `philharmonic-connector-impl-llm-openai-compat`
   - `philharmonic-connector-impl-llm-anthropic` (stub)
   - `philharmonic-connector-impl-llm-gemini` (stub)
   - `philharmonic-connector-impl-sql-postgres`
   - `philharmonic-connector-impl-sql-mysql`
   - `philharmonic-connector-impl-dns` (stub)
   - `philharmonic-connector-impl-email-smtp` (stub)
   - `philharmonic-connector-impl-embed` (heaviest — ndarray,
     tokenizers, tract; audit carefully but conservatively)
   - `philharmonic-connector-impl-vector-search`
5. **Top tier**:
   - `philharmonic` (meta-crate; `[features]` block is
     **off-limits**)
   - `philharmonic-api`
6. **Bins** (not published; pure transparency):
   - `bins/mechanics-worker`
   - `bins/philharmonic-connector`
   - `bins/philharmonic-api-server`
7. **xtask** (separate target dir; pre-landing.sh --xtask):
   - `xtask`

After each crate's edits, run:

```sh
CARGO_TARGET_DIR=target-main cargo check -p <crate-name> --all-targets
```

(use the wrapper if it exists; `scripts/lib/cargo-target-dir.sh`
sets the var). If `--all-targets` fails on a dev-dep trim, fix the
trim. If it fails on a src/ trim, fix the trim. Don't paper over
with `--no-default-features` flags at invocation — the
declarative shape in Cargo.toml is what we're auditing.

When all 33 are done, run:

```sh
./scripts/pre-landing.sh
./scripts/pre-landing.sh --xtask
```

Both must end with `=== pre-landing: all checks passed ===`.

Then run the duplicates check:

```sh
CARGO_TARGET_DIR=target-main cargo tree --workspace -e all --duplicates 2>&1 | tee /tmp/d24-dup-before-vs-after.txt
```

Compare against the pre-audit state (capture both). If the audit
**introduced** new duplicates, note them in residual risks and
keep the relevant feature. If it **removed** duplicates, note
the savings in the Outcome.

Then run the banned-dep tree-presence check:

```sh
for banned in ring native-tls rustls-platform-verifier rustls-native-certs openssl-sys pyo3 maturin; do
  echo "=== $banned ===";
  CARGO_TARGET_DIR=target-main cargo tree --workspace --invert "$banned" -e all --target all 2>&1 | head -40;
done
```

`ring` is allowed via the `quinn-proto` wrapper only. Everything
else must print "package not found in the dependency graph" (or
the equivalent cargo-tree negative message).

Then run `cargo deny check bans`:

```sh
CARGO_TARGET_DIR=target-main cargo deny check bans
```

Must be clean.

## Commit strategy

Commits **per dependency-tier batch** (1, 2, 3, 4, 5, 6, 7
above), not per individual crate. Each commit:

```
audit: Cargo.toml default-features cleanup — tier N (<N> crates)

Conservative behaviour-preserving audit per ROADMAP §3.J D24.
Crates touched: <list>.
Patch-bumps: <list of crate@version pairs>.

D24 round 01.
```

Use `./scripts/commit-all.sh` (NOT `--parent-only`; submodule
crates' Cargo.toml edits land inside their submodules, and the
parent picks up the pointer + CHANGELOG additions in the same
batch). `commit-all.sh` is the only path; raw `git commit` is
forbidden.

If `commit-all.sh` aborts (e.g. clippy fail), **fix the clippy
issue in the audit, do not work around it**. The audit's bias is
to KEEP features; clippy failures usually mean a feature was
trimmed too aggressively.

## Verification checklist (before declaring done)

- [ ] Every workspace Cargo.toml's direct deps either have
  `default-features = false` with an explicit feature list,
  *or* an inline comment explaining why defaults are kept (e.g.
  `# defaults kept: trimming pulls <banned-dep>`).
- [ ] All published crates with Cargo.toml changes have a
  patch-version bump and a CHANGELOG entry.
- [ ] `./scripts/pre-landing.sh` clean.
- [ ] `./scripts/pre-landing.sh --xtask` clean.
- [ ] `cargo deny check bans` clean.
- [ ] `cargo tree --workspace --invert <banned>` prints nothing
  (or `quinn-proto`-only for `ring`).
- [ ] `cargo tree --workspace -e all --duplicates` is no worse
  than pre-audit (document any new duplicates).
- [ ] No `[profile.release]` edits.
- [ ] No `[features]` redesign in any published crate.
- [ ] No version bumps to released-version pins of deps (only
  the audited crate's own version bumps).
- [ ] No `src/` behaviour change.
- [ ] All commits signed-off via `commit-all.sh`.
- [ ] No `push-all.sh` invocation (Claude pushes).
- [ ] No `publish-crate.sh` invocation (Yuka publishes).

## Outcome

Pending — will be updated after Codex run.

---

<task>
Workspace-wide `default-features = false` audit per ROADMAP
§3.J D24 — conservative, behaviour-preserving sweep across all
33 workspace `Cargo.toml` files.

**Reference docs (read in this order; they are authoritative
if anything in this prompt contradicts them):**

1. `docs/ROADMAP.md` §3.J **D24** — canonical spec.
2. `CONTRIBUTING.md` §§3.1, 4, 5, 10.9, 11.
3. `deny.toml` — banned-dep posture (pyo3, maturin,
   openssl-sys, native-tls, rustls-platform-verifier,
   rustls-native-certs are no-wrapper full bans; `ring` is
   wrapper-allowed for `quinn-proto` only).
4. The above prompt preamble (this file's `## …` sections).

**Goal:** For every direct dep in every workspace Cargo.toml,
set `default-features = false` and re-list every default feature
that the depending crate **actually uses** (grep src/ / tests/
/ examples/ for API call sites). **Bias: KEEP features when
unsure.** This is a transparency audit, not a minimisation audit.

**Per-crate version policy:** Patch-bump every published crate
whose Cargo.toml changes; add a CHANGELOG entry per the template
in the preamble. Do not bump dep-version pins of deps. Do not
publish.

**Order:** Process crates in dependency order (leaf → root) per
the 7-tier batching in the preamble. Run `cargo check -p
<crate> --all-targets` after each crate. Commit per tier.

**Verification:** `./scripts/pre-landing.sh` and
`./scripts/pre-landing.sh --xtask` both clean. `cargo deny check
bans` clean. `cargo tree --workspace --invert <banned>` empty
for every banned dep (or `quinn-proto`-only for `ring`). No new
`cargo tree --duplicates` entries vs. pre-audit baseline.

**Non-goals:** No new deps. No version bumps to dep pins. No
src/ behaviour change. No `[profile.release]` edits. No
`[features]` redesign in published crates. No HTTP-client / TLS-
backend changes. No publishing. No raw `git`. No `--no-verify`.

<default_follow_through_policy>
Codex is expected to land the **complete audit** in this round
unless it hits a hard blocker (e.g. workspace build wedged on
a fundamental issue, host out of disk). "Tier 3 done, tier 4
pending" is **not** a complete result — keep going through all
7 tiers in the same round. The audit is mechanical and parallel-
safe within tiers; only the cross-tier dependency cascade
matters for ordering.

If a single crate's audit genuinely cannot complete (e.g. a dep's
default features include a removed-upstream feature, requiring a
non-trivial Cargo.lock investigation), **skip that crate**,
note it in the Outcome's residual risks, and continue with the
rest. Don't block the whole sweep on one crate.
</default_follow_through_policy>

<completeness_contract>
"Complete" means:

1. Every workspace Cargo.toml has been visited (33 files).
2. Either edited per the rules above, or explicitly noted as
   "no-op" (stub crates with empty deps blocks; already-
   disciplined crates with no further edits needed).
3. All published-crate version bumps + CHANGELOG entries
   landed.
4. All 7-tier commits landed via `commit-all.sh`.
5. Final pre-landing.sh (both passes) clean.
6. Final cargo deny clean.
7. Final cargo tree banned-dep checks clean.
8. Outcome section of this prompt file updated with: (a) list
   of crates whose Cargo.toml changed, (b) list of patch-bumps,
   (c) any residual risks, (d) any new duplicates introduced,
   (e) commit SHAs.

If any of (1)–(7) is incomplete, the dispatch is INCOMPLETE.
Report INCOMPLETE clearly with what's done and what's left.
</completeness_contract>

<verification_loop>
After each crate's edits:
  CARGO_TARGET_DIR=target-main cargo check -p <crate> --all-targets

After each tier's commit:
  no extra verification needed — the per-crate check covered it.

At the end (after tier 7):
  ./scripts/pre-landing.sh
  ./scripts/pre-landing.sh --xtask
  CARGO_TARGET_DIR=target-main cargo deny check bans
  CARGO_TARGET_DIR=target-main cargo tree --workspace -e all --duplicates
  for each banned dep: CARGO_TARGET_DIR=target-main cargo tree --workspace --invert <dep>

Do not run `cargo fmt` / `cargo clippy` / `cargo test` directly —
`pre-landing.sh` handles them. Do not run `cargo build
--workspace` standalone as a "check" — the per-crate `cargo
check -p` covers it.

If a `cargo check -p <crate>` fails after a trim:
1. The trim was too aggressive. Revert the specific feature
   removal that caused the failure. Add the feature back to
   the explicit list. Re-run `cargo check -p <crate>`.
2. If it still fails, the failure is unrelated to D24. STOP
   and report.

If `pre-landing.sh` fails at the end:
1. Read the failure carefully. If a single crate's clippy /
   doctest / test caused it, that crate was likely
   over-trimmed.
2. Revert the offending trim in that crate, commit a fix.
3. Re-run pre-landing.sh.
</verification_loop>

<missing_context_gating>
If, before you start editing, the workspace state diverges from
the prompt's claims (e.g. a crate listed as a stub now has deps;
`deny.toml` has changed; a Cargo.toml is mid-edit with uncommitted
changes), STOP and report the divergence. Do not proceed.

`./scripts/status.sh` should print `(clean)` for the parent
repo and all submodules before you start. If it doesn't, STOP.

`grep -rn "default-features = true"` across workspace Cargo.tomls
should normally print nothing (Cargo's `true` is the implicit
default; nobody explicitly writes it). If you find one, that's
a curiosity to note — the audit makes it explicit anyway.
</missing_context_gating>

<action_safety>
- All git state changes via `scripts/commit-all.sh` only. No raw
  `git commit`, no raw `git push`, no raw `git add` outside the
  script. The script's pre-commit hooks enforce signoff +
  signature + Audit-Info trailer.
- **Never** `git commit --no-verify`, **never** `git
  --no-gpg-sign`. The hooks are non-negotiable.
- **Never** `git reset`, **never** `git rebase`, **never** `git
  amend`. History is append-only.
- **Never** invoke `./scripts/push-all.sh`. Claude pushes after
  reviewing.
- **Never** invoke `./scripts/publish-crate.sh`. Yuka publishes.
- Every `cargo` invocation needs `CARGO_TARGET_DIR=target-main`
  (or `target-xtask` for `xtask/`). The wrappers in `scripts/`
  set this; if you call cargo directly, set it yourself.
- POSIX-ish host: no `bash`-only constructs in any shell you
  invoke. The wrappers are POSIX `#!/bin/sh`.
- This host's `/tmp` is a tmpfs; large `target-main` writes go
  through `scripts/lib/cargo-target-dir.sh` which redirects to
  the appropriate disk location. Don't override.
- Resource pressure: at session start CPU 0.5%, load 0.00, mem
  65% available, swap 6.6%. Host is idle; you have headroom.
  Run `./scripts/xtask.sh resource-pressure` at the start of
  the long final `pre-landing.sh` run if you want to confirm
  the box is still idle.
</action_safety>

<structured_output_contract>
At the end of the dispatch, return:

1. **Summary** (2-3 sentences): what the audit accomplished;
   key numbers (crates touched / patch-bumps issued / features
   trimmed / features explicitly kept).
2. **Touched files**: full list, grouped by tier.
3. **Patch-bumps issued**: `crate@old-version → crate@new-version`
   for each. CHANGELOG entry confirmed for each.
4. **No-op crates**: crates visited but not changed (stubs,
   already-disciplined).
5. **Residual risks**:
   - Features explicitly kept due to banned-dep risk (list with
     reasoning).
   - Any new `cargo tree --duplicates` entries (with reasoning).
   - Any crate skipped due to hard issue (with reasoning).
6. **Verification results**:
   - `pre-landing.sh`: PASS / FAIL
   - `pre-landing.sh --xtask`: PASS / FAIL
   - `cargo deny check bans`: PASS / FAIL
   - `cargo tree --invert <banned>` per banned dep: PASS (empty)
     / FAIL (with output) / `quinn-proto`-only for `ring`.
7. **Git state**:
   - Final SHAs per tier commit (parent + each touched
     submodule).
   - `./scripts/status.sh` should be clean at the end.
   - **NOT pushed** — Claude pushes.
8. **Outcome paragraph** for the prompt-archive file: 4-6
   sentences summarising the round for posterity, ready to drop
   into `## Outcome` of this file.
</structured_output_contract>
</task>
