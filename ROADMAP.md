# Philharmonic Implementation Roadmap

**Audience**: Claude Code, working in the `philharmonic-workspace` repo.

**Purpose**: A linear plan for implementing the Philharmonic crate
family from current state to v1 MVP (working end-to-end deployment
serving real tenants).

**Current state** (2026-04-21):

- Design: complete.
- Phase 0 (workspace setup): **done**, with substantial added
  infrastructure beyond the original scope — see the Phase 0
  section below and §9.
- Phase 1 (`mechanics-config` extraction): **implementation
  complete in git** (`mechanics-config 0.1.0`, `mechanics-core
  0.2.3`). Publish to crates.io: pending.
- Phases 2–9: not started.

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
- 23 submodule directories, one per crate, each being a separate
  Git repo at `github.com/metastable-void/<crate-name>`.
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
- `13-conventions.md` — code conventions.
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
first and update `docs/design/13-conventions.md §Git workflow`.

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
`docs/design/13-conventions.md §Pre-landing checks`.

**Follow append-only discipline.** The substrate has no `UPDATE`
or `DELETE` semantics. Entity state changes are new revisions;
soft-deletes are revisions with an `is_retired` scalar set true.
If you find yourself wanting to "just update a field," re-read
`02-design-principles.md` and `05-storage-substrate.md`.

**MSRV and edition.** Edition 2024, `rust-version = "1.85"`, set
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
  each of the 23 crates.
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
  parse check), `rust-lint.sh`, `rust-test.sh`, `pre-landing.sh`
  (mandated pre-landing driver), `check-toolchain.sh`,
  `check-api-breakage.sh`, `publish-crate.sh`, `cargo-audit.sh`,
  `crate-version.sh`, `codex-status.sh`, plus
  `scripts/lib/workspace-cd.sh` shared helper. All POSIX sh,
  validated by `test-scripts.sh` (and CI).
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
  via `--follow-tags`). `./scripts/check-api-breakage.sh`
  against the previous tag before any new release.

**Acceptance criteria**:
- `git submodule status` shows all 23 submodules at a clean
  commit. ✓
- `cargo check --workspace` succeeds. ✓
- `./scripts/status.sh` runs without error. ✓
- `./scripts/pre-landing.sh` passes at the workspace root. ✓
- `./scripts/test-scripts.sh` passes against every `scripts/*.sh`
  and `scripts/lib/*.sh`. ✓
- GitHub Actions CI green on `main`.

---

### Phase 1 — Extract `mechanics-config`

**Status**: **Implementation complete in git; publish pending.**

Landed work (see `docs/codex-prompts/2026-04-20-0001-phase-1-*`
for the Codex prompts and the commits they produced):

- `mechanics-config 0.1.0` created with the Boa-free schema
  types and pure structural validation. `cargo tree -p
  mechanics-config | grep -iE 'boa|reqwest|tokio'` is empty.
- `mechanics-core 0.2.3` depends on `mechanics-config = "0.1.0"`,
  wraps the extracted types with Boa GC newtypes
  (`#[unsafe_ignore_trace]`), re-exports at
  `mechanics_core::endpoint::*` and
  `mechanics_core::job::MechanicsConfig` for back-compat.
- Behavior change: schema validation now fails at config-
  construction time instead of job-call time (noted in
  `mechanics-core/CHANGELOG.md 0.2.3`).
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

**Remaining work**:

1. `./scripts/check-api-breakage.sh v0.2.2` on `mechanics-core`
   to confirm the extraction is non-breaking at the public
   surface. (The public API re-exports the types at the old
   paths, so this should pass; run it to confirm before
   publishing.)
2. `./scripts/publish-crate.sh --dry-run mechanics-config`
   then the real publish.
3. `./scripts/publish-crate.sh --dry-run mechanics-core` then
   real publish. Must be in this order — `mechanics-core 0.2.3`
   pins `mechanics-config = "0.1.0"`, so the dep has to be on
   crates.io first.
4. `./scripts/push-all.sh` to ship the `v0.1.0` and `v0.2.3`
   tags alongside branch commits.

**Acceptance criteria** (remaining):
- `mechanics-config 0.1.0` published on crates.io. _(pending)_
- `mechanics-core 0.2.3` published on crates.io. _(pending)_
- `cargo tree -p mechanics-config | grep -iE 'boa|reqwest|tokio'`
  returns empty. ✓
- Workspace `cargo test --workspace` passing
  (`./scripts/rust-test.sh`). ✓
- Mechanics-core integration tests passing
  (`./scripts/rust-test.sh --ignored mechanics-core`).
  _(run locally before publish)_

---

### Phase 2 — `philharmonic-policy`

**Goal**: Implement the policy layer entity kinds and basic
operations.

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
- All seven entity kinds compile and pass their validation.
- SCK crypto tests include at least three test vectors (keys,
  plaintexts, expected ciphertexts) that are hand-verifiable.
