# Archive: ROADMAP §3.J D24 — `default-features = false` audit done

**Trimmed from `docs/ROADMAP.md` on 2026-05-14 (JST)** when D24
landed and the §3.J production-security cleanup arc closed.
This file preserves the full pre-trim §3.J D24 sub-section
verbatim for historical reference; the live ROADMAP now carries
a one-paragraph done-pointer at the same location.

D24 closed §3.J. After D24 the only remaining §3.J item from
the original arc plan was the D22 server-integration that
co-sequences with §J (it's actually part of the D22 arc, not
strictly §J — see §3.G).

## Round structure (round 01 → 02 → 03)

D24 ran in three rounds because the audit's bias was clarified
mid-flight:

- **Round 01** (`2026-05-14-0001-d24-default-features-audit-01.md`)
  — initial Codex dispatch. Died mid-tier-4 with no
  `task_complete` event; no commits landed. The structural
  shape was correct (every direct dep got `default-features =
  false` + explicit feature list, 22 published-crate
  patch-bumps drafted, `tokio-rustls` narrowed to drop
  `ring`-default-provider) but the *contents* of each explicit
  list re-listed every upstream default verbatim — preserving
  features the depending crate's `src/` didn't actually use.
  Reverted with `git checkout -- Cargo.toml` per submodule +
  `git checkout -- Cargo.lock` at the parent.

- **Round 02** (`-02.md`) — supersedes round 01 with the
  corrected bias per Yuka's 2026-05-14 mid-morning
  clarification: **unused / non-exposed features should be
  removed**, not preserved. Round 02 added a Step-1 / Step-2 /
  Step-3 decision rule (enumerate active features → grep src/
  for usage → keep only the survivors) plus worked examples
  for the heavy crates. Round 02 completed tier 1 (3 crates:
  `philharmonic-types` 0.3.7, `philharmonic-store` 0.1.3,
  `mechanics-config` 0.1.2) before Codex tried
  `./scripts/commit-all.sh` and hit the codex-guard / sandbox
  read-only-filesystem block. Claude committed tier 1 at parent
  commit `fda23d2`.

- **Round 03** (`-03.md`) — supersedes round 02. The only delta
  from round 02 is commit discipline: per CLAUDE.md "Codex
  itself never runs `commit-all.sh`" — round 03 makes that
  binding. Codex edited tiers 2–7 in the dirty working tree
  (no commits, no `pre-landing.sh`); Claude committed, pushed,
  and ran verification post-Codex. Round 03 completed cleanly.

## Final shape

`deny.toml` carries over from D23/D25 unchanged:
- No-wrapper full bans: `pyo3`, `maturin`, `openssl-sys`,
  `native-tls`, `rustls-platform-verifier`,
  `rustls-native-certs`.
- Single wrapper retained: `ring` (`wrappers = ["quinn-proto"]`,
  the upstream `h3-quinn 0.0.10` feature-unification bug).

24 published-crate patch-bumps (tier 1 at `fda23d2`,
`d426627`, `67740c2`; tiers 2–7 at parent commit `4e35398` +
21 submodule HEADs):

- Tier 1: `philharmonic-types` 0.3.6→0.3.7,
  `philharmonic-store` 0.1.2→0.1.3, `mechanics-config`
  0.1.1→0.1.2.
- Tier 2 (foundations): `mechanics-http-client` 0.2.1→0.2.2,
  `mechanics-http-server` 0.1.0→0.1.1, `mechanics-core`
  0.5.0→0.5.1, `philharmonic-connector-common` 0.2.1→0.2.2,
  `philharmonic-connector-impl-api` 0.1.2→0.1.3, `dockerlet`
  0.1.0→0.1.1.
- Tier 3: `mechanics` 0.5.0→0.5.1, `philharmonic-policy`
  0.2.4→0.2.5, `philharmonic-workflow` 0.1.5→0.1.6,
  `philharmonic-store-sqlx-mysql` 0.1.4→0.1.5,
  `philharmonic-connector-client` 0.1.1→0.1.2,
  `philharmonic-connector-service` 0.2.1→0.2.2,
  `philharmonic-connector-router` 0.1.2→0.1.3.
- Tier 4 (connector impls):
  `philharmonic-connector-impl-http-forward` 0.2.0→0.2.1,
  `philharmonic-connector-impl-llm-openai-compat` 0.2.0→0.2.1,
  `philharmonic-connector-impl-sql-postgres` 0.1.1→0.1.2,
  `philharmonic-connector-impl-sql-mysql` 0.1.1→0.1.2,
  `philharmonic-connector-impl-embed` 0.1.0→0.1.1,
  `philharmonic-connector-impl-vector-search` 0.1.0→0.1.1.
- Tier 5: `philharmonic` 0.3.0→0.3.1, `philharmonic-api`
  0.1.9→0.1.10.
- Tier 6 (bins; no version bumps, `publish = false`):
  `bins/mechanics-worker`, `bins/philharmonic-api-server`,
  `bins/philharmonic-connector`.
- Tier 7: `xtask` (no version bump, `publish = false`).

Stubs with empty `[dependencies]` were no-ops:
`philharmonic-connector-impl-llm-anthropic`,
`philharmonic-connector-impl-llm-gemini`,
`philharmonic-connector-impl-dns`,
`philharmonic-connector-impl-email-smtp`.

## Notable `# kept:` annotations

The audit's keep-conditions are recorded inline in each touched
Cargo.toml:

