# Phase 8 sub-phase A — `philharmonic-api` skeleton (initial dispatch)

**Date:** 2026-04-28
**Slug:** `phase-8-sub-phase-a-skeleton`
**Round:** 01 (initial dispatch)
**Subagent:** `codex:codex-rescue`

## Motivation

Phase 7 Tier 1 landed end-to-end on 2026-04-27; the connector
layer is feature-complete with all four data-layer connectors
plus the original Phase-6 set published at 0.1.0. Phase 8 is
implementing `philharmonic-api`, the public HTTP API.

Phase 8 is large (30+ endpoints, full auth + authz, rate
limiting, audit, observability) and is split into nine
sub-phases A→I, each one Codex round. **This dispatch
implements sub-phase A: the skeleton.** No real auth, no real
authz, no real endpoint handlers. Just the plumbing that
sub-phases B–H replace pieces of.

Non-crypto task. The crypto-review-protocol approach gate has
already been approved (recorded in
[`docs/notes-to-humans/2026-04-28-0001-phase-8-decisions-confirmed.md`](../notes-to-humans/2026-04-28-0001-phase-8-decisions-confirmed.md))
on the basis that Phase 8 introduces no new crypto — it
consumes wave-A/B primitives. Sub-phase A doesn't touch crypto
at all; B/E/G will, with code-level review at each gate.

## References (read end-to-end before coding)

- [`docs/design/10-api-layer.md`](../design/10-api-layer.md) —
  authoritative API-layer spec. Sub-phase A implements the
  scaffolding for everything in this doc; later sub-phases
  fill in the bodies. Pay particular attention to:
  - §"Request routing" — the deployment-supplied tenant/scope
    resolution model that sub-phase A ships.
  - §"Authentication" §"Distinguishing authentication
    contexts" — defines the `AuthContext` enum that
    sub-phase A creates as types-and-placeholders. **Do not
    implement real authentication in this round.**
  - §"Hosting the workflow engine" — explains the crate is a
    library exposing a router constructor; deployment chooses
    the process layout. Sub-phase A's public surface
    matches that framing.
- [`ROADMAP.md` §Phase 8](../../ROADMAP.md) — task list and
  sub-phase A→I plan. Sub-phase A's scope per
  §"Sub-phase plan".
- [`docs/notes-to-humans/2026-04-27-0003-phase-8-design-and-decisions.md`](../notes-to-humans/2026-04-27-0003-phase-8-design-and-decisions.md)
  — design rationale, sub-phase boundaries.
- [`docs/notes-to-humans/2026-04-28-0001-phase-8-decisions-confirmed.md`](../notes-to-humans/2026-04-28-0001-phase-8-decisions-confirmed.md)
  — Yuka's confirmed decisions: A→I sub-phasing,
  testcontainers-MySQL test backend, crypto-approach
  approval, `RequestScopeResolver` trait shape.
- [`CONTRIBUTING.md`](../../CONTRIBUTING.md):
  - §4 git workflow (signed-off, signed, scripts only).
  - §5 + §5.1 + §5.3 cooldown rule.
  - §10.3 no panics in library `src/`.
  - §10.4 libraries take bytes, not file paths.
  - §10.9 HTTP client split — runtime crates use
    `reqwest`/`hyper`+`tokio`+rustls, **not** `ureq`. The API
    crate is runtime, so `axum` (which uses `hyper` + tokio
    + rustls under the hood) is the right stack.
  - §11 pre-landing checks.
- `philharmonic-policy` 0.1.0 — provides the `Tenant` entity
  type and `MintingAuthority` / `Principal` / etc. Sub-phase
  A only needs `Tenant` + a few placeholder references.
- `philharmonic-types` — `EntityId<E>`, `Uuid`, `UnixMillis`.

If anything in this prompt contradicts the docs above, the
docs win. Flag contradictions and stop.

## Crate state (starting point)

- `philharmonic-api` is a fresh 0.0.0 placeholder submodule:
  - `Cargo.toml` with empty `[dependencies]`.
  - `src/lib.rs` containing only `// philharmonic-api: placeholder`.
  - License files + CHANGELOG + README from the scaffolder.