- Permission evaluation tests cover: permission granted,
  permission denied, retired role, retired membership, retired
  principal, suspended tenant.
- `philharmonic-policy` publishes as `0.1.0`.

---

### Phase 3 — `philharmonic-connector-common`

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
  (`philharmonic-types`, `mechanics-config`, `coset`, `serde`).
- Unit tests for token claim serde round-trip and realm
  registry lookup by kid.
- `philharmonic-connector-common` publishes as `0.1.0`.

---

### Phase 4 — `philharmonic-workflow`

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
  integration tests.
- Full status-transition state machine coverage.
- `philharmonic-workflow` publishes as `0.1.0`.

---

### Phase 5 — Connector triangle (CRYPTO — Yuka reviews)

**Goal**: Implement the three connector crates that together
carry the crypto-sensitive wire protocol. **Yuka reviews this
code personally.** Do not publish these without her sign-off.

**Reference**: `08-connector-architecture.md`,
`11-security-and-cryptography.md` exhaustively.

**Crates touched**: `philharmonic-connector-client`,
`philharmonic-connector-router`, `philharmonic-connector-service`.

**Tasks**:

1. **`philharmonic-connector-client`** (the lowerer):
   - Implement `ConfigLowerer` trait from `philharmonic-workflow`.
   - On each `lower()` call:
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
     h. Mint a COSE_Sign1 token with claims `iss, exp, kid, realm,
        tenant, inst, step, config_uuid, payload_hash`. Sign
        with the lowerer's Ed25519 key.
     i. Assemble the `MechanicsConfig` entry: POST to
        `<realm>.connector.our-domain.tld`, `Authorization:
        Bearer <COSE_Sign1 bytes, base64url>`,
        `X-Encrypted-Payload: <COSE_Encrypt0 bytes, base64url>`.

2. **`philharmonic-connector-router`** (pure dispatcher):
   - Minimal HTTP server that terminates TLS for
     `<realm>.connector.our-domain.tld` and forwards to connector
     service instances in the realm.
   - No token verification, no decryption, no rate limiting.
   - Round-robin or least-connections load balancing is fine;
     pick whatever's simple.

3. **`philharmonic-connector-service`** (service framework):
   - HTTP listener accepting POST requests.
   - Verify COSE_Sign1 signature by `kid` against the registered
     lowerer public key.
   - Check token `exp` not passed.
   - Compute SHA-256 of encrypted payload; verify against
     `payload_hash` claim.
   - Check token `realm` claim matches this binary's realm.
   - Decrypt COSE_Encrypt0 with realm private key by `kid`.
   - Parse decrypted JSON; verify inner `realm` field matches
     token `realm` (belt-and-suspenders).
   - Look up `impl` field in the implementation registry; reject
     if unknown.
   - Dispatch: call `handler.execute(config_subobject, request,
     ctx)` where `ctx` is built from the verified token claims.
   - Wrap result in HTTP response; map `ImplementationError`
     variants to appropriate HTTP status codes.

4. **Crypto test vectors.** Generate before implementation is
   finalized:
   - Fixed ML-KEM-768 keypair, fixed X25519 keypair, fixed
     Ed25519 keypair.
   - Fixed plaintext payload.
   - Expected COSE_Encrypt0 bytes.
   - Expected COSE_Sign1 bytes.

   Test that encrypt-then-decrypt and sign-then-verify round-trip
   through the exact expected ciphertext/signature. This catches
   nonce-reuse, HKDF-input-ordering, and AEAD-additional-data
   bugs that round-trip-only tests miss.

**Acceptance criteria**:
- Crypto test vectors pass in isolation (known inputs → known
  outputs, not just round-trip).
- Cross-crate integration test: `connector-client` encrypts,
  `connector-service` decrypts, content matches exactly.
- Yuka has reviewed the crypto code paths and signed off.
- No `unsafe` blocks in crypto code.
- No custom crypto primitives — only `ml-kem`, `x25519-dalek`,
  `aes-gcm`, `ed25519-dalek`, `hkdf`, `sha2` from RustCrypto.

---

### Phase 6 — First implementations

**Goal**: Ship two connector implementations that together prove
the end-to-end path works: one generic HTTP implementation
(simplest) and one LLM implementation (unblocks the chat-app use
case).

**Reference**: `08-connector-architecture.md`, specifically the
full wire protocol specs under "v1 implementation set".

**Crates touched**:
`philharmonic-connector-impl-http-forward`,
`philharmonic-connector-impl-llm-openai-compat`.

**Tasks**:

