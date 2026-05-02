# Philharmonic Implementation Roadmap

**Audience**: Claude Code, working in the `philharmonic-workspace` repo.

**Purpose**: A linear plan for implementing the Philharmonic crate
family from current state to v1 MVP (working end-to-end deployment
serving real tenants).

**Current state** (2026-05-02):

- Design: complete.
- Phase 0 (workspace setup): **done**, with substantial added
  infrastructure beyond the original scope — see the Phase 0
  section below and §9.
- Phase 1 (`mechanics-config` extraction): **done** (2026-04-21).
  `mechanics-config 0.1.0`, `mechanics-core 0.3.0`, and
  `mechanics 0.3.0` are all published to crates.io with signed
  `v<version>` release tags inside their submodules. The
  mechanics-core bump from originally-drafted 0.2.3 to 0.3.0 was
  made after `check-api-breakage.sh` surfaced the type-identity
  change (schema types moved to `mechanics-config`, re-exported
  at the same paths) as a breaking change under cargo's pre-1.0
  semver rules — see
  `docs/notes-to-humans/2026-04-21-0006-mechanics-core-semver-checks-finding.md`.
  `mechanics` bumped in lockstep (0.2.0/0.2.1-unpublished →
  0.3.0) so downstream consumers opt into the new `mechanics-core`
  type identity explicitly rather than silently under a caret
  upgrade.
- Phase 2 (`philharmonic-policy`): **done** (2026-04-22).
  Published as `philharmonic-policy 0.1.0` with signed `v0.1.0`
  tag. Shipped: seven entity kinds
  (`Tenant`/`TenantEndpointConfig`/`Principal`/`RoleDefinition`/
  `RoleMembership`/`MintingAuthority`/`AuditEvent`) plus
  `TenantStatus` + `PrincipalKind` discriminants; three-way
  tenant-binding permission evaluation
  (`evaluate_permission`); `PermissionDocument` with parse-time
  atom validation against the 22-atom `ALL_ATOMS` list; SCK
  AES-256-GCM primitives (`Sck`, `sck_encrypt`, `sck_decrypt`)
  with wire format v1 and AAD binding over
  `tenant_id || config_uuid || key_version`; `pht_` API token
  format with Zeroize discipline. Delivered across two waves
  (non-crypto foundation at 2026-04-21 submodule commit
  `790c23d`; crypto foundation at `0085819`) plus auth-
  boundary hardening from an independent Codex security review
  (`1cde0e1` — cross-tenant role-confusion fix + atom
  validation) and zeroization hardening from Claude's Gate-2
  re-review pass (`7ca357e` — H1 + H2). Gate-1 approval at
  `docs/crypto/approvals/2026-04-21-phase-2-sck-and-pht.md`
  (amended 2026-04-22 for the `rand 0.10` swap), Gate-2
  approval at `-01.md` alongside.
- Phase 3 (`philharmonic-connector-common`): **done**
  (2026-04-22). Published as `philharmonic-connector-common
  0.1.0`. `0.2.0` (adds `iat` claim to `ConnectorTokenClaims`)
  publishes alongside the connector triangle 2026-04-23 —
  see Phase 5.
- Phase 4 (`philharmonic-workflow`): **done** (2026-04-22).
  Published as `philharmonic-workflow 0.1.0` with signed
  `v0.1.0` tag.
- Phase 5 Wave A (COSE_Sign1 authorization tokens): **landed
  2026-04-22**, Gate-2 approved. Client-side mint and
  service-side verify live in the triangle crates.
- Phase 5 Wave B (hybrid KEM + COSE_Encrypt0): **landed
  2026-04-23**, Gate-2 approved
  (`docs/crypto/approvals/2026-04-23-0001-phase-5-wave-b-codex-dispatch-complete.md`).
  Two Codex rounds — main implementation
  (`docs/codex-prompts/2026-04-22-0005-phase-5-wave-b-hybrid-kem-cose-encrypt0.md`)
  + zeroization/dead-code follow-up
  (`docs/codex-prompts/2026-04-23-0001-phase-5-wave-b-zeroization-followup.md`).
  Claude's two audit notes (round 1 +
  zeroization-delta) under
  `docs/notes-to-humans/2026-04-23-000{1,2}-*.md`.
- Phase 5 triangle publish: **done** 2026-04-23.
  `philharmonic-connector-common 0.2.0` (adds `iat` claim;
  breaking over 0.1.0), `philharmonic-connector-client 0.1.0`,
  `philharmonic-connector-service 0.1.0`,
  `philharmonic-connector-router 0.1.0` — all four on crates.io
  with signed `v0.2.0` / `v0.1.0` tags (verified via
  `verify-tag.sh`: local + signed + pushed, ok). First real
  releases for client/service/router (names were never
  reserved at 0.0.0 on crates.io).
- Phase 6 (API core): **done** (2026-04-24).
- Phase 7 Tier 1 (6 connector impls): **done** (2026-04-27).
  Tier 2–3 connectors (SMTP, Anthropic, Gemini) deferred to
  post-Golden-Week (on or after 2026-05-07).
- Phase 8 (end-to-end): **complete** (2026-04-28).
- Phase 9 (integration + deployment): **complete** (2026-05-02).
  Reference deployment operational; end-to-end workflow with
  `llm_openai_compat` connector tested successfully on the WebUI.

Work through phases in order unless a phase is explicitly noted
as parallel-safe. Consult the design documentation as the
authoritative source for architectural questions — **do not invent
architectural decisions**. If a design doc is wrong or incomplete,
update the doc first, then implement.

---

## 1. Project context

### What Philharmonic is

A workflow orchestration system built as independent Rust crates.
JavaScript workflows run in stateless Boa runtimes; external I/O
goes through per-realm connector services with per-step encrypted
authorization; storage is append-only and content-addressed in a
MySQL-family database.

### Layered architecture

Six layers, each with a clean dependency boundary upward:

1. **Cornerstone** (`philharmonic-types`) — shared vocabulary types.
2. **Storage substrate** (`philharmonic-store`, backends) —
   backend-agnostic storage traits.
3. **Execution substrate** (`mechanics-config`, `mechanics-core`,
   `mechanics`) — JS execution as an HTTP service.
4. **Policy** (`philharmonic-policy`) — tenants, principals,
   per-tenant encrypted endpoint configs, roles, minting
   authorities.
5. **Workflow** (`philharmonic-workflow`) — orchestration engine.
6. **Connector layer** (`philharmonic-connector-*`) — lowerer,
   router, service framework, and per-category implementations.
7. **API** (`philharmonic-api`) — public HTTP API.

### Workspace structure

You are operating inside `philharmonic-workspace`, a parent Git
repo containing:

- `Cargo.toml` — workspace manifest listing all members, shared
  third-party pins in `[workspace.dependencies]`, and a
  `[patch.crates-io]` block redirecting each Philharmonic crate
  dependency to its local submodule path. Crate manifests keep
  normal versioned deps so they remain independently
  buildable/publishable; the patch table rewrites them to local
  paths when building from the workspace.
- 26 submodule directories (25 crate submodules + docs-jp), each
  being a separate Git repo at `github.com/metastable-void/<name>`.
- `scripts/` — helper scripts (`status.sh`, `pull-all.sh`,
  `push-all.sh`, `commit-all.sh`).
- `docs/design/` (expected) — design documentation this roadmap
  references.

### Design documentation

**Authoritative.** Consult these before making any architectural
decision:

- `01-project-overview.md` — big picture.
- `02-design-principles.md` — cross-cutting commitments
  (append-only, content-addressed, backend-agnostic, stateless
  execution, layered ignorance, implementation uniformity).
- `03-crates-and-ownership.md` — crate catalog, dependency graph.
- `04-cornerstone-vocabulary.md` — `philharmonic-types` contents.
- `05-storage-substrate.md` — substrate traits and backends.
- `06-execution-substrate.md` — `mechanics-*` crates.
- `07-workflow-orchestration.md` — workflow layer, engine, traits.
- `08-connector-architecture.md` — capability/implementation/config
  model, wire protocols for every v1 implementation.
- `09-policy-and-tenancy.md` — entity kinds, permission atoms,
  minting authorities, API token format.
- `10-api-layer.md` — API endpoint surface and permission mapping.
- `11-security-and-cryptography.md` — threat model, crypto design,
  observability, error envelope.
- `12-deferred-decisions.md` — what's intentionally out of scope.
- `13-conventions.md` — redirect stub (development conventions
  moved to [`/CONTRIBUTING.md`](CONTRIBUTING.md) at the repo
  root).
- `14-open-questions.md` — still-open design questions.
- `15-v1-scope.md` — what ships with v1.

---

## 2. How to work

### Operating principles

**Design docs are authoritative.** Every architectural choice
traces to a passage in the design docs. If you want to deviate,
first update the relevant doc with reasoning, then implement
accordingly. Don't build silent exceptions.

**Submodule discipline.** Each crate is a separate Git repo. When
you change files inside a crate:
1. Commit inside the submodule on a feature branch.
2. Push to the submodule's origin.
3. In the parent workspace, `git add <submodule-dir>` to bump the
   pointer.
4. Commit and push the parent.

Never commit in the parent referencing an unpushed submodule
commit. The parent's Git config should have
`push.recurseSubmodules=check` set to catch this.

