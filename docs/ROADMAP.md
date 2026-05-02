# Philharmonic Implementation Roadmap

**Audience**: Claude Code, working in the `philharmonic-workspace` repo.

**Purpose**: The authoritative implementation plan and milestone
archive for Philharmonic. V1 is complete through the first working
end-to-end deployment; active work now lives in the post-v1 dispatch
plan below.

**Current state** (2026-05-02):

- Design: complete.
- v1 implementation path: **complete through Phase 9**.
- Reference deployment: operational; a WebUI-created workflow has run
  end-to-end through API, mechanics worker, connector router/service, and
  an OpenAI-compatible upstream LLM via `llm_openai_compat`.
- Completed v1 milestones are archived concisely in §4. Historical
  implementation detail lives in dated `docs/codex-prompts/`,
  `docs/codex-reports/`, `docs/notes-to-humans/`, and
  `docs/crypto/{proposals,approvals}/` files.
- Active post-v1 work is tracked in §9: embedding datasets, remaining
  Phase 7 Tier 2/3 connector implementations, and WebUI/docs follow-through.

Consult the design documentation as the authoritative source for
architectural questions — **do not invent architectural decisions**. If a
design doc is wrong or incomplete, update the doc first, then implement.

---

## 1. Project context

### What Philharmonic is

A workflow orchestration system built as independent Rust crates.
JavaScript workflows run in stateless Boa runtimes; external I/O
goes through per-realm connector services with per-step encrypted
authorization; storage is append-only and content-addressed in a
MySQL-family database.

### Layered architecture