1. **`http_forward`**:
   - Config shape reuses `mechanics_config::HttpEndpoint`. Do not
     reinvent; depend on `mechanics-config` and use its type.
   - Request shape: `{url_params, query, headers, body}`,
     validated against the config's `HttpEndpoint`.
   - Response shape: `{status, headers, body}`. Headers filtered
     to the config's `exposed_response_headers`.
   - Error handling: upstream 4xx/5xx returns as a normal
     response (not an error); only network/timeout failures
     surface as `ImplementationError`.
   - Use `reqwest` with `rustls-tls`.
   - Integration tests against `httpbin.org` or a local test
     server.

2. **`llm_openai_compat`**:
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
   - Integration tests: real OpenAI API (minimal, cheap model) and
     a local vLLM container for `vllm_native`.

**Acceptance criteria**:
- End-to-end flow works: create tenant endpoint config for
  `http_forward`, create workflow template referencing it,
  execute a step, see the HTTP call land at the target.
- Same end-to-end flow for `llm_openai_compat` with OpenAI as
  the target.
- Unit tests cover dialect translation with fixed expected
  provider-specific request bodies (test vectors for request
  shape too, not just round-trips).
- Both crates publish as `0.1.0`.

---

### Phase 7 — Additional implementations (parallel-safe)

**Goal**: Ship the remaining implementations. These can be worked
on in any order since they don't block each other.

**Reference**: `08-connector-architecture.md` for each
implementation's wire protocol.

**Crates touched** (one crate each):
- `philharmonic-connector-impl-llm-anthropic`
- `philharmonic-connector-impl-llm-gemini`
- `philharmonic-connector-impl-sql-postgres`
- `philharmonic-connector-impl-sql-mysql`
- `philharmonic-connector-impl-email-smtp`
- `philharmonic-connector-impl-embed`
- `philharmonic-connector-impl-vector-search`

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

**Reference**: `10-api-layer.md` exhaustively for the endpoint
surface, permission mapping, authentication flows, rate
limiting.

**Crates touched**: `philharmonic-api`.

**Tasks**:

1. HTTP framework: use `axum` (aligns with Tokio ecosystem).

2. Subdomain routing:
   - `<tenant>.api.our-domain.tld/v1/...` for tenant endpoints.
   - `admin.our-domain.tld/v1/...` for operator endpoints.
   - Middleware resolves `<tenant>` from the Host header and
     attaches it to request context.

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

5. Endpoint implementation — follow the full surface in doc 10:
   - Workflow management (templates, instances, steps).
   - Endpoint config management (CRUD including decrypted read).
   - Principal, role, role-membership management.
   - Minting authority management.
   - Token minting endpoint.
   - Tenant settings.
   - Audit log.
   - Operator endpoints on `admin.` subdomain.

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

**Acceptance criteria**:
- Every endpoint in doc 10 implemented with correct permission
  enforcement.
- Every endpoint has integration tests covering happy path and
  at least one auth/permission failure path.
- Rate limiting observable in tests (request bursts return 429
  with `Retry-After`).
- Audit events appear in the substrate for relevant operations.
- `philharmonic-api` publishes as `0.1.0`.

---

### Phase 9 — Integration and reference deployment

**Goal**: A running deployment serving real workflows.

**Tasks**:

1. End-to-end integration test suite:
   - `testcontainers` for MySQL.
   - Compose up: API + mechanics worker + connector router +
     connector service (single-realm config bundling all impls).
   - Test flows: tenant admin creates endpoint config; tenant
     admin creates workflow template; tenant caller executes
     steps; instance reaches terminal state; audit log records
     all of it.
   - Chat-app flow: minting authority mints instance-scoped
     ephemeral token; browser-equivalent caller executes steps
     with the ephemeral token; subject identity appears in step
     records.

2. Reference deployment on the developer's infrastructure:
   - Deploy the binaries to the developer's on-prem or IaaS.
   - TLS certificates for `*.api.our-domain.tld`,
     `admin.our-domain.tld`, `*.connector.our-domain.tld`.
   - Actual realm KEM keypairs generated and deployed to realm
     connector service binaries; public keys deployed to the
     lowerer.
   - Actual SCK deployed.
   - At least one tenant provisioned (the developer itself).
   - At least one working workflow (probably the chat-app
     end-to-end).

3. Documentation reconciliation:
   - For every case where implementation revealed a design doc
     was wrong or incomplete, update the doc.
   - Pass through docs 01, 08, 09, 10, 11, 15 at minimum; others
     as needed.
   - Fix the "still open" items in doc 14 that were settled
     during implementation.

**Acceptance criteria**:
- End-to-end suite passes in CI.
- Reference deployment reachable over the internet, accepting
  tenant API calls with proper TLS.
- At least one workflow running real traffic for at least a
  week without incident.

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

- All 23 crates published at `0.1.0` or higher on crates.io.
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
- `docs/design/*.md` — design docs (authoritative);
  `13-conventions.md` is the one most-frequently-updated.
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
# API-breakage check against the previous release (or any baseline)
./scripts/check-api-breakage.sh v0.1.0

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