**Git via scripts.** All Git operations go through the helpers in
`scripts/` (`status.sh`, `pull-all.sh`, `commit-all.sh`,
`push-all.sh`). They encode the submodule-first ordering and the
signoff rule. If a script doesn't do what you need, extend it
first and update [`CONTRIBUTING.md §4`](CONTRIBUTING.md#4-git-workflow).

**Every commit is signed off *and* signed.** Commits in the
parent and every submodule must carry both a `Signed-off-by:`
trailer (DCO, via `-s`) and a cryptographic signature (GPG or
SSH, via `-S`). The scripts pass both flags and verify the
signature after committing — an unsigned commit triggers a
rollback. Non-negotiable.

**Pre-landing checks (mandatory).** Before committing any
change that touches Rust code, run:

```bash
./scripts/pre-landing.sh
```

It auto-detects modified crates (dirty submodules) and runs
`rust-lint.sh`, `rust-test.sh`, and `rust-test.sh --ignored
<crate>` for each modified crate — all three phases must pass.

`#[ignore]` is the project convention for tests that need real
infrastructure (testcontainers, live services); step 2 skips
them for speed, step 3 exercises them per-modified-crate. Don't
run raw `cargo fmt/check/clippy/test` when the scripts cover
the case. Clippy runs with `-D warnings` — fix the root cause,
don't silence at crate scope. See
[`CONTRIBUTING.md §11`](CONTRIBUTING.md#11-pre-landing-checks).

**Follow append-only discipline.** The substrate has no `UPDATE`
or `DELETE` semantics. Entity state changes are new revisions;
soft-deletes are revisions with an `is_retired` scalar set true.
If you find yourself wanting to "just update a field," re-read
`02-design-principles.md` and `05-storage-substrate.md`.

**MSRV and edition.** Edition 2024, `rust-version = "1.88"`, set
in every crate's own `Cargo.toml`.

**Licensing.** All crates: `license = "Apache-2.0 OR MPL-2.0"`.
Include `LICENSE-APACHE` and `LICENSE-MPL` in each submodule.

### What to stop and ask about

Stop and flag for Yuka's review rather than proceeding:

- **Anything in the crypto path.** SCK encrypt/decrypt, COSE_Sign1
  signing/verification, COSE_Encrypt0 encryption/decryption, the
  ML-KEM-768 + X25519 + HKDF + AES-256-GCM hybrid construction,
  payload-hash binding. Yuka reviews this code personally. See
  §7 "Crypto review protocol" below.
- **Schema changes after 0.1.0 publication.** Breaking changes to
  published crates need explicit decisions about bump cadence
  and migration.
- **Departures from the design docs.** If implementation reveals
  a design doc is wrong or ambiguous, update the doc with a
  clear note about what changed and why, then proceed.
- **Permission atom vocabulary additions.** If a new atom seems
  necessary, check whether an existing one covers the case
  first. Adding atoms has a maintenance cost.

### What you can decide yourself

- Internal implementation details (helper functions, private
  types, error-variant structure inside a crate).
- Test-suite organization.
- Third-party crate choices *within* what the design docs
  specify (e.g., "use `metrics` crate" is specified; which
  version of `metrics-exporter-prometheus` is yours to pick).
- Local code organization (module layout inside a crate).

---

## 3. Cross-cutting conventions

### Observability (from `11-security-and-cryptography.md`)

All crates use the `tracing` crate for logging with structured
fields:

```rust
use tracing::{info, warn, error, instrument};

#[instrument(skip(config), fields(tenant_id = %ctx.tenant_id))]
async fn execute(config: &JsonValue, ctx: &ConnectorCallContext) { ... }
```

Required log fields on every record: `ts`, `level`,
`correlation_id`, `crate`, `msg`.

Promoted fields when present: `tenant_id`, `instance_id`,
`step_seq`, `config_uuid`, `impl`, `realm`, `duration_ms`.

Metrics via the `metrics` crate, Prometheus-exposed via
`metrics-exporter-prometheus`. Metric names:
`philharmonic_<component>_<thing>_<unit>`, e.g.
`philharmonic_api_requests_total`,
`philharmonic_workflow_step_duration_seconds`,
`philharmonic_connector_payload_encrypt_duration_seconds`.

**Never use `tenant_id` as a metric label** (cardinality
explodes). Per-tenant observability goes through logs.

Correlation ID: header `X-Correlation-ID`, UUID v4, generated at
API ingress if absent, forwarded through all hops.

### Error envelope (from `11-security-and-cryptography.md`)

Tenant-facing API errors:

```json
{
  "error": {
    "code": "resource_not_found",
    "message": "Workflow template does not exist.",
    "details": { "resource_type": "workflow_template", "id": "..." },
    "correlation_id": "..."
  }
}
```

Codes are lowercase snake_case. HTTP status → code families
specified in doc 11.

### Permission atoms (from `09-policy-and-tenancy.md`)

The v1 vocabulary is closed. Available atoms:

- `workflow:template_create`, `workflow:template_read`,
  `workflow:template_retire`
- `workflow:instance_create`, `workflow:instance_read`,
  `workflow:instance_execute`, `workflow:instance_cancel`
- `endpoint:create`, `endpoint:rotate`, `endpoint:retire`,
  `endpoint:read_metadata`, `endpoint:read_decrypted`
- `tenant:principal_manage`, `tenant:role_manage`,
  `tenant:minting_manage`
- `mint:ephemeral_token`
- `tenant:settings_read`, `tenant:settings_manage`
- `audit:read`
- `deployment:tenant_manage`, `deployment:realm_manage`,
  `deployment:audit_read` (operator-only)

Do not add atoms without checking the existing list first and
flagging for review.

### API token format (from `09-policy-and-tenancy.md`)

```
pht_<43-char base64url-encoded 32 random bytes, no padding>
```

47 chars total. `pht_` prefix enables grep-based leak detection.
Hash the full token (including prefix) with SHA-256 for storage.

### Canonical JSON and content addressing

Content-addressed blobs use SHA-256 of RFC 8785 (JCS) canonical
JSON. The `philharmonic-types` cornerstone provides the
`CanonicalJson` type; use it, don't reinvent canonicalization.

### Statelessness

Workers and connector services hold no cross-request state
beyond operational state (connection pools, loaded keys,
static registries built at startup). No credential caches. No
per-tenant in-memory state. Every piece of authenticated context
arrives on the request or comes from a substrate lookup.

---

## 4. Phase plan

### Phase 0 — Workspace setup

**Status**: **Done**, with substantial added infrastructure
beyond the original plan.

**Originally-planned tasks** (all complete):
- `philharmonic-workspace` repo initialized with submodules for
  each of the 25 crates.
- Workspace `Cargo.toml` with all members listed and
  `[workspace.dependencies]` populated.
- Helper scripts in `scripts/` executable.
- Each submodule has an initial commit (README + `.gitignore` +
  placeholder `Cargo.toml` + `src/lib.rs`).
- Recommended Git config (`push.recurseSubmodules=check`
  auto-configured by `./scripts/setup.sh`; the three
  non-safety-critical ones documented in README).

**Infrastructure added beyond the plan** (see §9 for the full
file inventory):

- `scripts/` expanded from basic helpers to a full workflow:
  `setup.sh`, `status.sh`, `pull-all.sh`, `commit-all.sh` (with
  enforced `-s` signoff and `-S` GPG/SSH signing; rollback on
  unsigned commits), `push-all.sh` (`--follow-tags`), `heads.sh`,
  `check-detached.sh`, `show-dirty.sh`, `test-scripts.sh` (POSIX
  parse check), `rust-lint.sh`, `rust-test.sh`, `miri-test.sh`
  (routine `cargo +nightly miri test` for UB checks,
  `<crate> [<test>...]` shape), `pre-landing.sh` (mandated
  pre-landing driver), `check-toolchain.sh`,
  `check-api-breakage.sh`, `publish-crate.sh`, `verify-tag.sh`
  (three-way post-release check: local + signed + pushed to
  origin at same SHA), `cargo-audit.sh`, `crate-version.sh`,
  `codex-status.sh`, `print-audit-info.sh` (audit-trailer
  generator invoked once by `commit-all.sh`),
  `mktemp.sh` / `web-fetch.sh` (portability wrappers — never
  call raw `mktemp` / `curl` / `wget` from workspace scripts),
  `xtask.sh` (invocation wrapper for the in-tree `xtask/` crate;
  see §"In-tree workspace tooling" below), plus
  `scripts/lib/workspace-cd.sh` and `scripts/lib/workspace-members.sh`
  shared helpers. All POSIX sh, validated by `test-scripts.sh`
  (and CI).
- `xtask/` — in-tree (non-submodule) member crate holding
  workspace dev tooling written in Rust. Multi-bin layout
  (`src/bin/*.rs`, `publish = false`). Current bins: `gen-uuid`
  (canonical source for stable wire-format UUIDs),
  `crates-io-versions` (sparse-index query, `ureq` +
  `serde_json` — replaces the former `crates-io-versions.sh`
  shell script that depended on `jq` + `web-fetch.sh`), and
  `web-fetch` (`ureq` + `rustls`, replaces the old shell
  curl/wget/fetch/ftp fallback chain). Invoked via
  `./scripts/xtask.sh <tool> -- <args>`. See
  [`CONTRIBUTING.md §8`](CONTRIBUTING.md#8-in-tree-workspace-tooling-xtask).
- `.github/workflows/ci.yml` at the parent level — runs
  `setup.sh` + `pre-landing.sh` on push/PR (workspace-level; the
  `--ignored` phase runs contributor-side only).
- Journal infrastructure: `docs/codex-prompts/` (Codex prompt
  archive, committed before every Codex invocation),
  `docs/notes-to-humans/` (significant findings from Claude
  preserved to Git, not chat scrollback), common
  `YYYY-MM-DD-NNNN-<slug>[-NN].md` filename format.
- Agent instructions: `CLAUDE.md` (Claude's bootstrap),
  `AGENTS.md` (Codex's bootstrap), `docs/instructions/`
  (human-to-agent rules), `.codex/config.toml` (project-local
  Codex CLI config, activated via `CODEX_HOME=.codex`).
- `HUMANS.md` at the workspace root for Yuka's own notes
  (agent-read, agent-immutable by rule; committed by
  `commit-all.sh` via `git add -A` when dirty).
- `.editorconfig` and `.gitattributes` at the workspace root
  and copied into every submodule — aligned with rustfmt
  defaults (LF, 4-space, 100-col) and text/binary normalization.
- Pre-landing conventions: `cargo fmt --all --check`, `cargo
  clippy --workspace --all-targets -- -D warnings`, `cargo test
  --workspace` mandated before every Rust-touching commit; plus
  per-touched-crate `--ignored` phase to exercise integration
  tests only for modified crates.
- Publishing conventions: per-release signed annotated tag
  `v<version>` inside the crate's submodule (created only on
  successful `cargo publish`, pushed by the next `push-all.sh`
  via `--follow-tags`).
  `./scripts/check-api-breakage.sh <crate> [<baseline-version>]`
  against the previous crates.io release before any new release.

**Acceptance criteria**:
- `git submodule status` shows all 26 submodules at a clean
  commit. ✓
- `cargo check --workspace` succeeds. ✓
- `./scripts/status.sh` runs without error. ✓
- `./scripts/pre-landing.sh` passes at the workspace root. ✓
- `./scripts/test-scripts.sh` passes against every `scripts/*.sh`
  and `scripts/lib/*.sh`. ✓
- GitHub Actions CI green on `main`.

---

### Phase 1 — Extract `mechanics-config`

**Status**: **done (2026-04-21).** All three crates published
to crates.io with signed `v<version>` tags in their submodules:
`mechanics-config 0.1.0`, `mechanics-core 0.3.0`,
`mechanics 0.3.0`. The mechanics-core bump from originally-
drafted 0.2.3 to 0.3.0 was made after `check-api-breakage.sh`
surfaced a type-identity change that cargo-semver-checks
correctly flagged as a breaking change under cargo's pre-1.0
rules — see
`docs/notes-to-humans/2026-04-21-0006-mechanics-core-semver-checks-finding.md`.
`mechanics` was published in the same session, co-moved to 0.3.0
so downstream consumers opt in explicitly.

Landed work (see `docs/codex-prompts/2026-04-20-0001-phase-1-*`
for the Codex prompts and the commits they produced):

- `mechanics-config 0.1.0` created with the Boa-free schema
  types and pure structural validation. `cargo tree -p
  mechanics-config | grep -iE 'boa|reqwest|tokio'` is empty.
- `mechanics-core 0.3.0` depends on `mechanics-config = "0.1.0"`,
  wraps the extracted types with Boa GC newtypes
  (`#[unsafe_ignore_trace]`), re-exports at
  `mechanics_core::endpoint::*` and
  `mechanics_core::job::MechanicsConfig` so call-site paths keep
  working. The minor-digit bump (0.2.x → 0.3.0) signals the
  underlying type-identity change to cargo's resolver.
- Behavior change: schema validation now fails at config-
  construction time instead of job-call time (noted in
  `mechanics-core/CHANGELOG.md 0.3.0`).
- `mechanics` bumped `0.2.1 → 0.3.0` in lockstep: its own
  version and its `mechanics-core` dep pin both moved to the 0.3
  line, so downstream consumers opt into the new `mechanics-core`
  type identity explicitly rather than crossing it silently under
  a caret-range upgrade of either crate.
- Post-extraction cleanup: 5 orphan `.rs` files removed from
  `mechanics-core/src/internal/http/` that were no longer
  compiled but would still have shipped in the crates.io
  tarball (see
  `docs/notes-to-humans/2026-04-21-0002-mechanics-core-pre-publish-review.md`).

**Goal** (recorded for context): Split `mechanics-core` schema
types into a Boa-free `mechanics-config` crate so downstream
consumers (notably the lowerer in
`philharmonic-connector-client`) don't transitively depend on
Boa.

**Reference**: `06-execution-substrate.md`, section "Schema
extraction (settled)".

**Remaining work**: none. Phase 1 complete.

**Acceptance criteria**:
- `mechanics-config 0.1.0` published on crates.io. ✓ _(2026-04-21)_
- `mechanics-core 0.3.0` published on crates.io. ✓ _(2026-04-21)_
- `mechanics 0.3.0` published on crates.io. ✓ _(2026-04-21)_
- Signed `v0.1.0` / `v0.3.0` / `v0.3.0` release tags inside the
  respective submodule repos, pushed via `./scripts/push-all.sh`
  with `--follow-tags`. ✓
- `cargo tree -p mechanics-config | grep -iE 'boa|reqwest|tokio'`
  returns empty. ✓
- Workspace `cargo test --workspace` passing
  (`./scripts/rust-test.sh`). ✓
- `./scripts/check-api-breakage.sh mechanics-core` clean at the
  major bump (`v0.2.2 → v0.3.0 (major change)`,
  `no semver update required`). ✓

---

### Phase 2 — `philharmonic-policy`

**Status**: **done (2026-04-22).** Published as
`philharmonic-policy 0.1.0` on crates.io with signed `v0.1.0`
release tag in the submodule.

Landed across three dispatches plus a review pass:

- **Wave 1 — non-crypto foundation** (submodule commit
  `790c23d`, parent bump `65fc3c4`, 2026-04-21). Six entity
  kinds with stable `KIND: Uuid` constants (all of §"Tasks"
  below minus crypto), permission evaluation walking
  `RoleMembership` → `RoleDefinition` → permission atoms,
  three test tiers. Full landing summary in
  `docs/notes-to-humans/2026-04-21-0008-phase-2-wave-1-landed.md`.
- **Wave 2 — crypto foundation** (submodule commit
  `0085819`, parent bump `2c98467`, 2026-04-22). SCK
  AES-256-GCM primitives with wire format v1 and AAD binding
  over `tenant_id || config_uuid || key_version`, `pht_` API
  token format with Zeroize discipline. Gate-1 approval at
  `docs/crypto/approvals/2026-04-21-phase-2-sck-and-pht.md`
  (amended 2026-04-22 to swap `rand_core + getrandom` for
  `rand 0.10`); Gate-2 approval at
  `docs/crypto/approvals/2026-04-21-phase-2-sck-and-pht-01.md`.
  Detail in
  `docs/notes-to-humans/2026-04-22-0001-phase-2-wave-2-landed-pending-gate2.md`.
- **Auth-boundary hardening** (submodule commit `1cde0e1`,
  parent `89dd590`, 2026-04-22) — closes two findings
  surfaced by an independent Codex security review at
  `docs/codex-reports/2026-04-22-0001-philharmonic-policy-security-review.md`.
  Finding #1: cross-tenant role confusion in
  `evaluate_permission` (role's tenant wasn't checked). Fix:
  three-way tenant binding (principal / membership / role).
  Finding #2: `PermissionDocument` accepted arbitrary
  strings. Fix: parse-time validation against the canonical
  22-atom `ALL_ATOMS` list.
- **Gate-2 re-review + zeroization hardening** (submodule
  commit `7ca357e`, parent `59f860a`, 2026-04-22). Claude-
  driven Gate-2 review pass per Yuka's condition in the
  approval doc; two stack-copy zeroization gaps (H1 in
  `token.rs`, H2 in `sck.rs::from_file`) tightened via
  `Zeroizing<[u8; 32]>` wrapping at declaration and
  pass-by-reference. Detail in
  `docs/notes-to-humans/2026-04-22-0005-gate-2-claude-review-outcome.md`
  and `0006-crypto-zeroization-hardening-applied.md`.

Final test discipline (as shipped at `0.1.0`):

- 10 unit tests (`src/lib.rs`), 15 crypto-vector tests
  (`tests/crypto_vectors.rs`), 14 mock-backed tests
  (`tests/permission_mock.rs`), 12 MySQL-testcontainer tests
  (`tests/permission_mysql.rs`, `#[ignore]`-gated).
- Python reference generators (`tests/crypto_vectors/gen_sck.py`,
  `gen_pht.py`) committed for audit-reproducibility; the
  committed Rust hex constants match pyca `cryptography 41.0.7`
  byte-for-byte across all 3 SCK + 3 `pht_` vectors.
- Miri clean across the full non-ignored test suite (`cargo
  +nightly miri test -p philharmonic-policy`).

**Remaining work**: none. Phase 2 complete.

**Reference**: `09-policy-and-tenancy.md` exhaustively;
`05-storage-substrate.md` for storage traits.

**Crates touched**: `philharmonic-policy`.

**Tasks**:
1. Implement the seven entity kinds:
   - `Tenant`
   - `Principal` (with `epoch` scalar reserved, unused in v1)
   - `TenantEndpointConfig` (minimal shape: `display_name` and
     `encrypted_config` content slots, `key_version` and
     `is_retired` scalars, `tenant` entity slot)
   - `RoleDefinition`
   - `RoleMembership`
   - `MintingAuthority` (with `epoch` scalar)
   - `AuditEvent`

   Each entity kind gets a stable `KIND: Uuid` constant. Generate
   these UUIDs once and commit them; they're part of the wire
   format for substrate storage.

2. Implement SCK-based encryption for `TenantEndpointConfig`:
   - AES-256-GCM encrypt on submit, decrypt on read.
   - SCK loaded from deployment secret storage (for v1, a file
     path configurable via environment variable is fine).
   - **FLAG for Yuka's review** — this is crypto-path code.
   - Include test vectors: fixed SCK bytes + fixed plaintext →
     exact ciphertext match.

3. Implement the `pht_` API token format utilities:
   - `generate_api_token() -> (String, Sha256Hash)` — returns the
     plaintext token and its storage hash.
   - `parse_api_token(s: &str) -> Result<...>` — validates format
     and extracts the hashable form.
   - Constants: `TOKEN_PREFIX = "pht_"`, `TOKEN_BYTES = 32`,
     `TOKEN_ENCODED_LEN = 43`.

4. Implement permission evaluation:
   - `evaluate_permission(principal, tenant, required_atom) -> bool`
     that walks `RoleMembership` → `RoleDefinition` → permission
     array membership.

5. Unit tests covering:
   - Entity CRUD via substrate (use the sqlx-mysql backend in
     dev-dependency via `testcontainers` for integration).
   - SCK round-trip with known test vectors.
   - API token generate/parse round-trip.
   - Permission evaluation with nested role memberships.

**Acceptance criteria**:
- All seven entity kinds compile and pass their validation. ✓
- SCK crypto tests include at least three test vectors (keys,
  plaintexts, expected ciphertexts) that are hand-verifiable. ✓
  _(3 vectors cross-checked byte-for-byte against pyca
  `cryptography 41.0.7`; Python generators committed.)_
- Permission evaluation tests cover: permission granted,
  permission denied, retired role, retired membership, retired
  principal, suspended tenant. ✓ _(Tenant-status check at eval
  time is deferred — flagged in the Gate-2 review note 0005 as
  Wave 1 code intended to be enforced at the API layer in
  Phase 8; not a Phase 2 regression. All other coverage
  present in mock + MySQL tiers.)_
- `philharmonic-policy` publishes as `0.1.0`. ✓ _(2026-04-22)_

---

### Phase 3 — `philharmonic-connector-common` ✓ _(done 2026-04-22)_

**Status**: Landed in a single Codex dispatch (see
`docs/codex-prompts/2026-04-22-0002-phase-3-connector-common.md`).
Pre-landing + miri both green (10 tests). Gate-2 review
completed by Yuka in-session; dedicated approval doc deferred
because Phase 3 carries no crypto construction (types-only).
Published as `philharmonic-connector-common 0.1.0` on
crates.io; tag `v0.1.0` signed and pushed.

**Goal**: Shared wire-format types for the connector layer.

**Reference**: `08-connector-architecture.md`,
`11-security-and-cryptography.md`.

**Crates touched**: `philharmonic-connector-common`.

**Tasks**:
1. Define token claim types for the connector authorization
   token:
   ```rust
   struct ConnectorTokenClaims {
       iss: String,
       exp: u64,
       kid: String,
       realm: String,
       tenant: Uuid,
       inst: Uuid,
       step: u64,
       config_uuid: Uuid,
       payload_hash: [u8; 32],
   }
   ```

2. Define `ConnectorCallContext` as the verified-claim bundle
   passed to `Implementation::execute`:
   ```rust
   pub struct ConnectorCallContext {
       pub tenant_id: Uuid,
       pub instance_id: Uuid,
       pub step_seq: u64,
       pub config_uuid: Uuid,
       pub issued_at: u64,
       pub expires_at: u64,
   }
   ```

   Plain `Uuid` (not `EntityId<T>`) to keep this crate free of
   dependencies on `philharmonic-policy`.

3. Define the realm model:
   ```rust
   pub struct RealmId(String);          // newtype wrapper
   pub struct RealmPublicKey { ... }    // ML-KEM-768 + X25519
   pub struct RealmRegistry {           // kid-indexed map
       by_kid: HashMap<String, RealmPublicKey>,
   }
   ```

4. Define the COSE_Sign1 and COSE_Encrypt0 wrapper types using
   the `coset` crate. These are thin type-safe wrappers; the
   actual signing/verification lives in
   `philharmonic-connector-client` and
   `philharmonic-connector-service`.

5. Define `ImplementationError` with variants covering:
   - `InvalidConfig` (impl couldn't deserialize its config)
   - `UpstreamError` (4xx/5xx from external service)
   - `UpstreamUnreachable` (network failure)
   - `UpstreamTimeout`
   - `SchemaValidationFailed` (for LLM implementations)
   - `ResponseTooLarge`
   - `InvalidRequest` (malformed request from script)
   - `Internal` (catch-all with string detail)

**Acceptance criteria**:
- `philharmonic-connector-common` compiles with minimal deps
  (`philharmonic-types`, `coset`, `serde`, `thiserror`). ✓
  _(`mechanics-config` ended up unused in Phase 3 scope; may
  return in Phase 5.)_
- Unit tests for token claim serde round-trip and realm
  registry lookup by kid. ✓ _(10 tests total.)_
- `philharmonic-connector-common` publishes as `0.1.0`. ✓
  _(2026-04-22)_

---

### Phase 4 — `philharmonic-workflow` ✓ _(done 2026-04-22)_

**Status**: Landed via one Codex dispatch (see
`docs/codex-prompts/2026-04-22-0003-phase-4-workflow.md` and
Codex's notes at
`docs/codex-reports/2026-04-22-0002-phase-4-workflow.md`).
Claude's review is at
`docs/notes-to-humans/2026-04-22-0008-phase-4-claude-review.md`.
Pre-landing + `cargo +nightly miri test` + tier-2 MySQL
testcontainers all green. Gate-2 review completed in-session
(not a crypto-construction trigger). One design-doc fix applied
alongside: `07-workflow-orchestration.md §Status transitions`
now lists `Pending → Failed` explicitly (the engine writes one
instance revision per step, so a first-step failure transitions
directly from Pending without an intermediate Running
revision). Published as `philharmonic-workflow 0.1.0`; tag
`v0.1.0` signed and pushed.

**Goal**: Implement the workflow orchestration engine.

**Reference**: `07-workflow-orchestration.md` exhaustively.

**Crates touched**: `philharmonic-workflow`.

**Tasks**:
1. Implement the three entity kinds: `WorkflowTemplate` (with
   `script` and `config` content slots; `config` is a
   `{script_name: config_uuid}` map), `WorkflowInstance`,
   `StepRecord`.

2. Implement `SubjectContext`:
   ```rust
   pub struct SubjectContext {
       pub kind: SubjectKind,
       pub id: String,
       pub tenant_id: EntityId<Tenant>,
       pub authority_id: Option<EntityId<MintingAuthority>>,
       pub claims: JsonValue,
   }
   pub enum SubjectKind { Principal, Ephemeral }
   ```

3. Implement `StepExecutor` and `ConfigLowerer` traits exactly
   as specified in doc 07.

4. Implement `WorkflowEngine` with `create_instance`,
   `execute_step`, `complete`, `cancel`. Each takes
   `SubjectContext`; each emits appropriate `StepRecord` on
   success or failure.

5. Implement status transitions per the state machine in doc 07.
   Enforce terminal-state immutability
   (`Completed`/`Failed`/`Cancelled` reject further operations
   with `InstanceTerminal` error).

6. Implement the execution sequence (9 steps per doc 07
   "Execution sequence"). Critical details:
   - Step records are created *before* instance revisions so the
     step record can pin to the pre-step instance revision.
   - Malformed result (missing `context` or `output`) is treated
     as script error, not transport failure.
   - `done: true` in script return transitions instance to
     `Completed`.

7. Step record subject content records **identifier and
   authority only**, never full injected claims. No
   configurability.

8. Unit tests:
   - Status transitions exhaustive.
   - Execution sequence with mock `StepExecutor` and
     `ConfigLowerer`.
   - Subject context propagation from engine to step record.

**Acceptance criteria**:
- All engine methods work with `testcontainers`-based substrate
  integration tests. ✓ _(3 MySQL testcontainer tests pass.)_
- Full status-transition state machine coverage. ✓
  _(Exhaustive matrix test in `src/status.rs`; 8 tier-1
  behavioral tests in `tests/engine_mock.rs` covering the
  execution sequence, terminal-state immutability, transport vs.
  script errors, malformed-result handling, subject propagation,
  and the audit-discipline invariant.)_
- `philharmonic-workflow` publishes as `0.1.0`. ✓
  _(2026-04-22)_

---

### Phase 5 — Connector triangle (CRYPTO — Yuka reviews)

**Goal**: Implement the three connector crates that together
carry the crypto-sensitive wire protocol. **Yuka reviews this
code personally.** Do not publish these without her sign-off.

**Reference**: `08-connector-architecture.md`,
`11-security-and-cryptography.md` exhaustively.

#### Wave split

Phase 5 is split into two waves, each with its own Gate-1
crypto proposal and Gate-2 code review. Decided 2026-04-22 to
keep each review surface tractable — Phase 2 used the same
wave pattern and it kept round-trips manageable.

- **Wave A — COSE_Sign1 authorization tokens.** Ed25519
  signing, COSE_Sign1 construction and verification, kid-based
  public-key registry, token expiry + payload-hash claim
  binding. No encryption. Touches `connector-client` (mint)
  and `connector-service` (verify). `connector-router` stays
  no-crypto. Smaller, well-understood primitives (RustCrypto
  `ed25519-dalek` + `coset`). Gate-1 proposal at
  `docs/crypto/proposals/2026-04-22-phase-5-wave-a-cose-sign1-tokens.md`.

- **Wave B — Hybrid KEM + AEAD payload encryption.** ML-KEM-768
  + X25519 hybrid KEM, HKDF-SHA256 key derivation, AES-256-GCM
  with AAD binding via the RFC 9052 `Enc_structure`. COSE_Encrypt0
  construction and verification. Realm public-key distribution.
  Zeroization discipline for intermediate key material. Larger
  surface; more novel (PQC). Builds on Wave A's token so that
  encrypted payloads can be hash-bound at mint time.
  **Gate-1 approved 2026-04-22** (proposal r3 at
  `docs/crypto/proposals/2026-04-22-phase-5-wave-b-hybrid-kem-cose-encrypt0.md`;
  approval at
  `docs/crypto/approvals/2026-04-22-phase-5-wave-b-hybrid-kem-cose-encrypt0.md`;
  Codex security review at
  `docs/codex-reports/2026-04-22-0005-phase-5-wave-b-hybrid-kem-cose-encrypt0-security-review.md`).
  Reference vectors live at `philharmonic-connector-service/tests/vectors/wave-b/`
  (23 hex files + JSON plaintext + README). **Landed 2026-04-23**
  across two Codex rounds — main implementation
  (`docs/codex-prompts/2026-04-22-0005-phase-5-wave-b-hybrid-kem-cose-encrypt0.md`)
  + zeroization/dead-code follow-up
  (`docs/codex-prompts/2026-04-23-0001-phase-5-wave-b-zeroization-followup.md`).
  **Gate-2 approved 2026-04-23**
  (`docs/crypto/approvals/2026-04-23-0001-phase-5-wave-b-codex-dispatch-complete.md`).
  Wave B also lands the `philharmonic-connector-common 0.2.0`
  bump (adds `iat` claim per Wave A Gate-2 decision (A) later)
  — forced a Wave A reference-vector regeneration and added a
  composition-vector family
  (`philharmonic-connector-client/tests/vectors/wave-a/wave_a_composition_*.hex`) that
  points at Wave B's `payload_hash`.

Neither wave publishes on its own — the triangle's crates
publish as `0.1.0` only after Wave B's end-to-end tests pass.
Waves are internal review milestones; crates-io consumers see
only the completed surface. **Published 2026-04-23**:
`philharmonic-connector-common 0.2.0`,
`philharmonic-connector-client 0.1.0`,
`philharmonic-connector-service 0.1.0`,
`philharmonic-connector-router 0.1.0` — all four with signed
release tags (`verify-tag.sh`: local + signed + pushed, ok).

**Crates touched**: `philharmonic-connector-client`,
`philharmonic-connector-router`, `philharmonic-connector-service`.

---

#### Wave A — COSE_Sign1 authorization tokens

**Scope**: signing and verification only. Tokens authenticate
the lowerer and bind to a `payload_hash` claim; Wave A tests
with arbitrary payload bytes so the token format lands
independently of the KEM work.

**Tasks**:

1. **Lowerer-side mint (in `philharmonic-connector-client`)**:
   - Ed25519 key pair lifecycle — key loading from the
     lowerer's configured private-key source, kid tagging,
     `Zeroizing` wrapper for the private bytes, signing.
   - COSE_Sign1 construction over a CBOR-encoded
     `ConnectorTokenClaims` payload (type already shipping in
     `philharmonic-connector-common 0.1.0`). Protected header
     declares the algorithm (`EdDSA` / COSE alg -8) and the
     `kid`.
   - `mint_token(claims) -> ConnectorSignedToken` public API.

2. **Service-side verify (in `philharmonic-connector-service`)**:
   - Kid-indexed `MintingKeyRegistry` of Ed25519 public keys,
     registered at service boot from config.
   - COSE_Sign1 signature verification.
   - Claim checks: `exp` in the future (monotonic wall-clock
     `UnixMillis`); `payload_hash` equals the SHA-256 of the
     caller-supplied encrypted-payload bytes (the bytes are
     opaque in Wave A — any SHA-256 agreement passes);
     optional `realm` equality against the service's own realm.
   - `verify_token(token, payload_bytes, expected_realm) ->
      Result<ConnectorCallContext, TokenVerifyError>` public
     API.
   - Typed error enum distinguishing: signature mismatch,
     expired, unknown kid, payload-hash mismatch, realm
     mismatch, malformed COSE.

3. **Crypto test vectors (known-answer tests)**:
   - Fixed Ed25519 keypair (seed committed in `tests/vectors/`).
   - Fixed claim set with every field populated.
   - Fixed CBOR encoding of the claim payload (byte-identical).
   - Fixed COSE_Sign1 output bytes (byte-identical).
   - Tampered-signature rejection (flip one signature bit).
   - Expired-token rejection.
   - Wrong-kid rejection.
   - Payload-hash-mismatch rejection.

4. **Router: no-op.** `philharmonic-connector-router` stays
   unimplemented in Wave A; no crypto needed for a pure
   HTTP dispatcher.

**Acceptance criteria (Wave A)**:
- Gate-1 proposal approved by Yuka.
- Known-answer test vectors pass byte-for-byte.
- Round-trip test: mint → verify succeeds end to end.
- Tamper / expiry / kid / hash rejection paths each have a
  dedicated test.
- No panics in `src/`; Ed25519 private bytes wrapped in
  `Zeroizing`; no `unsafe`.
- Gate-2 code review by Yuka.
- **No publish at Wave A end.** The crates stay at `0.0.0`
  until Wave B finishes.

**Wave A status — landed 2026-04-22, Gate-2 approved
2026-04-22.**
- Gate-1: approved (proposal r4,
  `docs/crypto/approvals/2026-04-22-phase-5-wave-a-cose-sign1-tokens.md`).
  Triggered a `philharmonic-types 0.3.5` upstream fix
  (CBOR-bstr `Sha256` serde) and a new workspace rule at
  `docs/design/13-conventions.md` §Library crate boundaries.
- Reference vectors: generated with cbor2 + cryptography at
  `philharmonic-connector-client/tests/vectors/wave-a/` before Codex dispatch; matched
  byte-for-byte by the Rust implementation.
- Implementation: Codex, commits `9634f68` + `bcf8ea6` + parent
  pointer `ac02232`. Codex prompt archive:
  `docs/codex-prompts/2026-04-22-0004-phase-5-wave-a-cose-sign1-tokens.md`.
- Verification: positive KAT + 10 negative vectors green;
  `pre-landing.sh` + `miri-test.sh` pass on both crates; no
  panics, no `unsafe`, no file I/O in library code.
- Claude Gate-2 review: PASS —
  `docs/notes-to-humans/2026-04-22-0011-phase-5-wave-a-claude-review.md`.
  One flagged follow-up (`ConnectorCallContext.issued_at` has
  no source claim; Codex set it to `now` at verify time —
  semantically "time verified" not "time issued").
- Yuka Gate-2 approval:
  `docs/crypto/approvals/2026-04-22-phase-5-wave-a-cose-sign1-tokens-01.md`.
  `issued_at` follow-up decided **(C) for now, (A) later** —
  accept `= now` through Wave A; add an `iat` claim to
  `ConnectorTokenClaims` with the `philharmonic-connector-common
  0.2.0` bump that Wave B forces.
- Codex accuracy-checked the Claude Gate-2 note —
  `docs/codex-reports/2026-04-22-0004-phase-5-wave-a-claude-review-accuracy-check.md`.
  Two precision nits recorded under the note's §Corrigenda;
  neither changes the Gate-2 conclusion.
- **Crates remain at `0.0.0`; publish waits for Wave B's
  end-to-end tests.** Gate-2 approval is for "code as landed in
  working tree," not for crates-io release.

---

#### Wave B — Hybrid KEM + AEAD payload encryption

**Scope**: the encryption half of the triangle, composed with
Wave A's token so that `payload_hash` binds real ciphertext.

**Tasks**:

1. **Lowerer-side encrypt (in `philharmonic-connector-client`)**:
   - Complete the `ConfigLowerer` pipeline stubbed in Wave A:
     a. For each entry in the template's abstract config, fetch
        the `TenantEndpointConfig` by UUID from
        `philharmonic-policy`.
     b. Verify the config's tenant matches the instance's tenant.
     c. Verify `is_retired == false`.
     d. Decrypt the config's `encrypted_config` with SCK.
     e. Parse JSON, read only the `realm` field.
     f. Re-encrypt the decrypted bytes — **byte-identical** — to
        the realm's KEM public key via COSE_Encrypt0 with the
        ML-KEM-768 + X25519 hybrid construction.
     g. Hash the encrypted payload (SHA-256).
     h. Mint the Wave-A COSE_Sign1 token over claims
        `{iss, exp, kid, realm, tenant, inst, step, config_uuid,
        payload_hash}`.
     i. Assemble the `MechanicsConfig` entry: POST to the
        deployment-supplied connector URL for the realm
        (e.g. `<realm>.connector.<deployment-domain>` if the
        deployment uses subdomain-per-realm; the framework
        treats the URL as deployment-supplied configuration),
        `Authorization: Bearer <COSE_Sign1 bytes, base64url>`,
        `X-Encrypted-Payload: <COSE_Encrypt0 bytes, base64url>`.

2. **Service-side decrypt (in `philharmonic-connector-service`)**:
   - Build on Wave A's token-verify: after the token is valid,
     decrypt COSE_Encrypt0 with the realm private key selected
     by `kid`.
   - Parse decrypted JSON; verify inner `realm` field matches
     token `realm` (belt-and-suspenders).
   - Look up `impl` field in the implementation registry;
     reject if unknown.
   - Dispatch: call `handler.execute(config_subobject, request,
     ctx)` where `ctx` is built from the verified token claims.
   - Wrap result in HTTP response; map `ImplementationError`
     variants to appropriate HTTP status codes.

3. **`philharmonic-connector-router` (pure dispatcher)**:
   - Minimal HTTP server that fronts the connector services
     for one realm, configured by the deployment to listen on
     whichever URL the deployment chose (subdomain-per-realm,
     path-prefix-per-realm, host-header dispatch, etc. — the
     router doesn't prescribe a shape). Forwards to connector
     service instances in the realm.
   - No token verification, no decryption, no rate limiting.
   - Round-robin or least-connections load balancing is fine;
     pick whatever's simple.

4. **Crypto test vectors (known-answer tests)**:
   - Fixed ML-KEM-768 keypair, fixed X25519 keypair (seeds
     committed).
   - Fixed plaintext payload, fixed KEM randomness.
   - Expected COSE_Encrypt0 bytes (byte-identical).
   - Combined with Wave A's Ed25519 keypair: expected
     COSE_Sign1 over `payload_hash = SHA-256(COSE_Encrypt0 bytes)`.

   Known-answer tests catch nonce-reuse, HKDF-input-ordering,
   and AEAD-additional-data bugs that round-trip-only tests
   miss. Required to pass byte-for-byte.

**Acceptance criteria (Wave B)**:
- Gate-1 proposal approved by Yuka (separate doc from Wave A).
- Known-answer test vectors pass byte-for-byte for both the
  hybrid KEM and COSE_Encrypt0 layers.
- End-to-end integration test: `connector-client` encrypts +
  signs, `connector-service` verifies + decrypts, plaintext
  matches exactly.
- Zeroization discipline verified: HKDF output, shared secret,
  AEAD key all `Zeroizing`-wrapped; audit by grepping for
  the wrapper on every key-material variable.
- Yuka has reviewed the crypto code paths (Gate-2) and signed
  off.
- No `unsafe` blocks in crypto code.
- No custom crypto primitives — only `ml-kem`, `x25519-dalek`,
  `aes-gcm`, `ed25519-dalek`, `hkdf`, `sha2` from RustCrypto.
- All three triangle crates publish as `0.1.0`.

---

### Phase 6 — First implementations

**Goal**: Ship two connector implementations that together prove
the end-to-end path works: one generic HTTP implementation
(simplest) and one LLM implementation (unblocks the chat-app use
case).

**Reference**: `08-connector-architecture.md`, specifically the
full wire protocol specs under "v1 implementation set" and the
§"Crate organization" → `philharmonic-connector-impl-api`
subsection.

**Crates touched**:
`philharmonic-connector-impl-api` (new, prerequisite),
`philharmonic-connector-impl-http-forward`,
`philharmonic-connector-impl-llm-openai-compat`.

**Tasks**:

0. **`philharmonic-connector-impl-api` (prerequisite for 1 and 2)** ✓ _(done 2026-04-24, published as 0.1.0)_:
   - New non-crypto trait-only crate, created and published
     before any impl crate in this phase. Rationale: doc 08
     §"Implementation trait" locates the trait here rather than
     in the crypto-reviewed `connector-service` crate, so that
     non-breaking trait-surface changes don't re-trigger crypto
     Gate 1 / Gate 2 on `connector-service`, and impl crates
     don't pull the crypto dep surface into their tree.
   - Public surface: the `#[async_trait] Implementation` trait
     (full signature in doc 08) plus re-exports of
     `ConnectorCallContext` and `ImplementationError` from
     `connector-common`, and a re-export of `async_trait` from
     the `async-trait` crate.
   - Dependencies: `philharmonic-connector-common`,
     `async-trait = "0.1"`, `serde_json` (for
     `serde_json::Value`). No crypto deps, no HTTP stack, no
     tokio.
   - Async mechanism: `#[async_trait]` macro, deliberately
     chosen over native async-fn-in-traits for dyn-compat +
     `Send`-bound-inference reasons that still bite in 2026.
     See doc 08 §"Why `async_trait` (in 2026)" for the full
     rationale.
   - Publish as `0.1.0`. Workspace members list + publish-queue
     order updated accordingly.

1. **`http_forward`** ✓ _(done 2026-04-24, published as 0.1.0)_:
   - Depends on `philharmonic-connector-impl-api` and
     `mechanics-config`. Does **not** depend on
     `philharmonic-connector-service` directly.
   - Config shape reuses `mechanics_config::HttpEndpoint`. Do not
     reinvent; depend on `mechanics-config` and use its type.
     Load-time validation uses `HttpEndpoint::prepare_runtime`;
     cache the returned `PreparedHttpEndpoint` for reuse.
   - Request shape: `{url_params, query, headers, body}`,
     validated against the config's `HttpEndpoint`.
   - Response shape: `{status, headers, body}`. Headers filtered
     to the config's `exposed_response_headers`.
   - Error handling: upstream 4xx/5xx returns as a normal
     response (not an error); only network/timeout failures
     surface as `ImplementationError`.
   - Use `reqwest` with `rustls-tls` (per the workspace HTTP-
     client split — see CONTRIBUTING.md §10.9). tokio runtime;
     a single `reqwest::Client` reused across calls; per-
     request timeout from `HttpEndpoint.timeout_ms`. No
     `ureq`, no `native-tls`.
   - Integration tests against a `wiremock`-backed local mock
     (preferred for determinism in CI); `httpbin.org` smokes
     optional and gated on an env flag.

2. **`llm_openai_compat`** ✓ _(done 2026-04-24, published as 0.1.0; spec `docs/notes-to-humans/2026-04-24-0003-phase-6-task-2-llm-openai-compat-impl-spec.md`, review `docs/notes-to-humans/2026-04-24-0004-phase-6-llm-openai-compat-review.md`)_:
   - Depends on `philharmonic-connector-impl-api` (same
     no-crypto dep surface as `http_forward`).
   - Config shape: `{base_url, api_key, dialect, timeout_ms}`.
   - Dialect enum: `openai_native` | `vllm_native` |
     `tool_call_fallback`.
   - Request shape: OpenAI-like-minimal per doc 08 —
     `{model, messages, output_schema, max_output_tokens?,
     temperature?, top_p?, stop?}`.
   - Response shape: `{output, stop_reason, usage}`.
   - Per-dialect translation to the provider's wire format:
     - `openai_native`: native `response_format: json_schema`
       at the top level of the request body.
     - `vllm_native`: top-level `structured_outputs:
       {"json": <schema>}` field. Not `extra_body` — that's a
       Python-client idiom; we construct the HTTP body directly.
     - `tool_call_fallback`: synthetic tool whose input schema
       is `output_schema`, forced via `tool_choice`, extracted
       from tool-call arguments.
   - Normalize `stop_reason` values across dialects to the
     documented set (`end_turn`, `max_tokens`, `stop_sequence`,
     `content_filter`, `error`).
   - Normalize usage accounting (`input_tokens`,
     `output_tokens`).
   - Validate output against `output_schema` using `jsonschema`
     or similar; if validation fails, return
     `SchemaValidationFailed`.
   - Testing discipline — **deterministic tests first,
     external-dep tests `#[ignore]`-d by default**:
     - Primary: `wiremock`-backed integration tests with
       fixtures for each dialect's request + response shape
       (same pattern as http_forward's `request_vectors.rs`).
       CI-safe, deterministic, dialect-translation coverage
       is the contract.
     - Fixture provenance — extracted from upstream to our
       tree as pure JSON (no Python); commit under
       [`docs/upstream-fixtures/vllm/`](docs/upstream-fixtures/vllm/)
       with pinned SHAs + license attribution in the
       directory's `README.md`:
       - `vllm_native` — extracted from vLLM's own test
         suite (Apache-2.0) at commit
         `cf8a613a87264183058801309868722f9013e101`:
         - `sample_json_schema.json` (from
           `tests/conftest.py:sample_json_schema`) — the
           employee-profile schema.
         - `structured_outputs_json_chat_request.json`
           (from `tests/entrypoints/openai/chat_completion/test_chat.py::test_structured_outputs_json_chat`)
           — the on-the-wire HTTP request body showing
           `structured_outputs: {"json": <schema>}` at the
           top level (confirming doc 08's shape directly
           against upstream, since the Python client's
           `extra_body=dict(structured_outputs=...)` merges
           into the top of the HTTP body).
       - `openai_native` + `tool_call_fallback`: request +
         response shapes follow the OpenAI chat-completion
         public API spec (`response_format: {type: "json_schema"
         ...}`, `tools + tool_choice` for the fallback).
         Fixtures synthesized against the documented envelope
         (`choices[0].message`, `usage`, `finish_reason`).
     - Response-body fixtures for the wiremock path are
       synthesized to match the documented envelope, not
       captured from a live run — upstream only commits
       request patterns; actual response bytes only exist
       when their tests run. Envelope shape itself is stable
       enough (OpenAI chat-completion public API) that
       synthesis is safe; the drift-catching role falls to
       the smokes below.
     - Optional smoke against real OpenAI API: `#[ignore]`
       + an env gate such as `OPENAI_SMOKE_ENABLED=1
       OPENAI_API_KEY=...`. Manual only; not on any CI
       path; small cheap model.
     - Optional smoke against a CPU INT8 vLLM endpoint:
       `#[ignore]` + an env gate such as
       `VLLM_SMOKE_ENABLED=1 VLLM_BASE_URL=http://...`.
       vLLM-on-CPU is viable on the Xeon 8259CL box
       (AVX-512 + AVX512_VNNI accelerate INT8 dot products,
       48 physical cores); it is a non-starter on typical
       ultrabook dev hosts. Tests must not assume the box
       is available — `#[ignore]` keeps `cargo test` from
       failing when it isn't. This smoke is the drift check
       against committed fixtures: diff real-vLLM output
       against the synthesized-envelope fixtures; any
       non-empty diff is either upstream drift or our
       stale cache.

**Acceptance criteria**:
- [x] `philharmonic-connector-impl-api` published as `0.1.0`
  before either impl crate is published. ✓ _(2026-04-24)_
- [x] Unit tests cover dialect translation with fixed expected
  provider-specific request bodies (test vectors for request
  shape too, not just round-trips). ✓ _(both impl crates;
  llm_openai_compat's dialect translation is anchored against
  the pinned vLLM upstream fixture + real-OpenAI captures
  under `docs/upstream-fixtures/`)_
- [x] Both impl crates publish as `0.1.0`. ✓ _(http_forward +
  llm_openai_compat, both 2026-04-24)_
- [ ] End-to-end flow works: create tenant endpoint config for
  `http_forward`, create workflow template referencing it,
  execute a step, see the HTTP call land at the target.
  _(deferred — requires `philharmonic-connector-service`
  realm binary + `philharmonic-api` layer; lands in Phase 8+.
  Not gated by Phase 6; impl crates are shipped so downstream
  consumers can wire them in as realm binaries come online.)_
- [ ] Same end-to-end flow for `llm_openai_compat` with OpenAI
  as the target. _(deferred with the above for the same
  reason.)_

---

### Phase 7 — Additional implementations (parallel-safe)

**Goal**: Ship the remaining implementations. They're
crate-level independent and don't block each other
technically, but we work them in the priority tiers below —
driven by product need, not by technical dependency.

**Reference**: `08-connector-architecture.md` for each
implementation's wire protocol.

**Priority ordering** (2026-04-24, captured from Yuka):

- **Tier 1 — data-layer connectors** ✓ _(done 2026-04-27)_:
  - `philharmonic-connector-impl-sql-postgres` — **0.1.0
    published 2026-04-27** (impl landed 2026-04-24 from
    Codex round 01 + Claude housekeeping fixes: Oid type,
    drop `timetz` arm, `postgresql://` scheme alias,
    `tests/common/mod.rs` layout, Docker-serialized
    integration tests).
  - `philharmonic-connector-impl-sql-mysql` — **0.1.0
    published 2026-04-27** (impl landed 2026-04-24 from
    Codex round 01, reviewer-approved; Docker-serialized
    integration tests).
  - `philharmonic-connector-impl-vector-search` — **0.1.0
    published 2026-04-27** (impl landed 2026-04-24 from
    Codex round 01, stateless in-memory cosine kNN per
    the spec; 34 tests passing, no external deps).
  - `philharmonic-connector-impl-embed` — **0.1.0
    published 2026-04-27** evening (wave 2). Round-01
    `fastembed` + `ort` was reverted as a library choice
    after the glibc-only `ort-download-binaries` link
    constraint was surfaced 2026-04-24 (deployment
    target includes musl); Yuka picked pure-Rust
    `tract` + `tokenizers` instead. Round 02
    rewrite landed via Codex, then needed three Claude
    follow-ups to ship: (1) `inline-blob` proc-macro
    crate (Yuka-authored same day, adopted as
    workspace submodule) to place bge-m3's 2.27 GB
    bundled bytes in `.lrodata.*` sections (`include_bytes!`
    of >2 GB in regular `.rodata` overflows rust-lld's
    32-bit PC-relative relocation range with small code
    model); (2) prefer
    `sentence_bert_config.json`'s canonical
    `max_seq_length` over `config.json`'s
    `max_position_embeddings` so XLM-RoBERTa-class models
    like bge-m3 don't trip tract's gather op via
    position-index-out-of-range; (3) switch tokenizer to
    `PaddingStrategy::BatchLongest` so a short query
    isn't padded to 8192 positions for inference.
    Default-bundled model is `BAAI/bge-m3`; the
    `bundled-default-model` Cargo feature
    (default-on) gates the build-time HuggingFace fetch,
    overridable via `PHILHARMONIC_EMBED_DEFAULT_MODEL`
    + `PHILHARMONIC_EMBED_DEFAULT_REVISION` env vars and
    by `--no-default-features` for offline / packaging
    builds. Round-03 prompt at
    [`docs/codex-prompts/2026-04-27-0001-phase-7-embed-tract-03.md`](docs/codex-prompts/2026-04-27-0001-phase-7-embed-tract-03.md);
    architecture-decision note at
    [`docs/notes-to-humans/2026-04-27-0002-phase-7-embed-default-bundled-model-architecture.md`](docs/notes-to-humans/2026-04-27-0002-phase-7-embed-default-bundled-model-architecture.md).

  Wave 1 (the three publish-ready crates) shipped on
  2026-04-27 morning rather than co-landing with the embed
  rewrite, per Yuka's call to publish what didn't need
  more attention. Wave 2 (`embed`) shipped 2026-04-27
  evening after the tract rewrite + inline-blob landed.
  Docker-backed integration tests in both SQL crates use
  `serial_test`'s `#[file_serial(docker)]` to prevent
  containers from piling up and OOMing the host.

  **Workspace-internal `inline-blob 0.1.0`** also
  shipped 2026-04-27 — Yuka-authored proc-macro that
  emits `static [u8; N]` items into `.lrodata.<name>`
  (SysV-ABI large-rodata) with an anchor in
  `.lbss.<name>`. Intended for any future ELF-target
  workspace member that needs to embed multi-GB blobs;
  consumed today only by `philharmonic-connector-impl-embed`.
  See [`inline-blob/README.md`](inline-blob/README.md).

- **Tier 2 — SMTP** (do after Tier 1):
  - `philharmonic-connector-impl-email-smtp`

  Single connector, discrete scope, `lettre`-based. Falls
  naturally into the gap between Tier 1 completion and the
  Tier 3 restart window.

- **Tier 3 — additional LLM providers** (deferred until
  after Japan's Golden Week holidays; restart **on or after
  2026-05-07 (木)**):
  - `philharmonic-connector-impl-llm-anthropic`
  - `philharmonic-connector-impl-llm-gemini`

  `llm_openai_compat` 0.1.0 (Phase 6 Task 2) already
  covers OpenAI + vLLM + any OpenAI-chat-compatible server
  (Together, Groq, OpenRouter), so Anthropic + Gemini are
  the remaining first-class providers. Deferral rationale:
  Golden Week 2026 runs 2026-04-29 (昭和の日) through
  2026-05-06 (振替休日); treating the window as a
  pause/reset before a second substantive LLM-provider
  sprint avoids cramming a complex dialect-translation
  task into the last few working days before the holiday
  block. Pick back up on 2026-05-07 or later.

**Per-implementation pattern**:
1. Define config deserialization (serde struct for the
   implementation's `config` shape).
2. Define request/response types matching the capability's wire
   protocol.
3. Implement `Implementation::execute`.
4. Integration tests against the real external service where
   possible, or a testcontainer where not.
5. Unit tests covering error mapping.
6. Publish as `0.1.0`.

**Specific notes**:

- **SQL implementations** use `sqlx` for both drivers. Placeholder
  syntax matches the driver's native form (`?` for MySQL, `$1`
  for Postgres); scripts write driver-native SQL. Row format is
  dict-per-row per doc 08. `columns` array is populated even for
  empty result sets.

- **Embed and vector-search**: split-vs-unified is flagged as an
  open question in doc 14. Default to split (two crates) unless
  the first consumer's needs suggest otherwise.

- **Email SMTP**: use `lettre`. Config carries submission server
  credentials and authentication method.

**Acceptance criteria** (per crate):
- All standard acceptance: compiles, tests pass, publishes at
  `0.1.0`.
- End-to-end test with at least one real workflow that exercises
  the implementation.

---

### Phase 8 — `philharmonic-api`

**Goal**: Implement the public HTTP API.

**Target**: Phase 8 done end-to-end (sub-phases A→I) by
**Sat 2026-05-02**, alongside Phase 9 task 2 (test WebUI +
binary targets) so end-to-end exercise is in shape before
Yuka returns from Golden Week. See
[`docs/notes-to-humans/2026-04-28-0002-pre-gw-target-may-2-end-to-end.md`](docs/notes-to-humans/2026-04-28-0002-pre-gw-target-may-2-end-to-end.md)
for the working calendar, what gets cut first if the
target slips, and what stays in regardless (the three
crypto-review gates B/E/G; the 0.1.0 publish; the
testcontainers happy path).

**Reference**: `10-api-layer.md` exhaustively for the endpoint
surface, permission mapping, authentication flows, rate
limiting.

**Crates touched**: `philharmonic-api`.

**Tasks**:

1. HTTP framework: use `axum` (aligns with Tokio ecosystem).

2. Tenant + scope resolution as a deployment-supplied input.
   The crate exposes a `RequestScopeResolver` trait with one
   async method that returns a `RequestScope` enum:
   ```rust
   #[async_trait]
   pub trait RequestScopeResolver: Send + Sync + 'static {
       async fn resolve(
           &self,
           parts: &http::request::Parts,
       ) -> Result<RequestScope, ResolverError>;
   }

   pub enum RequestScope {
       Tenant(EntityId<Tenant>),
       Operator,
   }
   ```
   Plugged in at app construction via
   `Arc<dyn RequestScopeResolver>`. The implementation can
   read a subdomain, a path prefix, a TLS client-cert
   SAN/CN, a fixed-tenant constant, or anything else — the
   framework is agnostic. Doc 10 §"Request routing"
   enumerates the shapes deployments commonly pick.
   Middleware calls the resolver, attaches the resulting
   `RequestScope` to the request context, and rejects
   requests the resolver can't classify with a structured
   error.

3. Authentication middleware:
   - Parse `Authorization: Bearer <token>` header.
   - Route: `pht_`-prefixed → long-lived lookup; otherwise parse
     as COSE_Sign1 for ephemeral.
   - Attach `AuthContext` enum (`Principal` or `Ephemeral`) to
     the request.

4. Authorization middleware:
   - Each endpoint declares its required permission atom.
   - For `Principal`: evaluate against role memberships.
   - For `Ephemeral`: check `permissions` claim directly.
   - For instance-scoped `Ephemeral`: verify instance ID in URL
     matches `instance` claim.
   - For `Operator` scope: route to the operator endpoint set;
     reject tenant endpoints. The framework defines the operator
     surface as a separate router; how the deployment exposes
     it (separate ingress, separate binary, separate port) is
     a deployment choice.

5. Endpoint implementation — follow the full surface in doc 10:
   - Workflow management (templates, instances, steps).
   - Endpoint config management (CRUD including decrypted read).
   - Principal, role, role-membership management.
   - Minting authority management.
   - Token minting endpoint.
   - Tenant settings.
   - Audit log.
   - Operator endpoints (deployment routes them to the operator
     surface via the `RequestScope` resolver).

6. Rate limiting: single-node token buckets per tenant per
   endpoint family. Use `governor` or similar.

7. Audit recording: every policy-relevant operation writes an
   `AuditEvent` entity.

8. Error envelope: every error response uses the structured
   envelope from doc 11.

9. Pagination: cursor-based on every list endpoint, default
   50, max 200. Opaque cursor string.

10. Observability middleware: correlation ID propagation,
    request logging, Prometheus metrics.

The crate ships as a library that exposes an `axum::Router`
(or equivalent constructor) plus the trait surfaces it needs
plugged in (`RequestScopeResolver`, store, executor client,
lowerer, signing keys, etc.). Whether a deployment runs it as
one process, splits it across many, or embeds it in-process
for a single user is the deployment's choice; the framework
does not prescribe.

**Sub-phase plan** (A→I; see
[`docs/notes-to-humans/2026-04-27-0003-phase-8-design-and-decisions.md`](docs/notes-to-humans/2026-04-27-0003-phase-8-design-and-decisions.md)
for the rationale). Each sub-phase is one Codex round (or
Claude housekeeping for I) with pre-landing-green at each
cut. Sub-phases B/E/G are crypto-touching — code-level
crypto review (per
[`crypto-review-protocol`](.claude/skills/crypto-review-protocol/SKILL.md))
fires before each is merged. Approach gate already approved
2026-04-28 (recorded in
[`docs/notes-to-humans/2026-04-28-0001-phase-8-decisions-confirmed.md`](docs/notes-to-humans/2026-04-28-0001-phase-8-decisions-confirmed.md)).

- **A — Skeleton.** ✅ Done 2026-04-28. axum app,
  `RequestScopeResolver` trait + middleware, request context
  type, error envelope shape, observability middleware
  (correlation ID, structured logging), placeholder
  auth/authz layers. Crate at 0.0.0.
- **B0 — Ephemeral-token primitives. Crypto-touching.**
  ✅ Done 2026-04-28 (Gate-1 + Gate-2 approved).
  `EphemeralApiTokenClaims` + `ApiSigningKey` +
  `mint_ephemeral_api_token` + 14-step
  `verify_ephemeral_api_token` + `ApiVerifyingKeyRegistry`
  in `philharmonic-policy 0.2.0` (not yet published;
  `[patch.crates-io]` bridges). 2 positive KATs + 19
  negative vectors + proptest fuzz. Codex audit findings
  (trailing bytes + unknown fields) fixed inline.
- **B1 — Auth middleware. Crypto-touching (consumer).**
  ✅ Done 2026-04-28 (Gate-2 approved). Bearer parsing,
  `pht_` lookup via `find_by_content`, COSE_Sign1
  verification via B0 primitives, `AuthContext` population,
  authority-tenant binding, epoch enforcement, generic 401
  collapsing. 15 integration tests. Codex audit findings
  (oversize bearer pre-check + MySQL index migration +
  doc drift) fixed inline.
- **C — Authz + tenant scope.** ✅ Done 2026-04-28.
  Permission-atom evaluation (role-based for Principal via
  `evaluate_permission`, claim-list for Ephemeral),
  tenant-scope enforcement, instance-scope infrastructure
  (ready for D to plug in URL extraction).
  `RequiredPermission` extension pattern for per-route
  atom declaration. `ApiStore` trait introduced. 11
  integration tests.
- **D — Workflow management endpoints.** ✅ Done 2026-04-28.
  All 13 endpoints per doc 10 §228-268 (5 template + 8
  instance). Cursor pagination, WorkflowEngine integration,
  stub executor/lowerer, instance-scope enforcement.
  5 integration tests.
- **E — Endpoint config management. Crypto-touching.**
  ✅ Done 2026-04-28 (Gate-2 approved). 6 handlers per
  doc 10 §270: create (SCK encrypt), list, read metadata,
  read decrypted (SCK decrypt), rotate, retire. No
  plaintext/ciphertext in logs or metadata reads.
  `require_sck` gate on all 6 handlers. 8 integration
  tests. Codex audit finding (incomplete SCK gate) fixed.
- **F — Principal + role + minting-authority CRUD.**
  ✅ Done 2026-04-28. 16 handlers across 5 route modules:
  principal (create + list + rotate + retire), role (create
  + list + modify + retire), membership (assign + remove),
  minting authority (create + list + rotate + bump-epoch +
  retire + modify). Long-lived `pht_` token generation
  (returned once; only SHA-256 hash persists; no token in
  logs). 7 integration tests.
- **G — Token minting endpoint. Crypto-touching.**
  ✅ Done 2026-04-28 (Gate-2 approved). `POST /v1/tokens/mint`
  per doc 10 §355. Codex audit fixes: builder validates
  kid+issuer against registry, post-serialization
  `MAX_TOKEN_BYTES` guard, permission-atom validation against
  `ALL_ATOMS`, duplicate dedup. 9 integration tests.
- **H — Audit + rate limit + tenant-admin + operator
  endpoints.** ✅ Done 2026-04-28. Tenant settings
  read/update, audit-event list (paginated + filterable),
  in-memory token-bucket rate limiting per tenant per
  endpoint family (429 + Retry-After), operator tenant
  create/suspend/unsuspend. 9 integration tests.
- **I — Publish.** ✅ Done 2026-04-28. `philharmonic-policy`
  0.2.0 + `philharmonic-api` 0.1.0 published to crates.io.
  CHANGELOGs finalized. Tags `v0.2.0` / `v0.1.0` in
  respective submodules.

**Phase 8 status: COMPLETE** (2026-04-28). All nine
sub-phases (A→I) landed in a single day. Three crypto-review
cycles (B0, E, G) passed both gates. Five Codex audit rounds
with inline fixes. 86 integration tests across the API crate.
`philharmonic-api 0.1.0` and `philharmonic-policy 0.2.0`
published.

**Acceptance criteria** (met):
- ✅ Every endpoint in doc 10 implemented with correct
  permission enforcement.
- ✅ Every endpoint has integration tests covering happy path
  and at least one auth/permission failure path.
- ✅ Rate limiting observable in tests (request bursts return
  429 with `Retry-After`).
- ✅ Audit events appear in the substrate for relevant
  operations.
- ✅ `philharmonic-api` published as `0.1.0`.

---

### Phase 9 — Integration and reference deployment

**Goal**: A running deployment serving real workflows. Turn
the published library crates into executable processes with
TLS, config-file loading, and a minimal WebUI — then prove
them end-to-end.

**Target**: Meta-crate wiring + at least one running bin
target + testcontainers e2e happy path by **Sat 2026-05-02**.
Phase 8 completed 2026-04-28; the remaining window is for
Phase 9 work only. The chat-app-shaped ephemeral-token flow,
the `install` subcommand, musl cross-compilation, and Docker
compose can slip past 5/2 if scope pressure forces a cut. See
[`docs/notes-to-humans/2026-04-28-0002-pre-gw-target-may-2-end-to-end.md`](docs/notes-to-humans/2026-04-28-0002-pre-gw-target-may-2-end-to-end.md)
for the earlier cut-order and
[`docs/notes-to-humans/2026-04-29-0001-phase-9-integration-sketch.md`](docs/notes-to-humans/2026-04-29-0001-phase-9-integration-sketch.md)
for the full integration sketch and open questions.

**Architecture** (per HUMANS.md §Integration):

Three bin targets live in separate in-tree crates under
`bins/` at the workspace root (split from the meta-crate
2026-04-30 to isolate the 2.28 GB embed weights).
Each is an HTTP server with an
optional `https` Cargo feature (rustls TLS + HTTP/2). SIGHUP
reloads config and refreshes TLS certs. TOML config at
`/etc/philharmonic/<name>.toml` with a
`<name>.toml.d/*.toml` drop-in overlay. Clap CLI with
`serve`/`version`/`help` subcommands plus a root-only
`install` subcommand (copies binary + writes systemd unit +
creates config dirs + `systemctl enable`). All three must
compile for `x86_64-unknown-linux-musl`.

- `mechanics-worker` — wraps the `mechanics` HTTP service.
  The `mechanics` crate itself must be extended first with an
  `https` feature flag (rustls TLS support), then this bin
  wraps it with the shared server infra (Clap + config).
  **First bin to build** — simplest scope, proves the pattern
  (confirmed 2026-04-29).
- `philharmonic-connector` — the **connector service**
  entry point. Wraps `philharmonic-connector-service` + all
  shipped `Implementation` trait impls. Receives forwarded
  requests (from the connector router), verifies COSE_Sign1
  tokens via `MintingKeyRegistry`, decrypts hybrid-KEM
  payloads via `RealmPrivateKeyRegistry`, and dispatches to
  the appropriate `Implementation`. Feature-gated connector
  selection (all default-on). Run one per realm. **This is
  NOT the connector router** — the router lives in the API
  binary (see below).
- `philharmonic-api` — faces the internet (HTTPS at 443).
  Wires `philharmonic-api` library + store backend + policy +
  SCK + signing keys. Embeds the WebUI as static assets (SPA
  routing for non-API paths). Integrates the **connector
  router** (`philharmonic-connector-router`) as an embedded
  component — the API binary dispatches connector requests
  to upstream `philharmonic-connector` service instances
  (confirmed 2026-04-29).

The `philharmonic` meta-crate also re-exports every library
crate at its top level and feature-gates connector impls
(`default-features = false` to pick individually). Unshipped
impls (Anthropic, Gemini, SMTP) get off-by-default features
until their 0.1.0 lands.

**Shared server module** (`philharmonic/src/server/` or
similar — exact location TBD):

- Optional-TLS listener (rustls + tokio-rustls, gated behind
  `https` feature). The crypto backend must be vendored /
  pure-Rust-ish (aws-lc-rs or ring) — **no system OpenSSL
  headers**, no `libssl-dev` / `openssl-devel` packages.
  The build must succeed with only a Rust toolchain + a C
  compiler (for the vendored C in aws-lc-rs / ring). See
  [`CONTRIBUTING.md §10.9`](CONTRIBUTING.md#109-http-client-runtime-stack-vs-tooling-stack)
  for the full TLS-stack rule.
- SIGHUP signal handler (re-read config + refresh certs).
- TOML config loader (primary file + `.d/*.toml` overlay,
  merged in lexicographic order, location overridable via CLI).
- Clap CLI skeleton shared across bins.

**WebUI**: Redux + React + Webpack (confirmed 2026-04-29 —
this powers the first deployment and must be extensible).
Build artifacts committed to Git (`index.html`, `main.js`,
`main.css`, `icon.svg` from `common-assets/d-icon.svg`). No
Node.js needed at Rust build time — the JS build is a
separate human-triggered step. The Rust binary embeds the
committed artifacts. Minimum viable surface: login (paste
`pht_` token), workflow-template CRUD, instance lifecycle,
step execution, audit log.

**Tasks**:

1. **Meta-crate wiring** (step 1 in the sketch):
   ✅ Add all published library crate deps to
     `philharmonic/Cargo.toml`.
   ✅ Re-export each at top level.
   ✅ Feature-gate connector impls.
   - Bump to `0.1.0` and publish. **Pre-req**: publish the
     three unshipped connector impl placeholders
     (`llm-anthropic`, `llm-gemini`, `email-smtp`) to
     crates.io first — `cargo publish` resolves all deps
     (including optional) against the registry.

2. ✅ **Extend `mechanics` crate with `https` feature**
   (landed 2026-04-29):
   `TlsConfig::from_pem` + `MechanicsServer::run_tls` with
   rustls TLS + HTTP/1.1 / HTTP/2 ALPN via `tokio-rustls` +
   `hyper_util::server::conn::auto::Builder`. Vendored crypto
   backend (aws-lc-rs), no system OpenSSL headers.

3. ✅ **Shared server infrastructure** (landed 2026-04-29):
   `philharmonic/src/server/` module with `cli` (Clap
   `BaseArgs` + `BaseCommand` + `resolve_config_paths`),
   `config` (generic `load_config<T>` with TOML drop-in
   overlay merge), `reload` (`ReloadHandle` with SIGHUP +
   generation counter + `Notify` fan-out). 10 tests.

4. **Binary targets** (three bins in `bins/` at the
   workspace root, split from meta-crate 2026-04-30):
   - ✅ `mechanics-worker` (landed 2026-04-29): Clap CLI +
     TOML config (with drop-in overlays) + SIGHUP reload
     (token replacement via `replace_tokens`) + optional
     TLS (`--features https`). Default config fallback when
     no config file exists.
   - ✅ `philharmonic-connector` (reworked 2026-04-29):
     Connector **service** (not router). `POST /` handler:
     extract COSE_Sign1 token + encrypted payload →
     `verify_and_decrypt()` → `Implementation` trait
     dispatch. Six shipped impls feature-gated. Key
     registries hot-reloadable via SIGHUP. Error envelope
     maps `ImplementationError` variants to HTTP status.
   - ✅ `philharmonic-api` (landed 2026-04-30, without WebUI):
     MySQL store via `SinglePool::connect` (no direct sqlx
     dep), `ApiSigningKey` + `ApiVerifyingKeyRegistry`,
     optional SCK, embedded connector router at `/connector/`,
     `HeaderBasedScopeResolver` (placeholder),
     `StubExecutor`/`StubLowerer`, configurable rate limit
     overrides, dynamic router via `Arc<RwLock<Router>>` +
     `tower::oneshot` for SIGHUP hot-swap. WebUI embedded
     via `rust-embed` + SPA fallback routing (landed same
     day after task 5).

5. ✅ **WebUI** (landed 2026-04-30):
   React 19 + Redux Toolkit + TypeScript + Webpack 5. 8 pages
   (Login, Dashboard, Templates, Template Detail, Instances,
   Instance Detail, Audit Log, Tenant Settings), cursor
   pagination, JSON viewer, auth via `pht_` token in
   sessionStorage. 274K JS + 6.1K CSS + source maps committed
   in `philharmonic/webui/dist/`. Build via
   `./scripts/webui-build.sh --production`. Embedded in the
   API binary via `rust-embed` + SPA fallback routing
   (same day).

6. ✅ **End-to-end integration test suite** (landed 2026-04-30):
   - `testcontainers` for MySQL.
   - Spawn all three bins (in-process or child processes).
   - Test flows: tenant admin creates endpoint config; creates
     workflow template; caller executes steps; instance reaches
     terminal state; audit log records all of it.
   - Ephemeral-token flow: minting authority mints
     instance-scoped token; client executes steps with it;
     subject identity appears in step records.

7. ✅ **Real `ConfigLowerer` implementation** (landed 2026-04-30):
   Replace `StubLowerer` with a real implementation that
   wraps `philharmonic-connector-client`. Takes abstract
   endpoint config + request, mints a COSE_Sign1 authorization
   token, encrypts the payload via COSE_Encrypt0 with the
   realm's public KEM key, and produces the wire-format
   payload for the connector service. **Touches crypto paths**
   — triggers the crypto-review-protocol skill (Gate-1
   approach + Gate-2 code review).

8. ✅ **Real `StepExecutor` implementation** (landed 2026-04-30):
   Replace `StubExecutor` with a real implementation that
   sends the lowered payload to upstream services via HTTP:
   - JS execution steps → `mechanics` worker
   - Connector steps → `philharmonic-connector` service
     (via the embedded connector router or direct)
   Uses `reqwest` + `rustls-tls` per §10.9. Needs the
   worker/connector bind addresses from config.

   Tasks 7–8 are blocking for any workflow to actually
   execute steps end-to-end. The e2e tests (task 6) can
   exercise CRUD and auth without them, but the "execute
   step → terminal state" flow requires real executor +
   lowerer.

9. ✅ **`install` subcommand** (landed 2026-04-30):
    - Per-bin: copies binary to `/usr/local/bin/`, writes
      systemd unit, creates config dirs, `systemctl enable`.
    - Idempotent. Prints setup instructions at the end.

10. ✅ **musl target build** (landed 2026-04-30):
    - Verify `x86_64-unknown-linux-musl` for all three bins.
      This is a standard rustup target, not a hard cross-compile
      — the crate family is pure Rust + vendored C (aws-lc-rs
      via `cc`), no system libraries, no OpenSSL. Should work
      with `rustup target add x86_64-unknown-linux-musl` +
      `cargo build --target x86_64-unknown-linux-musl`.
      Requires `musl-tools` (`apt install musl-tools`) for
      the vendored C in aws-lc-rs. `.cargo/config.toml`
      configures the linker + CC automatically.
      `./scripts/musl-build.sh` builds all three bins.
    - CI target.

11. ✅ **Reference deployment** (operational 2026-05-02):
    - Three binaries deployed on developer infrastructure.
    - Realm KEM keypairs, SCK, signing keys deployed.
    - One tenant provisioned (developer).
    - One workflow exercised end-to-end with real traffic:
      OpenAI-compatible LLM via `llm_openai_compat` connector,
      tested through the WebUI. Full path verified: API server
      → lowerer (COSE_Sign1 + COSE_Encrypt0) → mechanics
      worker (JS execution) → connector router (path-based
      dispatch) → connector service (verify + decrypt +
      implementation dispatch) → upstream LLM → response back
      through the chain.
    - Multiple deployment bugs found and fixed during testing
      (see `docs/notes-to-humans/2026-05-01-000{4,5,6}-*.md`).

12. ✅ **Docker compose** (landed 2026-04-30):
    - Minimal Alpine images, no `install` subcommand inside
      containers.
    - Local override files for HTTPS certs, hostnames.

13. ✅ **Documentation reconciliation** (landed 2026-05-01):
    - Updated design docs 08, 15 where implementation diverged
      (connector-client is pure crypto, not full lowerer;
      connector-service does not host Implementation registry).
    - Fixed stale crate counts (23→25), submodule counts
      (24→26), phase status in ROADMAP preamble.
    - Fixed CONTRIBUTING.md §8.1 target-dir claim.
    - Updated member crate READMEs (api, types, meta-crate).
    - Added `pub(crate)` + doc comments across all crates;
      gated pre-landing on `RUSTDOCFLAGS="-D missing_docs"`.

**Cut order for 5/2** (must → should → can-slip):

- **Done**: tasks 1–5 (meta-crate, mechanics HTTPS, shared
  infra, all three bins, WebUI + embedding).
- **Must**: task 6 (e2e happy path — CRUD + auth, without
  real step execution).
- **Should**: tasks 7–8 (real lowerer + executor — needed
  for any workflow to run real steps).
- **Can slip**: tasks 9–12 (install, musl, deployment,
  Docker).
- **Always last**: task 13 (doc reconciliation after code
  settles).

**Acceptance criteria**:
- ✅ End-to-end suite passes in CI.
- ✅ Reference deployment operational, accepting tenant API
  calls.
- ✅ At least one workflow running real traffic (LLM-driven,
  `llm_openai_compat` connector, end-to-end through all
  components, verified 2026-05-02).
- Remaining: run for at least a week without incident;
  additional connector implementations (Phase 7 Tier 2/3,
  deferred post-GW).

---

## 5. Crypto review protocol

**Yuka reviews personally.** Do not publish crypto-sensitive
crates without her sign-off. Crypto-sensitive paths include:

- SCK encrypt/decrypt (`philharmonic-policy`).
- COSE_Sign1 signing and verification
  (`philharmonic-connector-client`,
  `philharmonic-connector-service`).
- COSE_Encrypt0 encryption and decryption (same two crates).
- The ML-KEM-768 + X25519 + HKDF + AES-256-GCM hybrid
  construction.
- Payload-hash binding logic.
- API token generation (`pht_` format).

### Review cadence

Crypto-adjacent work is gated at both ends:

1. **Pre-approval of approach, before coding.** Before Codex
   (or Claude) writes any of the primitives above, produce a
   short written proposal — the exact primitives, construction
   order, HKDF inputs, AEAD associated data, nonce scheme, key
   derivation / rotation story, zeroization points — and get
   Yuka's sign-off on the design. This is the cheap place to
   catch misuse; bugs found here cost nothing beyond the doc.
2. **Post-review of code, before publish.** Once the
   implementation exists, Yuka reviews the actual code and the
   committed test vectors line-by-line. A crate touching the
   list above does not get a `cargo publish` until this review
   is complete and all review comments are resolved.

Neither gate is waivable. A pre-approved approach doesn't skip
post-review; a clean post-review doesn't retroactively bless an
approach that was never pre-approved.

**Test vector discipline.** For every crypto operation, write
tests with known inputs and exact expected outputs. Round-trip
tests alone (encrypt-then-decrypt, sign-then-verify) can pass
while both sides are wrong in matching ways — they're necessary
but not sufficient.

Format for test vectors in `tests/crypto_vectors.rs`:

```rust
#[test]
fn sck_encrypts_to_known_ciphertext() {
    let sck = hex!("00112233...");
    let nonce = hex!("0011...");
    let plaintext = br#"{"realm":"llm",...}"#;
    let expected = hex!("aabbccdd...");
    let actual = sck_encrypt(&sck, &nonce, plaintext).unwrap();
    assert_eq!(actual, expected);
}
```

Generate expected outputs once, by hand or with a reference
implementation (Python `cryptography` library, for instance).
Commit the hex-encoded expected bytes.

**What to flag to Yuka.** Surface the following immediately when
starting crypto work:

- Your understanding of the hybrid KEM construction (order of
  KEM + ECDH, HKDF inputs, AEAD additional-data choice). Confirm
  before implementing.
- Any use of `unsafe` in or near crypto code.
- Any handling of key material that isn't zeroized after use.
- Any places where signatures or MACs are computed over
  untrusted input.

---

## 6. Pitfalls

A partial list of places implementation tends to go wrong:

**Substrate append-only discipline.** No `UPDATE`, no `DELETE`.
State changes are new revisions. Retirement is a scalar on a
new revision. Tests and helper code that "just update this row
for convenience" are a smell — rewrite to append.

**Workspace dependency spec.** Always use `{ version = "X.Y.Z",
path = "submodule-dir" }` in `[workspace.dependencies]`. Path
alone breaks publishing; version alone breaks local
cross-crate development.

**Submodule pointer drift.** The parent's pointer must track
pushed commits in each submodule. If you find yourself making a
commit in the parent that references a submodule commit that
only exists locally, you've broken the shared state. Fix by
pushing the submodule first.

**Capability-name collisions.** Capability names live in doc 08
(`llm_generate`, `sql_query`, `http_forward`, etc.). Don't
invent new ones. Implementation names (like `llm_openai_compat`)
are separate from capability names; both should match what doc
08 says exactly.

**COSE algorithm identifiers.** Use standard COSE algorithm IDs,
not invented ones. For hybrid KEM with ML-KEM-768 + X25519,
check what the `coset` crate ships with and what IETF has
registered as of now. The hybrid mode may need a custom
identifier if none is standardized yet; document the choice
in a comment and note it in doc 11 for later IETF tracking.

**Boa leaking into the lowerer.** The entire point of extracting
`mechanics-config` was to keep Boa out of
`philharmonic-connector-client`. Verify with
`cargo tree -p philharmonic-connector-client | grep -i boa`
which must return empty.

**Tenant ID as a metric label.** Resist the urge. Even in tests,
this creates habits that leak into production and blow up
cardinality. Per-tenant observability goes through logs.

**Using `sqlx` query macros across drivers.** The `query!()` and
`query_as!()` macros compile SQL at build time against a specific
database. For runtime-generated queries (which is what
`sql_query` does), use `sqlx::query()` and `sqlx::query_as()` —
the non-macro forms. Keep the macro forms for infrastructural
queries with fixed SQL (substrate schema queries, for example).

**Browser storage in artifacts.** Irrelevant here (no
artifacts); mentioned for completeness. No localStorage in
anything you build.

**Inventing `SubjectContext` fields.** The struct shape in doc
07 is exact. Adding a field — even one that seems useful —
breaks everyone who consumes the type. Push back through design
docs first.

---

## 7. When stuck

**First**: re-read the relevant design doc section. Nine times
out of ten the answer is there.

**Second**: check `14-open-questions.md`. If the thing you're
stuck on is listed as genuinely open, that's the signal to flag
it rather than invent.

**Third**: surface to Yuka with a specific question. Not
"what should I do about X?" but "my options for X appear to
be A, B, or C; A conflicts with doc 08 section Y; B has cost
Z; C is what I'd pick unless you have a reason to prefer
otherwise."

---

## 8. Definition of done for v1

Philharmonic v1 is shipped when:

- All 25 crates published at `0.1.0` or higher on crates.io.
- Reference deployment live on the developer's infrastructure with:
  - TLS-terminated API subdomain pattern working.
  - At least one tenant provisioned.
  - At least one non-trivial workflow in production use.
  - Audit log capturing real activity.
- End-to-end integration tests passing in CI.
- Design docs reconciled with implementation reality.
- No open crypto-review items.
- Rate limiting observably functional.
- Error envelope uniformly in use.
- Observability stack wired: structured logs flowing, metrics
  exposed, correlation IDs propagating.

Post-v1 work (`12-deferred-decisions.md`, `14-open-questions.md`
remaining items) begins after this point. Don't mix post-v1
scope into v1 phases.

---

## 9. Project-level files (current inventory)

The files in place at the workspace root and inside each
submodule, as of Phase 0's completion. Maintain these through
ongoing development.

**Workspace root** (`philharmonic-workspace`):

- `Cargo.toml` — workspace manifest: members, shared
  dependencies, `[patch.crates-io]` redirecting Philharmonic
  crate deps to local submodule paths.
- `README.md` — public-facing overview + contributor-facing
  workflow (cloning, scripts, pre-landing, publishing,
  AI-assisted development split).
- `ROADMAP.md` — this file. Living; updated in the same commit
  as work it describes.
- `CLAUDE.md` — Claude Code's session bootstrap: pointers to
  the roadmap, conventions, and the mandatory rules (scripts
  for Git, POSIX shell, pre-landing checks, journal filename
  format, notes-to-humans, HUMANS.md read-only, etc.).
- `AGENTS.md` — Codex's bootstrap, mirroring the Claude/Codex
  division from Codex's side (implementer role, no commits, no
  branches, POSIX-sh if writing shell, crypto paths flag-only).
- `HUMANS.md` — Yuka's own notes-to-self. Agent-readable,
  agent-immutable (enforced by
  `docs/instructions/README.md §HUMANS.md`). `commit-all.sh`
  auto-includes pending edits via `git add -A`.
- `LICENSE-APACHE`, `LICENSE-MPL` — dual-license text.
- `.gitignore` — `target/`, editor droppings, Codex session
  state under `.codex/`.
- `.gitattributes` — LF normalization, text/binary boundaries,
  `linguist-generated=true` on `Cargo.lock`.
- `.editorconfig` — matches rustfmt defaults (space, 4-wide,
  100-col).
- `.codex/config.toml` — project-local Codex CLI config
  (sandbox, approvals, reasoning effort). Activated via
  `CODEX_HOME=$PWD/.codex`.
- `.github/workflows/ci.yml` — GitHub Actions CI running
  `setup.sh` + `pre-landing.sh` on push-to-main and PRs.
- `scripts/*.sh` — the workspace script surface (see Phase 0
  "Infrastructure added beyond the plan" above for the full
  list).
- `scripts/lib/workspace-cd.sh` — sourced helper for
  workspace-root resolution with `$0`-path fallback.
- `docs/design/*.md` — architectural design docs (what
  Philharmonic *is*).
- `CONTRIBUTING.md` — workspace development conventions
  (authoritative; most-frequently-updated).
- `docs/codex-prompts/YYYY-MM-DD-NNNN-<slug>[-NN].md` —
  archived Codex prompts.
- `docs/notes-to-humans/YYYY-MM-DD-NNNN-<slug>[-NN].md` —
  Claude's significant-finding journal for Yuka.
- `docs/instructions/README.md` — human-to-agent rules that
  don't fit in CLAUDE.md or AGENTS.md (currently: HUMANS.md
  access rule).

**Per-submodule** (each crate repo):

- `Cargo.toml` — package manifest. Depends on other
  philharmonic crates via crates.io version spec; the parent's
  `[patch.crates-io]` redirects to local paths when building
  from the workspace.
- `src/lib.rs` (or `src/main.rs` for binaries).
- `README.md` — crate-level overview and usage.
- `CHANGELOG.md` — Keep-a-Changelog format; updated with every
  release.
- `LICENSE-APACHE`, `LICENSE-MPL` — per-submodule copies so
  standalone consumers get them.
- `.editorconfig`, `.gitattributes` — synced from the parent.
- `.github/workflows/ci.yml` — per-crate CI running `cargo
  check`, `cargo test`, `cargo clippy`, `cargo fmt --check`
  against the crate standalone (per-crate publishability
  requirement).

---

## 10. Quick reference — command shortcuts

From the workspace root:

```bash
# Status across all submodules
./scripts/status.sh

# Pull latest on all submodules' tracked branches
./scripts/pull-all.sh

# Commit pending changes in each submodule, then the parent
# (bumps submodule pointers). Message defaults to "updates".
./scripts/commit-all.sh "your message"

# Push pushed-pending commits across every submodule
./scripts/push-all.sh

# Pre-landing checks (mandatory before committing Rust code).
# Auto-detects modified crates and runs lint + workspace test +
# --ignored per modified crate. One command, all phases.
./scripts/pre-landing.sh
```

Publishing a crate (from the workspace root):

```bash
# API-breakage check against the latest crates.io release
# (pass a specific version as the second argument to override)
./scripts/check-api-breakage.sh <crate>

# Dry-run publish; stops before real publish, no tag created
./scripts/publish-crate.sh --dry-run <crate>

# Real publish: runs cargo publish, tags v<version> in the submodule
# on success, leaves the tag for the next push-all.sh to ship
./scripts/publish-crate.sh <crate>

# Push the tag alongside any other pending commits
./scripts/push-all.sh
```

---

End of roadmap. Begin with Phase 0 if setup isn't complete, or
Phase 1 otherwise. Consult design docs liberally. Flag anything
crypto-adjacent for Yuka's review.