- Workspace-internal `[patch.crates-io]` already redirects
  `philharmonic-api` to the local path.
- Crate stays at version `0.0.0` after this round —
  **publishable from sub-phase I, not before.** No publish
  in sub-phase A.

Target after this round: a working axum-based skeleton with
the trait surfaces sub-phases B–H plug into, plus enough
smoke-test coverage to demonstrate the middleware chain runs
correctly. No real auth, no real authz, no real endpoint
implementations.

## Decisions fixed upstream (do NOT deviate)

These come from
[`docs/notes-to-humans/2026-04-28-0001-phase-8-decisions-confirmed.md`](../notes-to-humans/2026-04-28-0001-phase-8-decisions-confirmed.md):

1. **HTTP framework: `axum`.** Latest cooldown-clear stable.
   No alternative frameworks.
2. **`RequestScopeResolver` trait shape — single async method
   returning an enum, plugged via `Arc<dyn …>`.** Concrete
   shape:
   ```rust
   #[async_trait::async_trait]
   pub trait RequestScopeResolver: Send + Sync + 'static {
       async fn resolve(
           &self,
           parts: &http::request::Parts,
       ) -> Result<RequestScope, ResolverError>;
   }

   #[derive(Debug, Clone)]
   pub enum RequestScope {
       Tenant(EntityId<Tenant>),
       Operator,
   }

   #[derive(Debug, thiserror::Error)]
   pub enum ResolverError {
       #[error("request does not carry tenant or operator scope")]
       Unscoped,
       #[error("scope-resolver internal error: {0}")]
       Internal(String),
   }
   ```
   Plug-in via `Arc<dyn RequestScopeResolver>` at builder
   time.
3. **No new crypto in Phase 8.** Sub-phase A doesn't touch
   crypto at all; later sub-phases consume wave-A/B
   primitives from `philharmonic-connector-common` /
   `philharmonic-policy`. Do not import or reference any
   crypto crate beyond what's already a transitive dep.
4. **Test substrate (later sub-phases): testcontainers-MySQL
   with `serial_test`'s `#[file_serial(docker)]`.**
   Sub-phase A doesn't touch the substrate yet — its smoke
   tests use axum's `TestServer` against the in-memory
   middleware chain. No testcontainers in this round.

## Scope

### In scope

