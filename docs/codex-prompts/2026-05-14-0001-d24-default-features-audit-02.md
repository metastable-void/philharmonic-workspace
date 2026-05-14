# D24 — workspace-wide `default-features = false` audit (round 02)

**Date:** 2026-05-14 (JST)
**Slug:** `d24-default-features-audit`
**Round:** 02 — supersedes round 01, which died mid-tier-4 and
was reverted (see round 01's `## Outcome` section).
**Subagent:** `codex:codex-rescue`

## Why round 02 — corrected bias

Round 01 (`-01.md`) was a structural pass: every direct dep got
`default-features = false` + an explicit feature list that
**re-listed every upstream default verbatim**. The structural
shape was correct, but the *contents* of each explicit feature
list were too conservative: Codex preserved features the
depending crate's `src/` doesn't actually use.

Yuka clarified on 2026-05-14 mid-morning:

> "unused / non-exposed features should be removed."

So the audit's bias is **not** "preserve every default
explicitly." It is:

- **Drop features the crate's `src/` doesn't exercise** — both
  upstream defaults *and* any feature already in the existing
  explicit list whose call sites disappeared since it was added.
- **Drop features that aren't re-exposed** through the crate's
  own `pub` API surface (i.e. a feature whose only purpose was
  to enable an API that this crate doesn't re-export and doesn't
  call internally is dead weight).
- **Keep features that are runtime-required** but invisible to
  `cargo check` (logging emit paths, default RNG sources, format
  parsers used through trait objects, serde format variants used
  by downstream crates, build-script-time features that affect
  generated code).
- **Keep features that, if dropped, would pull a banned dep**
  (the `quinn-proto`-via-`ring` wrapper is the only remaining
  exception; everything else must stay banned-dep-free).
- **Keep features that are part of the crate's published API
  surface** — e.g. if `philharmonic-types` re-exports an API
  gated behind `uuid/v7`, keep `v7` even if no in-crate code
  paths use it, because downstream consumers might.

This is a more substantial pass than round 01. The grep work is
the audit's main value: per dep, list the features active today
(defaults + explicit), then for each one decide
**keep / drop / kept-for-banned-dep-risk** based on grep
evidence.

## References (read in this order)

1. `docs/ROADMAP.md` §3.J **D24** — canonical spec.
2. `docs/codex-prompts/2026-05-14-0001-d24-default-features-audit-01.md`
   — the round 01 prompt and outcome. Read both — the round 01
   prompt's preamble has the tier ordering, version-bump policy,
   commit strategy, verification commands, and structured-output
   contract; round 02 inherits all of those unchanged. Only the
   feature-list selection rule differs (see "Corrected bias"
   above).
3. `CONTRIBUTING.md` §§3.1, 4, 5, 10.9, 11.
4. `deny.toml` — banned-dep posture (unchanged).

## What carries over from round 01 (unchanged)

- The **tier ordering** (7 tiers, leaf → root, processed in
  order, `cargo check -p <crate> --all-targets` after each crate).
- The **per-crate version bump policy** — patch-bump every
  published crate whose Cargo.toml is changed by this audit;
  CHANGELOG entry per the template; no bumps for `xtask`,
  `bins/*`, or unchanged crates.
- The **commit strategy** — one commit per tier batch, via
  `./scripts/commit-all.sh` (NOT `--parent-only`), with the
  message template documented in round 01's preamble.
- The **non-goals** — no new deps, no version pin bumps to deps,
  no src/ behaviour change, no `[profile.release]` edits, no
  `[features]` redesign in published crates, no HTTP-client /
  TLS-backend changes, no publishing.
- The **verification suite** — final `./scripts/pre-landing.sh`
  + `./scripts/pre-landing.sh --xtask` + `cargo deny check bans`
  + `cargo tree --workspace --invert <banned>` per banned dep
  + `cargo tree --workspace --duplicates` comparison.
- The **action_safety**, **missing_context_gating**,
  **completeness_contract**, **default_follow_through_policy**,
  and **structured_output_contract** blocks — same.
- The **CHANGELOG template** — same.

The only changes between round 01 and round 02 are: (a) this
bias clarification, and (b) the worked examples below.

## Decision rule per direct dep

For each direct dep in each workspace Cargo.toml:

### Step 1 — enumerate current active features

Take the union of:

- The upstream crate's `default = [...]` (look in
  `~/.cargo/registry/src/.../<crate>-<version>/Cargo.toml`, or
  `cargo info <crate>@<version>`).
- The existing explicit `features = [...]` array in the
  depending Cargo.toml.

That set is what's active **today**.