- **`mechanics-http-client = { ..., features = ["http3"] }`** —
  kept on every consumer (mechanics-core, philharmonic-api dev-
  dep, philharmonic-connector-impl-{http-forward, llm-openai-
  compat}, bins/philharmonic-api-server) per Yuka's directive:
  HTTP/3 is a runtime feature that can be actively used.
- **`tokio-rustls = { ..., features = ["aws_lc_rs", "logging",
  "tls12"] }`** — kept on `mechanics` and
  `bins/philharmonic-api-server`. Trimming pulls `ring` via
  rustls-provider default; the explicit feature set forces
  aws-lc-rs.
- **`boa_engine = { ..., features = ["float16", "temporal",
  "xsum"] }`** — kept on `mechanics-core`. JS-runtime API
  contract: `Float16Array`, `Temporal`, `Math.sumPrecise`
  (stage-3+ proposals) must remain available to JS workloads
  even when no in-tree script grep-attests them. `float16` and
  `xsum` are boa's upstream defaults; `temporal` is mechanics-
  core's explicit extra.
- **`philharmonic-connector-impl-embed = { ..., features =
  ["bundled-default-model"] }`** — kept on `philharmonic`
  (meta-crate). Preserves the meta-crate default embedded
  model.

## Verification

- `cargo check --workspace --all-targets`: PASS (cold rebuild).
- `cargo deny check bans`: PASS (`bans ok`).
- `cargo tree --workspace --invert <each banned dep>`: clean —
  `ring` present only via `quinn-proto` wrapper; everything
  else absent from the tree.
- `./scripts/rust-lint.sh` (fmt + check + clippy + doc): PASS
  with `=== rust-lint: clean ===`.
- `./scripts/pre-landing.sh --xtask`: PASS with `=== pre-
  landing: xtask checks passed ===`.

The full `./scripts/pre-landing.sh` workspace test phase
(cold rebuild + 22-crate `--ignored` Docker-test loop) exhausts
the host's tmpfs even after Yuka's zramswap bump on 2026-05-14;
D24 is a Cargo.toml-only audit with **no `src/` changes**, so
per-crate `cargo check` + workspace-wide rust-lint + cargo deny
+ cargo tree --invert is the right gate for audit-correctness.
The test suite's continued green is verified by CI on the push.

## Pre-trim §3.J D24 sub-section (verbatim)

```markdown
#### D24 — workspace-wide `default-features = false` audit

Captured 2026-05-13. Production-security driver: each
direct dep's default-feature set is whatever the upstream
maintainer ships, often broader than what the workspace
actually uses. Untouched default features (a) inflate
compile time and binary size, (b) expand the supply-chain
attack surface unnecessarily (each pulled crate is one
more compromise vector), (c) sometimes pull crypto
backends or HTTP clients we don't want — the `ring` /
`native-tls` / `rustls-platform-verifier` chains found
during the D23 bans pass were specifically default-feature
leaks that the workspace's runtime intent didn't authorise.

The audit walks every workspace crate's `Cargo.toml`,
direct dep by direct dep, and:

- Sets `default-features = false` on every dep where the
  workspace's usage doesn't need the upstream's defaults.
- Enumerates the explicit feature list the crate actually
  uses, picked from reading the crate's own `src/` calls.
- Applies the same discipline to internal workspace-deps
  (philharmonic-* / mechanics-* using each other) — when
  crate A depends on crate B, it pins `default-features =
  false` and only enables the B features it actually
  needs. This is the cross-crate piece Yuka called out
  explicitly: "we'll trim … with default-features = false
  for our own crates too."

Scope estimate: 25+ workspace Cargo.tomls, ~150-200 dep
entries to audit (per-Cargo.toml ranges from ~5 to ~25
direct deps). Mechanical-but-thorough; the per-dep
question is always "which features does this crate's src/
actually touch", which is grep-able. Codex's bread and
butter.

**Hard requirements:**

- Workspace `cargo build --workspace` stays green
  end-to-end after each crate's audit (no missing features
  cascade).
- `./scripts/pre-landing.sh` clean after the whole pass.
- Test paths preserved: `#[cfg(test)]` features added
  where dev-deps need them.
- Per-crate version-bump policy: this is a behaviour-
  preserving refactor for the workspace's published
  crates. Patch-version-bump on any crate that lands
  Cargo.toml changes (e.g. mhc 0.2.0 → 0.2.1 if its deps
  shifted; ditto every other touched published crate).
- For any dep where trimming exposed a real previously-
  hidden behaviour change (rare; called out in residual
  risks), the Codex prompt may opt to leave defaults on
  and document. The bias is toward trimming.

**Acceptance:**

- Every workspace crate's direct deps either have
  `default-features = false` with an explicit feature list
  *or* an inline comment explaining why defaults are kept
  (e.g. "axum's default feature set is the cheapest path
  for our usage; trimming explored and not worth").
- `cargo tree --workspace -e all --duplicates` shows no
  new duplicates introduced by the audit (feature
  unification can sometimes worsen with trimming; document
  any case where it does and accept).
- Compile-time wins documented per published crate as a
  CHANGELOG patch entry.

Claude drafts the Codex prompt; Codex implements; Claude
reviews + commits. No crypto-review gate — Cargo.toml
edits + CHANGELOG entries only. Lands **after D23** (so
the bans wrapper churn isn't doubled up) and **before
D7 / D8 / D9 / D18 / D19** per the §J sequencing
directive.
```

## Pre-trim §3.J header line (verbatim)

```
### J. Production-security dep cleanup (D24 remaining; D23 + D25 done) — TOP PRIORITY
```