1. **`Cargo.toml`**:
   - Bump `rust-version` to whatever's needed (1.89 minimum
     from the workspace; 1.88 default is fine if no newer
     features are needed).
   - **Runtime deps**:
     - `axum = "<verify-cooldown-clear>"` (likely 0.8.x)
     - `tower = "<verify>"` (compatible with axum's tower
       version)
     - `tower-http = { version = "<verify>", features =
       ["trace", "request-id"] }`
     - `tokio = { version = "1", features = ["rt-multi-thread",
       "macros", "signal"] }`
     - `tracing = "0.1"`
     - `tracing-subscriber = { version = "<verify>", features
       = ["env-filter", "json", "fmt"] }`
     - `serde = { version = "1", features = ["derive"] }`
     - `serde_json = "1"`
     - `thiserror = "2"`
     - `async-trait = "0.1"` (for the resolver trait)
     - `http = "<verify>"` (for `http::request::Parts`)
     - `philharmonic-types = "<verify-via-xtask>"`
       (currently 0.x — confirm)
     - `philharmonic-policy = "<verify>"` (currently 0.1.x
       — provides `Tenant`, `Principal`, `MintingAuthority`,
       `EntityId`, etc.)
   - **Dev-deps**:
     - `tokio = { version = "1", features = ["test-util",
       "macros", "rt-multi-thread"] }` (already in deps;
       extend features)
     - `axum-test = "<verify>"` OR `tower::ServiceExt` for
       sending requests directly. Pick whichever is cooldown-
       clear and idiomatic.
     - `serde_json = "1"` (already in deps; tests rely on it
       for assertions)
   - **No `[features]`** for sub-phase A. Later sub-phases may
     add e.g. a `test-util` feature for shared test
     scaffolding; for now the crate has no optional features.
   - **Cooldown rule**: every dep version added must pass the
     3-day cooldown via `./scripts/xtask.sh crates-io-versions
     -- <crate>` before committing `Cargo.toml`. Workspace-
     internal deps (`philharmonic-types`,
     `philharmonic-policy`) are exempt per
     [`CONTRIBUTING.md` §5.3](../../CONTRIBUTING.md#53-crate-version-cooldown).

2. **Module layout**:
   ```
   src/
   ├── lib.rs       — crate rustdoc + builder + Router constructor + re-exports
   ├── scope.rs     — RequestScope enum, RequestScopeResolver trait, ResolverError
   ├── context.rs   — RequestContext (per-request data attached by middleware)
   ├── auth.rs      — AuthContext enum (Principal | Ephemeral) — types only,
   │                  no actual authentication logic. Sub-phase B fills the body.
   ├── error.rs     — ApiError + ErrorEnvelope (wire JSON shape) + IntoResponse impl
   ├── middleware/
   │   ├── mod.rs
   │   ├── correlation_id.rs   — generate / propagate X-Correlation-Id
   │   ├── request_logging.rs  — structured log line per request
   │   ├── scope.rs            — calls RequestScopeResolver, attaches RequestScope
   │   ├── auth_placeholder.rs — TODO marker; sub-phase B replaces with real auth
   │   └── authz_placeholder.rs— TODO marker; sub-phase C replaces with real authz
   └── routes/
       ├── mod.rs
       └── meta.rs    — /v1/_meta/version, /v1/_meta/health (smoke-test endpoints)
   ```
   Each module file has a top-comment explaining what it
   does and which sub-phase fills in any TODO sections.

3. **Public surface (`lib.rs`)**:
   ```rust
   //! `philharmonic-api` — public HTTP API for Philharmonic.
   //! ...crate-root rustdoc covering: builder pattern,
   //! deployment-supplied trait surfaces, sub-phase A's role
   //! as scaffolding, what's not yet implemented.

   pub use auth::AuthContext;
   pub use context::RequestContext;
   pub use error::{ApiError, ErrorEnvelope, ErrorCode};
   pub use scope::{RequestScope, RequestScopeResolver, ResolverError};

   /// Builder for [`PhilharmonicApi`]. Constructs the axum
   /// router and middleware chain once all required trait
   /// implementations have been plugged in.
   ///
   /// In sub-phase A the only required dependency is the
   /// `RequestScopeResolver`. Sub-phases B–H add more
   /// (substrate store, executor client, lowerer, signing
   /// keys, etc.).
   pub struct PhilharmonicApiBuilder { /* private */ }

   impl PhilharmonicApiBuilder {
       pub fn new() -> Self;

       pub fn request_scope_resolver(
           self,
           resolver: std::sync::Arc<dyn RequestScopeResolver>,
       ) -> Self;

       pub fn build(self) -> Result<PhilharmonicApi, BuilderError>;
   }

   /// The fully-constructed API. Wraps an `axum::Router` so
   /// sub-phase A doesn't leak axum's generic-state surface;
   /// consumers get a ready-to-serve service.
   pub struct PhilharmonicApi {
       router: axum::Router,
   }

   impl PhilharmonicApi {
       pub fn into_router(self) -> axum::Router;
       pub fn into_make_service(self) -> axum::routing::IntoMakeService<axum::Router>;
   }

   #[derive(Debug, thiserror::Error)]
   pub enum BuilderError {
       #[error("missing required dependency: {0}")]
       MissingDependency(&'static str),
   }
   ```

4. **`scope.rs`** — types from §"Decisions fixed upstream"
   item 2 above, plus:
   ```rust
   pub use philharmonic_policy::Tenant;
   pub use philharmonic_types::EntityId;
   ```
   (verify these are the correct re-export paths). The
   `RequestScope` enum derives `Debug`, `Clone`. Avoid
   serde derives unless needed downstream — keep the
   surface minimal.

5. **`auth.rs`** — `AuthContext` enum exactly as doc 10
   §"Distinguishing authentication contexts" specifies:
   ```rust
   #[derive(Debug, Clone)]
   pub enum AuthContext {
       Principal {
           principal_id: EntityId<Principal>,
           tenant_id: EntityId<Tenant>,
       },
       Ephemeral {
           subject: String,
           tenant_id: EntityId<Tenant>,
           authority_id: EntityId<MintingAuthority>,
           permissions: Vec<String>,
           injected_claims: serde_json::Value,
           instance_scope: Option<EntityId<WorkflowInstance>>,
       },
   }
   ```
   For sub-phase A, no methods on `AuthContext` are
   required beyond derive-providers. The placeholder auth
   middleware constructs a default-ish variant; real
   construction lives in sub-phase B.

   If `WorkflowInstance` doesn't yet exist as an entity
   type in `philharmonic-policy`, use `Uuid` as a
   placeholder and document the TODO. Verify via grep
   first.

6. **`context.rs`** — what middleware attaches to each
   request:
   ```rust
   #[derive(Debug, Clone)]
   pub struct RequestContext {
       pub correlation_id: uuid::Uuid,
       pub started_at: std::time::Instant,
       pub scope: RequestScope,
       pub auth: Option<AuthContext>,
   }
   ```
   `auth` is `Option` because sub-phase A's placeholder
   auth middleware leaves it `None`; sub-phase B fills it.
   Extracted into handlers via axum's `Extension<RequestContext>`
   pattern.

   Add `uuid = { version = "<verify>", features = ["v4",
   "serde"] }` to `[dependencies]`.

7. **`error.rs`** — implements doc 11's structured error
   envelope. Wire shape (sub-phase A pins the bytes):
   ```json
   {
     "error": {
       "code": "<machine-readable enum>",
       "message": "<human-readable message>",
       "details": { /* optional, code-specific */ },
       "correlation_id": "<uuid>"
     }
   }
   ```
   Rust types:
   ```rust
   #[derive(Debug, Serialize, Deserialize)]
   pub struct ErrorEnvelope {
       pub error: ErrorBody,
   }

   #[derive(Debug, Serialize, Deserialize)]
   pub struct ErrorBody {
       pub code: ErrorCode,
       pub message: String,
       #[serde(skip_serializing_if = "Option::is_none")]
       pub details: Option<serde_json::Value>,
       pub correlation_id: uuid::Uuid,
   }

   #[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
   #[serde(rename_all = "snake_case")]
   pub enum ErrorCode {
       UnscopedRequest,        // RequestScopeResolver returned Unscoped
       InternalError,          // catch-all for internal errors
       NotFound,               // generic 404
       MethodNotAllowed,       // 405
       NotImplemented,         // 501 — sub-phase A returns this for stub endpoints
       // Future sub-phases extend the enum:
       // Unauthenticated, Forbidden, RateLimited, InvalidRequest, ...
   }

   #[derive(Debug, thiserror::Error)]
   pub enum ApiError {
       #[error("unscoped request")]
       Unscoped(#[from] ResolverError),
       #[error("internal error: {0}")]
       Internal(String),
       #[error("not implemented")]
       NotImplemented,
   }

   impl ApiError {
       pub fn code(&self) -> ErrorCode { ... }
       pub fn http_status(&self) -> StatusCode { ... }
   }

   impl axum::response::IntoResponse for ApiError {
       fn into_response(self) -> axum::response::Response {
           // Pull correlation_id from the request extension if
           // available; if not (rare — middleware should
           // always have set it), generate a new one and log
           // a warning.
           ...
       }
   }
   ```
   The error code enum starts narrow in sub-phase A; later
   sub-phases extend it (`Unauthenticated`, `Forbidden`,
   `RateLimited`, etc.). Extending an enum is additive —
   that's fine.

8. **Middleware** (each in its own file under `middleware/`):
   - `correlation_id.rs` — checks the request for
     `X-Correlation-Id`; if absent, generates a fresh v4
     uuid; attaches to the response. The id is injected
     into the `RequestContext` extension for downstream
     middleware/handlers.
   - `request_logging.rs` — emits a structured `tracing::info!`
     at request start (with method + uri + scope-pending) and
     at request end (with status + latency_us). Uses
     `tracing` spans so the correlation id propagates.
   - `scope.rs` — extracts the request `Parts`, calls
     `RequestScopeResolver::resolve(...)`, attaches the
     resulting `RequestScope` to the context. On
     `ResolverError`, returns an `ApiError::Unscoped` ⇒ 400.
   - `auth_placeholder.rs` — **NO-OP that documents itself**:
     leaves `RequestContext.auth = None`. A `tracing::debug!`
     line records "auth-placeholder: real auth lands in
     sub-phase B". Module-level rustdoc explains the TODO
     and points at sub-phase B.
   - `authz_placeholder.rs` — same shape: NO-OP, documented,
     points at sub-phase C.

   Middleware ordering in the chain:
   ```
   correlation_id → request_logging → scope_resolver →
       auth_placeholder → authz_placeholder → handler
   ```

9. **Routes** (sub-phase A only — smoke tests):
   - `GET /v1/_meta/version` — returns `{"version":
     env!("CARGO_PKG_VERSION")}`. No auth required (or:
     `auth_placeholder` runs but it's a no-op anyway).
   - `GET /v1/_meta/health` — returns `{"status": "ok"}`.
     Same auth treatment.

   Tenant-scoped routes (sub-phases D–H) are NOT in scope
   for A. The router has stub paths or zero handlers
   beyond the meta endpoints.

10. **Unit tests colocated with each module**:
    - `scope::tests` — `RequestScope` round-trip via debug
      formatter; nothing to actually unit-test on the trait
      itself (it's a trait).
    - `context::tests` — construction + correlation-id
      uniqueness check.
    - `error::tests` — every `ApiError` variant ⇒ correct
      `ErrorCode` ⇒ correct `StatusCode`. JSON
      round-trip of `ErrorEnvelope`. `IntoResponse`
      generates the expected JSON body + status code.

11. **Integration tests under `tests/`**:
    - `tests/middleware_chain.rs` — uses `axum::Router` with
      a stub `RequestScopeResolver` impl that returns
      `Tenant(EntityId::nil())` for one path,
      `Operator` for another, and `Err(Unscoped)` for a
      third. Asserts the middleware chain attaches the
      correct `RequestScope` and that errors return the
      structured envelope.
    - `tests/error_envelope.rs` — boots the router, hits
      `/v1/_meta/version`, asserts 200 with JSON shape.
      Hits a bogus path, asserts 404 with the structured
      error envelope (correlation_id must be present and a
      valid uuid).
    - `tests/correlation_id.rs` — sends a request with a
      caller-supplied `X-Correlation-Id`, asserts the
      response echoes it. Sends without; asserts the
      response carries a fresh uuid.

    Use `axum::Router`'s `oneshot` or `axum-test`. No
    external network, no testcontainers (sub-phase A
    doesn't touch substrate).

12. **Crate-root rustdoc on `src/lib.rs`**: density matching
    the recently-published Tier 1 sibling crates. Sections:
    - Architecture overview (this is the public API; lives
      in front of the workflow engine; deployment plugs in
      a `RequestScopeResolver` and consumes the resulting
      `axum::Router`).
    - Builder pattern + minimal example.
    - What sub-phase A includes vs what's still
      placeholder (auth, authz, real endpoint handlers).
    - Pointers to doc 10 + the sub-phase plan in ROADMAP.

13. **`CHANGELOG.md`** — add an `[Unreleased]` entry
    describing sub-phase A's scope: skeleton, scope
    resolution, error envelope, observability, placeholder
    auth/authz, smoke endpoints. Keep the file's
    `[0.0.0]` placeholder reservation untouched.

14. **`README.md`** — refresh with a short paragraph
    describing the crate's purpose + the builder pattern.

### Out of scope (flag; do NOT implement)

- **Real authentication.** Sub-phase B implements `pht_`
  lookup and COSE_Sign1 verification. Sub-phase A's
  `auth_placeholder` middleware leaves `auth = None`.
- **Real authorization.** Sub-phase C implements
  permission-atom evaluation. Sub-phase A's
  `authz_placeholder` is a no-op.
- **Any substantive endpoint handler.** No workflow CRUD,
  no endpoint-config CRUD, no principal/role CRUD, no
  token minting, no audit-log access. Just the meta smoke
  endpoints.
- **Substrate access.** Sub-phase A does not depend on
  `philharmonic-store` or `philharmonic-store-sqlx-mysql`.
  Don't add them to `Cargo.toml`.
- **WorkflowEngine integration.** Sub-phase D wires the
  engine in. A's stub handlers don't need it.
- **Rate limiting, audit recording, pagination cursors.**
  Sub-phase H.
- **TLS termination, ingress configuration.** Always a
  deployment concern, not a crate concern.
- **Any crypto code, key management, signing key loading.**
  Sub-phases B/E/G — and they consume existing wave-A/B
  primitives, never invent new crypto.
- **`cargo publish`, `git tag`, commit, push.** Claude
  handles those after review. Working tree stays dirty.
- **Workspace-root `Cargo.toml` edits.**
- **Bumping `version` past `0.0.0`.** Crate publishes from
  sub-phase I, not before.

## Workspace conventions (recap)

- Edition 2024, MSRV ≥ 1.88 (bump if a newer feature is
  needed; document why).
- License `Apache-2.0 OR MPL-2.0`.
- `thiserror` for library errors; no `anyhow` in
  non-test/non-build paths.
- **No panics in library `src/`** (CONTRIBUTING.md §10.3):
  no `.unwrap()` / `.expect()` / `panic!` /
  `unreachable!` / `todo!` / `unimplemented!` on reachable
  paths. Tests are exempt.
- **Library takes bytes, not file paths** (§10.4). Sub-phase
  A doesn't read any files at runtime.
- **No `unsafe`** in `src/`.
- **Rustdoc on every `pub` item.**
- HTTP client split (§10.9): the API crate is runtime →
  axum (which uses hyper + tokio + rustls) is correct.
  No `ureq` here.
- Use `./scripts/*.sh` wrappers (not raw cargo) — but tests
  run directly via cargo within the crate.

## Pre-landing

```sh
./scripts/pre-landing.sh philharmonic-api
```

Must pass green. No `--no-default-features` variant for
sub-phase A (no features defined yet).

## Git

You do NOT commit, push, branch, tag, or publish. Leave
the working tree dirty in the submodule (and in the parent
if any submodule pointer would change). Claude commits via
`./scripts/commit-all.sh` post-review.

Read-only git is fine (`log`, `diff`, `show`, `status`).

## Deliverables

1. Updated `Cargo.toml` with cooldown-checked deps (axum,
   tower, tower-http, tokio, tracing, tracing-subscriber,
   serde, serde_json, thiserror, async-trait, http,
   uuid, philharmonic-types, philharmonic-policy).
2. `src/lib.rs` with the public surface: builder,
   `PhilharmonicApi`, re-exports, crate-root rustdoc.
3. `src/{scope,context,auth,error}.rs` — types per
   §"In scope" #4–#7.
4. `src/middleware/{mod,correlation_id,request_logging,
   scope,auth_placeholder,authz_placeholder}.rs` per
   §"In scope" #8.
5. `src/routes/{mod,meta}.rs` per §"In scope" #9.
6. Unit tests colocated with each module per §"In scope"
   #10.
7. `tests/{middleware_chain,error_envelope,correlation_id}.rs`
   per §"In scope" #11.
8. Crate-root rustdoc per §"In scope" #12.
9. Refreshed `CHANGELOG.md` (`[Unreleased]` entry) and
   `README.md`.

Working tree: dirty. Do not commit.

## Structured output contract

1. **Summary** (3–6 sentences). What landed, what's
   placeholder vs real, where the seams for sub-phases B/C/D
   are clearly marked.
2. **Files touched** — every file added / modified.
3. **Verification results** — `pre-landing.sh` output, test
   counts (unit + integration), `cargo doc` clean (no broken
   intra-doc links).
4. **Residual risks / TODOs** — anything that didn't fit,
   anything that needs a Yuka call, any axum-version quirk.
   The placeholder auth/authz modules' TODO markers are
   expected — call them out so they're greppable.
5. **Git state** — `git -C philharmonic-api status --short`
   and `git -C . status --short`.
6. **Dep versions used** — exact `axum`, `tower`, `tower-http`,
   `tokio`, `tracing`, `tracing-subscriber`, `serde_json`,
   `thiserror`, `async-trait`, `http`, `uuid`,
   `philharmonic-types`, `philharmonic-policy`. Note whether
   each passed the 3-day cooldown.

## Default follow-through policy

- Carry through to pre-landing-green before returning. Do not
  return red.
- If pre-landing fails: fix and re-run.
- If a workspace-internal entity type doesn't exist as
  expected (e.g., `WorkflowInstance` not yet in
  `philharmonic-policy`): use a documented placeholder
  (`Uuid` instead of `EntityId<WorkflowInstance>`) and
  flag in residuals. Do not attempt to add the missing
  entity to `philharmonic-policy` from this round — that's
  out of scope.

## Completeness contract

- Every module in §"In scope" §"Module layout" exists with
  its specified content.
- Every test in §"Unit tests" + §"Integration tests" exists
  and runs green.
- `Cargo.toml` has the dep set above; no `philharmonic-store`
  / `philharmonic-store-sqlx-mysql` / `philharmonic-workflow`
  / `philharmonic-connector-*` deps yet (those land in later
  sub-phases).
- Crate-root rustdoc on `src/lib.rs` is non-empty and
  describes the architecture + builder pattern + sub-phase
  scope.
- Crate stays at version `0.0.0`.

## Verification loop

```sh
# Phase 0 — cooldown check for each new dep
./scripts/xtask.sh crates-io-versions -- axum
./scripts/xtask.sh crates-io-versions -- tower
./scripts/xtask.sh crates-io-versions -- tower-http
./scripts/xtask.sh crates-io-versions -- tokio
./scripts/xtask.sh crates-io-versions -- tracing
./scripts/xtask.sh crates-io-versions -- tracing-subscriber
./scripts/xtask.sh crates-io-versions -- serde
./scripts/xtask.sh crates-io-versions -- serde_json
./scripts/xtask.sh crates-io-versions -- thiserror
./scripts/xtask.sh crates-io-versions -- async-trait
./scripts/xtask.sh crates-io-versions -- http
./scripts/xtask.sh crates-io-versions -- uuid

# Final
./scripts/pre-landing.sh philharmonic-api
cargo doc -p philharmonic-api --no-deps
git -C philharmonic-api status --short
git -C . status --short
```

## Missing-context gating

- If `philharmonic-policy` doesn't expose `Tenant` /
  `Principal` / `MintingAuthority` / `WorkflowInstance` as
  the prompt assumes, grep the crate to find the correct
  paths, adapt, and document. STOP and flag if the entity
  surface is fundamentally different (we'd revise the
  AuthContext shape).
- If axum 0.8 (or whatever's latest cooldown-clear) has
  materially different middleware-chain APIs than 0.7's
  Tower-based shape: adapt within idiomatic patterns; if
  adaptation requires a different framework, STOP and
  flag.
- If the workspace's `[patch.crates-io]` doesn't redirect
  `philharmonic-policy` correctly: STOP and flag.
- If a dep is yanked or fails the 3-day cooldown: pin to
  the prior version and note.
- If any architecturally-significant surprise: STOP and
  flag.

## Action safety

- No `cargo publish`, no `git push`, no branch creation,
  no tags.
- No edits outside `philharmonic-api/` except `Cargo.lock`
  regeneration in the workspace root, which is fine and
  expected.
- No destructive ops.
- No new crypto code. If the round somehow surfaces a need
  to introduce crypto, STOP and flag — this round is
  explicitly non-crypto and the approach gate doesn't
  cover any new crypto introduction.

---

## Outcome

Pending — will be updated after Codex run.