### Step 2 — grep evidence per feature

For each feature in the active set, decide:

- **Pure plumbing features** (`std`, `alloc`, `derive` on serde,
  `error-in-core`, etc.) — keep without grep work; these are
  load-bearing for almost every Rust API surface.

- **Capability features** (a feature that gates a specific API
  module — `uuid/v4` gates `uuid::Uuid::new_v4`; `tokio/macros`
  gates `#[tokio::main]`; `clap/derive` gates `#[derive(Parser)]`)
  — grep the depending crate's `src/`, `tests/`, `examples/`,
  `build.rs`, and `benches/` for call sites of the gated API.
  Tools:
  - `rg '<api-call-pattern>' <crate>/src/ <crate>/tests/`
  - `rg 'use <upstream>::<module>' <crate>/`
  - For derive-style features: `rg '#\[derive\(<DeriveName>\)' <crate>/`
  - For runtime features (tokio's `process`, `signal`, `net`):
    `rg '<upstream>::<module>::' <crate>/`

  If grep finds zero hits in the depending crate AND the feature
  is not re-exposed through `pub` API → **drop**.

  If grep finds hits → **keep**.

  If grep is ambiguous (e.g. feature gates an internal type
  used by trait-object dispatch invisible to grep) → **keep**
  and add `# kept: <reason>` inline comment.

- **Re-exposed API features** — when the depending crate is a
  *library* (most workspace crates are), check whether the
  feature's gated API is re-exported by the depending crate's
  own `pub use` / `pub mod` / `pub fn`. If so, downstream
  consumers depend on it → **keep**, regardless of in-crate
  call sites.

- **Banned-dep-risk features** — features whose removal would
  cause cargo's feature unification to fall back to a banned
  default (the `tokio-rustls` case where dropping the explicit
  rustls-provider features falls back to `ring`). Verify with
  `cargo tree --workspace --invert ring` after the trim; if a
  new path appears, **keep the feature** and add an inline
  `# kept: trim pulls <banned-dep> via <chain>` comment.

- **Heuristics for the common heavy crates**:
  - `tokio` — keep only the features whose call sites appear in
    `rg 'tokio::(<feature>|task::|net::|signal::|process::|sync::|fs::)' <crate>/`. Drop `process`, `signal`, `fs`, `net`, etc. if not grepped.
  - `axum` — defaults include `tokio`, `http1`, `json`,
    `matched-path`, `original-uri`, `form`, `query`, `tower-log`,
    `tracing`. Grep `Json(`, `Form(`, `Query(`, `MatchedPath`,
    `OriginalUri`, etc. Drop whatever isn't grep-attested.
  - `clap` — defaults include `std`, `color`, `help`, `usage`,
    `error-context`, `suggestions`. CLI bins generally want all
    of these; libraries that use clap programmatically may not.
  - `tracing-subscriber` — defaults include `std`, `fmt`,
    `tracing-log`. Grep for `tracing_subscriber::fmt`,
    `with_log_compat`, etc.
  - `serde` — `derive` + `std` is almost always the only set
    needed; `alloc`-only is rare.
  - `serde_json` — `std` is almost always needed.
  - `boa_engine` — `default = ["float16", "xsum"]`. Mechanics-
    core's JS workload runs `Math.sumPrecise`? `Float16Array`?
    If grep doesn't show evidence (test fixtures, in-source JS
    literals), drop both.
  - `uuid` — `v3`/`v4`/`v5`/`v6`/`v7` are each independent
    capability features. Grep for `Uuid::new_v3` /
    `Uuid::new_v4` / `Uuid::new_v5` / `Uuid::now_v6` /
    `Uuid::now_v7`. Keep only what's grepped (plus what's
    re-exported as part of the crate's `pub` API).
  - `chrono` — defaults are big (`clock`, `std`, `serde`, ...).
    Most crypto/store crates only need `std` + `serde`.
  - `aes-gcm` — `aes` + `alloc` + `getrandom` is the standard
    set; verify each is used.
  - `ed25519-dalek` — `fast` is a performance feature (preferred
    on x86_64); keep. `std` keep. `zeroize` keep for crypto. So
    the upstream default `["fast", "std", "zeroize"]` is
    typically all needed — listing it explicitly is fine.
  - `rand` (0.10) — `std_rng`, `sys_rng`, `thread_rng` each gate
    different APIs. Grep for `rand::thread_rng()`,
    `rand::rngs::StdRng`, etc.

### Step 3 — write the explicit feature list