Seven layers, each with a clean dependency boundary upward:

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
  moved to [`/CONTRIBUTING.md`](../CONTRIBUTING.md) at the repo
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
first and update [`CONTRIBUTING.md §4`](../CONTRIBUTING.md#4-git-workflow).

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
[`CONTRIBUTING.md §11`](../CONTRIBUTING.md#11-pre-landing-checks).

**Follow append-only discipline.** The substrate has no `UPDATE`
or `DELETE` semantics. Entity state changes are new revisions;
soft-deletes are revisions with an `is_retired` scalar set true.
If you find yourself wanting to "just update a field," re-read
`02-design-principles.md` and `05-storage-substrate.md`.

**MSRV and edition.** Edition 2024. Workspace baseline
`rust-version = "1.88"`, set in every crate's own
`Cargo.toml`. Two crates declare 1.89 as a documented
exception: `inline-blob` and
`philharmonic-connector-impl-embed`. See
[`CONTRIBUTING.md §10.1`](../CONTRIBUTING.md#101-edition-and-msrv).

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

## 4. Completed v1 Milestone Archive

This section is historical archive, not active implementation
instruction. The full dispatch details live in the dated prompt,
report, approval, and notes files referenced below; this roadmap keeps
only the milestone summaries needed to understand current state. Active
post-v1 work starts in §9.

### Phase 0 — Workspace setup

**Status:** Done.

The parent workspace, submodules, root `Cargo.toml`, script wrappers,
Git hooks, CI, agent journals, and contributor docs were established.
The original helper-script scope grew into the current wrapper stack:
`setup`, `status`, `pull-all`, `commit-all`, `push-all`, lint/test
wrappers, publishing checks, audit/stat trailers, POSIX script tests,
and `xtask` Rust tooling for UUIDs, HTTP fetches, crate-version lookup,
calendar/status helpers, and related non-baseline work.

### Phase 1 — Extract `mechanics-config`

**Status:** Done 2026-04-21.

`mechanics-config 0.1.0`, `mechanics-core 0.3.0`, and `mechanics 0.3.0`
were published with signed release tags. The extraction moved Boa-free
HTTP endpoint/config schema types into `mechanics-config`; `mechanics-core`
keeps Boa runtime integration and re-exports compatibility paths. The
minor bump to `mechanics-core 0.3.0` was intentional after semver checks
identified the type-identity move as a breaking pre-1.0 change.

### Phase 2 — `philharmonic-policy`

**Status:** Done 2026-04-22.

`philharmonic-policy 0.1.0` shipped the tenant/policy entity model,
role evaluation, permission atom validation, SCK AES-256-GCM endpoint
config encryption, and `pht_` API token primitives. Crypto work passed
Yuka's Gate-1/Gate-2 review process; follow-up reviews fixed
cross-tenant role confusion, arbitrary permission strings, and two
zeroization gaps. The crate later published `0.2.0` during Phase 8 for
ephemeral API-token primitives.

### Phase 3 — `philharmonic-connector-common`

**Status:** Done 2026-04-22.

`philharmonic-connector-common 0.1.0` published shared connector wire
and error types: token claims, connector call context, realm/key wrapper
models, COSE wrapper types, and `ImplementationError`. `0.2.0` published
with Phase 5 to add the `iat` claim required by the composed connector
crypto flow.

### Phase 4 — `philharmonic-workflow`

**Status:** Done 2026-04-22.

`philharmonic-workflow 0.1.0` published workflow templates, instances,
step records, subject context, lowerer/executor traits, the workflow
engine, status transitions, terminal-state enforcement, and integration
tests over the substrate. A design-doc correction landed alongside for
first-step `Pending -> Failed` transitions.

### Phase 5 — Connector triangle

**Status:** Done 2026-04-23.

The connector crypto triangle shipped across two reviewed waves. Wave A
implemented COSE_Sign1 authorization token mint/verify with Ed25519,
`kid` lookup, expiry, realm, and payload-hash checks. Wave B added
ML-KEM-768 + X25519 hybrid KEM, HKDF-SHA256, AES-256-GCM,
COSE_Encrypt0 payload encryption, token/hash composition, and service
verification/decryption. Yuka's two-gate crypto review completed for both
waves. Published crates: `philharmonic-connector-common 0.2.0`,
`philharmonic-connector-client 0.1.0`,
`philharmonic-connector-service 0.1.0`, and
`philharmonic-connector-router 0.1.0`.

### Phase 6 — First implementations

**Status:** Done 2026-04-24.

The non-crypto implementation trait crate and the first two connector
implementations published: `philharmonic-connector-impl-api 0.1.0`,
`philharmonic-connector-impl-http-forward 0.1.0`, and
`philharmonic-connector-impl-llm-openai-compat 0.1.0`. The LLM
implementation covers OpenAI-compatible chat servers plus vLLM-native and
tool-call fallback structured-output dialects, with deterministic request
fixtures and optional real-service smokes.

### Phase 7 — Additional implementations (parallel-safe)

**Status:** Tier 1 done 2026-04-27; Tier 2/3 deferred to §9.

Tier 1 shipped data-layer implementations:
`philharmonic-connector-impl-sql-postgres 0.1.0`,
`philharmonic-connector-impl-sql-mysql 0.1.0`,
`philharmonic-connector-impl-vector-search 0.1.0`, and
`philharmonic-connector-impl-embed 0.1.0`. The embed crate pivoted from
`fastembed`/`ort` to pure-Rust `tract` + `tokenizers` for musl deployment;
`inline-blob 0.1.0` was added to embed multi-GB ONNX/external-data blobs in
large rodata sections. Tier 2/3 connector implementations (SMTP,
Anthropic, Gemini) are deferred to the post-v1 plan in §9.

### Phase 8 — `philharmonic-api`

**Status:** Done 2026-04-28.

`philharmonic-api 0.1.0` and `philharmonic-policy 0.2.0` published after
all API sub-phases landed: axum skeleton, scope resolver, auth/authz,
workflow management, endpoint config management, principal/role/minting
CRUD, token minting, audit log, rate limiting, tenant-admin/operator
routes, pagination, observability, and error envelopes. Crypto-sensitive
sub-phases B0, E, and G passed the required review gates. The API crate
closed with broad integration coverage and crate-level publication.

### Phase 9 — Integration and reference deployment

**Status:** Done 2026-05-02.

The published libraries were wired into executable processes and a working
reference deployment. Three in-tree bin crates now exist:
`mechanics-worker`, `philharmonic-connector`, and `philharmonic-api`.
Shared server infrastructure, optional TLS, config/drop-in loading, SIGHUP
reloads, install support, musl builds, Docker compose, real lowerer,
real executor, WebUI embedding, and e2e tests landed. A reference
deployment successfully exercised a WebUI-created workflow through the
full path: API server -> lowerer (COSE_Sign1 + COSE_Encrypt0) -> mechanics
worker -> connector router -> connector service -> `llm_openai_compat` ->
upstream LLM.

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

- All 25 crate names present on crates.io: substantive
  implementations at `0.1.0` or higher, plus the three
  Phase 7 Tier 2/3 placeholder names
  (`philharmonic-connector-impl-email-smtp`,
  `-llm-anthropic`, `-llm-gemini`) at `0.0.x` reserving
  namespace until they ship substantively.
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

## 9. Post-v1 dispatch plan

Phase 9 is complete (2026-05-02) and the reference deployment
is operational. The work below is post-v1 / post-GW: it does
not block deployment and is sequenced for the next development
cycle. Each numbered item is one Codex dispatch with its own
archived prompt under `docs/codex-prompts/` (see
`.claude/skills/codex-prompt-archive/SKILL.md`). The single
`(Gate 1)` item is **not** a Codex dispatch — Claude drafts the
proposal, Yuka reviews per the two-gate crypto-review protocol
(§5).

Total: **11 Codex dispatches plus 1 Gate-1 proposal.**

### A. Embedding datasets (6 dispatches + 1 Gate-1)

Authoritative design: [`docs/design/16-embedding-datasets.md`](design/16-embedding-datasets.md).
HUMANS.md errata (deterministic CBOR storage, `LONGBLOB`
migration on startup, friendly UI) are folded into the design.

- **(Gate 1)** Lowerer ephemeral support — Claude writes a
  Gate-1 proposal under `docs/crypto/proposals/` choosing
  between Approach A (`LowerScope` enum, public-trait change
  to `philharmonic-workflow`) and Approach B (synthesized
  non-persisted instance UUID, no trait change). Yuka reviews
  and signs off. **D4 and D5 below cannot start before Gate 1
  clears** because they touch COSE_Sign1 claim semantics and
  COSE_Encrypt0 AAD inputs.
- **D1** Substrate `MEDIUMBLOB → LONGBLOB` migration in
  `philharmonic-store-sqlx-mysql`, applied idempotently on
  startup. Independent of Gate 1.
- **D2** `mechanics-core`: optional `MechanicsJob.run_timeout`
  override, honored by `MechanicsPool`. Backward-compatible
  `Option<Duration>`. Independent of Gate 1.
- **D3** Embedding-datasets backend:
  - `EmbeddingDataset` entity + scalar/content slots in
    `philharmonic-policy`.
  - Permission atoms (`embed_dataset:create|read|update|retire`).
  - API CRUD endpoints + source-items + corpus endpoints in
    `philharmonic-api`.
  - `WorkflowTemplate.data_config` content slot + API
    validation.
  - Workflow engine `data` assembly in `execute_step`
    (`philharmonic-workflow`).

  Cross-crate but cohesive feature surface; one dispatch.
  Independent of Gate 1. If Codex hits scope limits, split
  into "policy entity + atoms" round-01 and "API + workflow
  data assembly" round-02.
- **D4** Lowerer ephemeral support per Gate-1 outcome.
  Touches `philharmonic-workflow` (Approach A) or only the
  API server lowerer (Approach B). **Gated on Gate 1.**
- **D5** Ephemeral embed job: built-in JS embed script
  (Codex-authored, compiled into the API binary as a static
  string) plus the background tokio task in
  `philharmonic-api-server` that lowers the embed endpoint,
  dispatches the mechanics job, and appends `Ready` /
  `Failed` revisions. **Gated on D4** (and therefore on
  Gate 1).
- **D6** Embedding-datasets WebUI: structured table editor
  for source items, Import modal for CSV/JSON bulk import,
  collapsed-by-default vector view in the corpus tab,
  i18n for `en.ts` and `ja.ts`. Depends on D3's API
  endpoints; can run in parallel with D4/D5. Per the
  HUMANS.md erratum, **no persistent raw-JSON view of the
  dataset itself**.

### B. Phase 7 Tier 2/3 connector implementations (3 dispatches)

Deferred post-Golden-Week 2026 per Phase 7 plan above. Each is
one substantive crate going from `0.0.x` placeholder to
`0.1.0` substantive implementation. None of these touch the
crypto path; the connector-service framework already validates
tokens and decrypts payloads — implementations only need to
implement the `Implementation` trait.

- **D7** `philharmonic-connector-impl-email-smtp` (Tier 2).
- **D8** `philharmonic-connector-impl-llm-anthropic` (Tier 3).
- **D9** `philharmonic-connector-impl-llm-gemini` (Tier 3).

Independent of one another and of section A; safe to run in
parallel.

### C. WebUI infrastructure and docs (2 dispatches)

- **D10** Add a maintained code-editor dependency
  (CodeMirror 6 or equivalent — must be FLOSS, actively
  maintained, no large native deps) to the WebUI for JSON /
  JavaScript editing. Retrofit existing JSON / JS editors
  (endpoint configs, workflow templates, etc.) to use it.
  Per HUMANS.md "Code editor" item. Useful prerequisite for
  D6's payload editor; can ship before D6.
- **D11** Workflow authoring guide rewrite (English).
  Codex re-reads the design docs and connector-architecture
  spec, then rewrites
  [`docs/guide/workflow-authoring.md`](guide/workflow-authoring.md)
  from scratch to reflect current implementation reality
  (per HUMANS.md "Update the workflow authoring guide").
  The Japanese mirror in
  [`docs-jp/ワークフロー作成ガイド.md`](../docs-jp/) is
  **not** a Codex dispatch — `docs-jp/README.md` reserves
  that submodule to Claude Code. Claude regenerates the JP
  guide after D11 lands.

### Suggested sequencing

1. **Now-ish** (no gates, independent, small): D1, D2, D10.
2. **Gate 1** — Claude drafts proposal; Yuka signs off.
3. **Embedding datasets feature**: D3 → D4 → D5; D6 in
   parallel after D3.
4. **Post-GW**: D7 / D8 / D9 in parallel (one crate each).
5. **Anytime**: D11 (independent of everything else).

### Dispatch discipline reminder

Per `.claude/skills/codex-prompt-archive/SKILL.md` and
[`CONTRIBUTING.md §15.2`](../CONTRIBUTING.md#152-codex-prompt-archive):
every Codex prompt is archived to
`docs/codex-prompts/YYYY-MM-DD-NNNN-<slug>.md` and committed
**before** the spawn. Each item above corresponds to one
archived prompt; multi-round work shares the daily-sequence
`NNNN` and increments the trailing `-NN` per the file-naming
convention.

---

## 10. Project-level files (current inventory)

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

## 11. Quick reference — command shortcuts

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

End of roadmap. V1 phases are archived in §4; begin current work
from the post-v1 dispatch plan in §9. Consult design docs
liberally. Flag anything crypto-adjacent for Yuka's review.