After Step 2, the explicit `features = [...]` array contains
only features marked **keep** or **kept-for-banned-dep-risk**.
Format alphabetically (cargo doesn't care about order, but
sorted lists are easier to review).

If the final list is empty AND the upstream default was already
empty, write `default-features = false, features = []`. The
explicit `features = []` makes the intent clear and avoids the
diff churn of removing it later.

## Worked examples (use these to calibrate)

### Example 1 — `mechanics-core` / `boa_engine`

Before (round 01):
```toml
boa_engine = { version = "0.21.0", default-features = false, features = ["float16", "temporal", "xsum"] }
```

Grep check:
- `rg 'Float16Array|float16|f16' mechanics-core/src/` → expect:
  no hits.
- `rg 'sumPrecise|xsum' mechanics-core/src/` → expect: no hits.
- `rg 'temporal|Temporal' mechanics-core/src/` → check; if no
  hits and the workspace's connector layer doesn't depend on
  Temporal calendar math, drop.
- Otherwise (the lowerer uses `Temporal.PlainDate`?), keep.

Likely round 02:
```toml
boa_engine = { version = "0.21.0", default-features = false, features = [] }
# or features = ["temporal"] if grep attests
```

### Example 2 — `philharmonic-types` / `uuid`

Before (round 01):
```toml
uuid = { version = "1.23", default-features = false, features = ["serde", "std", "v4", "v7"] }
```

Grep check:
- `rg 'Uuid::new_v4|Uuid::new_v7|now_v7|new_v4' philharmonic-types/src/`
- Check `pub` re-exports: does `philharmonic-types` re-export
  uuid's API? Probably yes (cornerstone types crate).
- `serde` and `std` are plumbing; keep.

Likely round 02: keep all of `["serde", "std", "v4", "v7"]` if
the cornerstone-types crate's API surface uses v4 + v7 (which
the workspace pattern for entity IDs strongly suggests). If
either is unused → drop.

### Example 3 — `mechanics` / `hyper`

Before (round 01):
```toml
hyper = { version = "1", default-features = false, features = ["full"] }
```

Grep check:
- `full` is an umbrella enabling `client`, `server`, `http1`,
  `http2`. Does mechanics need both client and server? It's an
  HTTP server crate, so server + http1 + http2 are likely.
  `client` probably unused.

Likely round 02:
```toml
hyper = { version = "1", default-features = false, features = ["http1", "http2", "server"] }
```

### Example 4 — `philharmonic-policy` / `rand`

Before (round 01):
```toml
rand = { version = "0.10", default-features = false, features = ["std", "std_rng", "sys_rng", "thread_rng"] }
```

Grep check:
- `rg 'rand::thread_rng|rand::rngs::ThreadRng' philharmonic-policy/src/`
- `rg 'rand::rngs::StdRng' philharmonic-policy/src/`
- `rg 'rand::random' philharmonic-policy/src/`

Likely round 02: keep only the variants grep attests; drop the rest.

### Example 5 — `mechanics` / `tokio-rustls`

Before (round 01):
```toml
tokio-rustls = { version = "0.26", default-features = false, features = ["aws_lc_rs", "logging", "tls12"], optional = true }
```

This is a banned-dep-risk case. The upstream default for
`tokio-rustls` enables the `ring` crypto provider; the explicit
`["aws_lc_rs", "logging", "tls12"]` set forces aws-lc-rs and
drops `ring`. **Keep as-is** — this is a deliberate forbidden-
dep elimination from D20.

## Bin-target rules (xtask, bins/*)

- `xtask` — uses `target-xtask/`, validated by `pre-landing.sh
  --xtask`. The `xtask` http-client stack is `ureq + rustls`
  (not `reqwest`) — see CONTRIBUTING.md §10.9. Apply the same
  drop-unused rule. No version bump (publish=false).
- `bins/*` — same drop-unused rule, no version bump. Bins
  consume the meta-crate `philharmonic` and may have already-
  trimmed feature lists.

## What this round MUST produce

1. Every workspace Cargo.toml visited (33 files).
2. Direct deps either have `default-features = false` + an
   explicit features list (with `features = []` allowed if the
   trimmed-explicit list is genuinely empty), OR an inline
   `# defaults kept: <reason>` comment.
3. Patch-bump on every published crate whose Cargo.toml is
   changed.
4. CHANGELOG entry per patch-bumped crate, following the
   template in round 01's preamble.
5. One commit per tier batch (7 tiers).
6. Final `pre-landing.sh` + `pre-landing.sh --xtask` clean.
7. Final `cargo deny check bans` clean.
8. Final banned-dep `cargo tree --invert` checks clean
   (ring-via-quinn-proto only).
9. Outcome section of *this* prompt file (the `-02.md`) updated
   with: list of crates whose Cargo.toml changed, patch-bump
   list, residual risks, any new duplicates, commit SHAs.

## Outcome

Pending — will be updated after Codex round 02 run.

---

<task>
Workspace-wide `default-features = false` audit per ROADMAP
§3.J D24 — round 02 with corrected bias.

**Reference docs (authoritative if anything in this prompt
contradicts them):**

1. `docs/ROADMAP.md` §3.J **D24**.
2. `docs/codex-prompts/2026-05-14-0001-d24-default-features-audit-01.md`
   — round 01 prompt, especially its tier ordering, version-bump
   policy, commit strategy, verification suite, action_safety,
   completeness_contract, default_follow_through_policy,
   structured_output_contract, and CHANGELOG template. **Read
   round 01 in full** — round 02 inherits its operational rules
   verbatim and only refines the feature-list selection rule.
3. `CONTRIBUTING.md` §§3.1, 4, 5, 10.9, 11.
4. `deny.toml`.

**Corrected bias for round 02:**

Round 01 preserved every upstream default feature explicitly.
Yuka clarified the audit's intent (2026-05-14 mid-morning):
**unused / non-exposed features should be removed.** So round
02 applies the **Step-1 / Step-2 / Step-3 decision rule** in
the `## Decision rule per direct dep` section of this prompt.

The keep-conditions remain narrow and well-defined:
- (a) feature is grep-attested as used in `src/` / `tests/` /
  `examples/` / `build.rs` / `benches/`, OR
- (b) feature gates a `pub` API surface that the depending
  crate re-exports, OR
- (c) feature is runtime-required but grep-invisible (with
  inline `# kept: <reason>` comment), OR
- (d) dropping the feature would re-introduce a banned dep
  (with inline `# kept: trim pulls <banned-dep> via <chain>`
  comment).

Otherwise: **drop the feature.**

**Goal (binding):**

For every direct dep in every workspace Cargo.toml:

1. Set `default-features = false`.
2. Compute the active feature set (upstream defaults ∪ existing
   explicit list).
3. Apply Step-2 grep check per feature.
4. Write a new explicit `features = [...]` list containing only
   the features that survive Step-3's keep-conditions.
5. If the final list is empty → `features = []` (explicit
   empty array, not omitted).

Apply the same rule to internal workspace deps that have a
`[features]` block (philharmonic, philharmonic-connector-impl-embed,
mechanics, mechanics-http-client, bins/philharmonic-api-server).
For internal deps without a `[features]` block,
`default-features = false` is a no-op syntactically; skip the
edit (adding it is noise).

**Tier ordering, per-crate cargo check, commit-per-tier shape,
patch-bump policy, CHANGELOG template, verification suite,
action_safety, completeness_contract, follow_through_policy,
structured_output_contract** — **all inherited verbatim from
round 01**. Re-read round 01 for those.

**Non-goals (unchanged from round 01):** no new deps; no
version pin bumps to deps; no src/ behaviour change; no
`[profile.release]` edits; no `[features]` redesign in
published crates; no HTTP-client / TLS-backend changes; no
publishing; no `push-all.sh`; no raw `git`; no `--no-verify`.

**Action safety reminders (binding):**
- All git via `./scripts/commit-all.sh` (NOT `--parent-only`).
- Never `push-all.sh`, never `publish-crate.sh`, never raw `git`.
- Every cargo via `CARGO_TARGET_DIR=target-main` (or
  `target-xtask` for `xtask/`).
- Run `./scripts/xtask.sh calendar-jp` at session start and
  after each state-changing git op.
- If the JST wall-clock is outside regular hours (10:00–19:00,
  ext 21:00), add a one-line "(JST now HH:MM <day> — outside
  regular hours; proceeding.)" note in the next message.
- POSIX-ish host: no bash-only constructs in shell.

**Mid-flight aborts:** if Codex cannot complete the full sweep
in one round (e.g. hit a hard upstream issue, run out of
budget), commit what's been done up through the last green
tier, update this prompt's `## Outcome` section with what's
left, and report INCOMPLETE clearly. The `<completeness_contract>`
inherited from round 01 still applies.

**Structured output (inherited from round 01):**

At the end, return:
1. Summary (2-3 sentences).
2. Touched files grouped by tier.
3. Patch-bumps issued (`crate@old → crate@new`).
4. No-op crates.
5. Residual risks.
6. Verification results.
7. Git state (final SHAs per tier commit).
8. Outcome paragraph for this `-02.md` file.
</task>
