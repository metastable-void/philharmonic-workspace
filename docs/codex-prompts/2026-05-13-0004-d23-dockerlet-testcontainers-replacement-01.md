# D23 — `dockerlet 0.1.0` + workspace testcontainers replacement

**Date:** 2026-05-13 (JST)
**Dispatch:** D23
**Crate:** `dockerlet` (new; `0.0.0` placeholder → `0.1.0`
substantive)
**Driver:** ROADMAP §3.J production-security cleanup
ordering — D23 lands the in-tree minimal testcontainers
replacement that drops the bollard-driven
`rustls-native-certs` pull from the workspace dev-dep tree.
**Round:** 01 (single-round dispatch covering the full
sweep: dockerlet library + 6 consumer migrations + workspace
patch + `deny.toml` wrapper removal).

## Context (read these in this order)

1. `CONTRIBUTING.md` — workspace conventions.
   - **§3.1** `[profile.release]` per-setting rationale
     (the canonical block to use in `dockerlet/Cargo.toml`
     verbatim — already present from the scaffolding
     commit; leave it intact).
   - **§5 Script wrappers** — every cargo invocation goes
     through the appropriate wrapper.
   - **§6 Shell scripts** — POSIX sh only.
   - **§10.3 Panics** — no `.unwrap()` / `.expect()` /
     `panic!()` on reachable paths in `dockerlet/src/`.
     Tests are exempt; src/ is not.
   - **§10.9 HTTP-client stack** — workspace is `rustls` +
     `aws-lc-rs` + `webpki-roots` only. dockerlet does not
     need to touch HTTP at the application layer (it talks
     to the local Docker daemon over a Unix socket), but
     ensure any HTTP-client dep it transitively pulls
     stays inside the existing deny.toml posture.
2. `docs/ROADMAP.md` **§3.J D23** — the canonical D23
   spec including the API surface the workspace's tests
   actually use, the bollard feature shape, and the
   concurrency-limit knob formula
   `min(4, available_parallelism / 4)`.
3. `deny.toml` — current `[bans]` deny list with the
   `rustls-native-certs` `wrappers = ["bollard"]` entry.
   D23's acceptance criterion is removing that wrapper
   (the entry becomes a no-wrapper full ban, matching
   `native-tls` / `rustls-platform-verifier`).
4. `dockerlet/Cargo.toml` + `dockerlet/src/lib.rs` —
   0.0.0 placeholder scaffolding (committed at `9fb21f5`).
   Replace the placeholder content with the 0.1.0
   implementation.
5. The six consumer test files for the migration:
   - `philharmonic-api/tests/e2e_mysql.rs`
   - `philharmonic-api/tests/e2e_full_pipeline.rs`
   - `philharmonic-policy/tests/permission_mysql.rs`
   - `philharmonic-workflow/tests/engine_mysql.rs`
   - `philharmonic-store-sqlx-mysql/tests/integration.rs`
   - `philharmonic-connector-impl-sql-mysql/tests/common/mod.rs`
   - `philharmonic-connector-impl-sql-postgres/tests/common/mod.rs`
   - …and any other `tests/**` file importing
     `testcontainers` / `testcontainers_modules` —
     grep the workspace to find them all before
     committing to a migration pass.

## Goal

Ship `dockerlet 0.1.0` as a thin, minimal-feature wrapper
over `bollard` for the workspace's integration-test
fixtures, migrate all six (or however many) consumer test
files away from `testcontainers` / `testcontainers-modules`,
update the workspace root `[patch.crates-io]` to override
`dockerlet` to the local path, and remove the
`rustls-native-certs` wrapper from `deny.toml`. After
this dispatch:

- `cargo tree --workspace --invert rustls-native-certs -e all --target all`
  prints **nothing**.
- `deny.toml` `[bans] deny` lists `rustls-native-certs`
  as a no-wrapper full ban.
- The six consumer test suites still pass (ignored MySQL
  + Postgres testcontainer tests run identically against
  the new container helper).
- `./scripts/pre-landing.sh` ends with
  `=== pre-landing: all checks passed ===`.

## Non-goals

- **Not a full `testcontainers` clone.** dockerlet targets
  exactly the workspace's surface area today. If a future
  consumer needs e.g. exec, copy-files-in, network alias,
  Compose support, etc., add it then; v1 ships the
  minimum the existing tests use.
- **Not a publish.** Yuka publishes `dockerlet 0.1.0`
  manually after reviewing the diff. Codex does not
  invoke `publish-crate.sh`.
- **No DNSSEC features**, no `rustls-platform-verifier`,
  no `rustls-native-certs`, no `native-tls`, no `ring`
  reintroduction anywhere in the dev-dep tree.
- **No Windows-host support.** The workspace targets
  POSIX hosts (`linux` / `darwin` / `freebsd` /
  `openbsd` / `netbsd`), so dockerlet can assume the
  Docker daemon Unix socket exists at
  `/var/run/docker.sock` (or `DOCKER_HOST` env override
  if set). No named-pipe transport.
- **No registry-side TLS handling.** The Docker daemon
  pulls images; dockerlet only talks to the daemon over
  the local Unix socket. No TLS at the dockerlet layer.
- **No DNSSEC / proxy / mTLS / cert pinning** at the
  bollard layer. The bollard feature set is deliberately
  narrow.

## Concrete tasks

### 1. Implement `dockerlet 0.1.0`

#### 1.1 `dockerlet/Cargo.toml`

Bump version `0.0.0` → `0.1.0`. Add dependencies (use
`default-features = false` everywhere; enumerate features
explicitly):

```toml
[dependencies]
bollard = { version = "0.20", default-features = false }  # see §1.2 for the exact feature audit
bytes = "1"
futures-util = { version = "0.3", default-features = false }
thiserror = "2"
tokio = { version = "1", default-features = false, features = ["rt", "sync", "time"] }
tracing = "0.1"
```

(Sanity-check each version against `crates-io-versions`
before locking. If `bollard 0.20` doesn't have the
narrowest needed feature set, surface and propose the
alternative.)

The `[profile.release]` block already in
`dockerlet/Cargo.toml` from the scaffolding commit
(canonical `opt-level = 3` / `lto = true` / `strip = true`
/ `codegen-units = 1` / `panic = "abort"` /
`overflow-checks = true`) stays unchanged.

#### 1.2 bollard feature audit

Bollard's feature graph entangles `home` / `ssl_providerless`
/ `rustls-native-certs` in ways the workspace explicitly
does not want pulled. Pick the **smallest** bollard
feature set that supports:

- Unix-socket transport (local Docker daemon).
- Container create / start / inspect / wait / stop /
  remove.
- Image pull (the daemon does it; dockerlet just asks).
- Log streaming (for the `WaitFor::message_on_stderr(...)`
  / `WaitFor::message_on_stdout(...)` readiness probe).
- Port mapping inspection (to read the host-side port the
  daemon assigned to an exposed container port).

Forbidden features (must NOT be enabled, even transitively
via feature unification):

- `home` (reads `~/.docker/config.json`).
- `ssl` / `ssl_providerless` (pulls rustls-native-certs).
- `rustls-native-certs`.
- `native-tls`.
- `aws-lc-rs` if it forces `ssl_providerless` (verify
  against bollard 0.20's feature graph; if the only path
  to aws-lc-rs is via `ssl_providerless`, leave the
  feature off — no TLS at all is fine for the Unix-socket
  case).

Verify the audit with
`cargo tree -p dockerlet --invert rustls-native-certs -e all --target all`
returning empty.

#### 1.3 Public surface

The library exposes (names are suggestions; pick what
reads cleanly while staying close to testcontainers'
shape for consumer migration):

```rust
pub struct GenericImage { … }

impl GenericImage {
    pub fn new(repo: impl Into<String>, tag: impl Into<String>) -> Self;
    pub fn with_exposed_port(self, port: ContainerPort) -> Self;
    pub fn with_wait_for(self, wait: WaitFor) -> Self;
    pub fn with_env_var(self, key: impl Into<String>, value: impl Into<String>) -> Self;
    pub fn with_startup_timeout(self, timeout: Duration) -> Self;
    pub async fn start(self) -> Result<Container, Error>;
}

pub struct ContainerPort(/* private */);
impl ContainerPort {
    pub fn tcp(port: u16) -> Self;
    // udp variant optional; consumers only use tcp today.
}
// Use the IntoContainerPort extension trait pattern so that
// `5432.tcp()` works the way the existing testcontainers
// callers expect.
pub trait IntoContainerPort {
    fn tcp(self) -> ContainerPort;
}
impl IntoContainerPort for u16 { … }

pub enum WaitFor {
    /// Wait for a substring to appear on the container's
    /// stderr stream.
    MessageOnStderr(String),
    /// Wait for a substring to appear on the container's
    /// stdout stream.
    MessageOnStdout(String),
    /// Wait for the daemon to mark the container as
    /// running (cheap but coarse).
    Running,
    /// Sleep for a fixed duration after the container
    /// starts. Discouraged — use a real readiness probe.
    Duration(Duration),
}

impl WaitFor {
    pub fn message_on_stderr(msg: impl Into<String>) -> Self;
    pub fn message_on_stdout(msg: impl Into<String>) -> Self;
}

pub struct Container { /* holds bollard ID + concurrency slot */ }

impl Container {
    pub async fn get_host(&self) -> Result<String, Error>;
    pub async fn get_host_port_ipv4(&self, container_port: ContainerPort) -> Result<u16, Error>;
}

impl Drop for Container {
    fn drop(&mut self) {
        // Best-effort: tell the daemon to stop + remove.
        // Synchronously block on a small tokio runtime
        // here is acceptable since Drop can't .await; do
        // not panic on cleanup failure (log via tracing
        // instead).
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("docker daemon unavailable: {0}")]
    DaemonUnavailable(String),
    #[error("startup timed out after {0:?}")]
    StartupTimeout(Duration),
    #[error("readiness probe failed: {0}")]
    ReadinessFailed(String),
    #[error("internal bollard error: {0}")]
    Bollard(String),
    #[error("internal: {0}")]
    Internal(String),
}
```

(The `IntoContainerPort` trait + `5432.tcp()` extension
pattern is the way the existing testcontainers callers
spell it — preserving the shape minimises consumer-side
diff.)

#### 1.4 Concurrency-limit knob

ROADMAP §3.J specifies
`min(4, available_parallelism / 4)` as the
testcontainer-test concurrency cap. Implement as a
**cross-process file-lock semaphore** so the cap applies
across cargo's per-test-binary process fan-out, not just
within a single test binary:

```rust
fn concurrency_limit() -> usize {
    let parallelism = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    std::cmp::max(1, std::cmp::min(4, parallelism / 4))
}
```

Implementation sketch:

- Pick a lockdir under `/tmp/dockerlet-locks-${uid}/`
  (per-uid, follows the workspace tmpfs pattern from
  `scripts/lib/cargo-target-dir.sh`). Create it on first
  use; idempotent.
- Lay down N empty lockfiles `slot-0` through
  `slot-{N-1}` where N is `concurrency_limit()`.
- `Container::start()` calls a private
  `ConcurrencySlot::acquire()` that loops over the N
  files trying non-blocking exclusive `flock` (via
  `tokio::fs::OpenOptions` + a small async-friendly
  `flock` shim, or the `fs2` crate if no in-tree wrapper
  exists). On success, the slot file handle is held until
  the `Container` is dropped.
- The N value is computed once via `LazyLock<usize>` —
  recomputing per-call is wasteful and racy.
- If all N slots are taken, `tokio::time::sleep(50ms)` +
  retry. Yield, don't busy-loop.

If `fs2` (or equivalent) is the cleanest path for `flock`
on a `std::fs::File`, add it as a dep — but **verify it
doesn't pull `winapi` / `windows-sys` on Linux** (those
are gated; should be fine). If `fs2` is unavailable or
deprecated, fall back to a direct `libc::flock` call
inside an `unsafe` block — but in that case, document
the `unsafe` rationale clearly in a comment and keep the
unsafe surface as narrow as possible (CONTRIBUTING.md
§10.3's panic-free rule doesn't forbid `unsafe` but the
workspace prefers `safe` wherever practical).

The concurrency limit applies to **container starts**,
not to all dockerlet calls. Once a container has its
slot, the rest of the test runs unimpeded.

#### 1.5 Implementation modules

Suggested layout under `dockerlet/src/`:

- `lib.rs` — re-exports the public surface; module
  declarations; crate-level rustdoc (the placeholder
  already has a draft — expand it for 0.1.0).
- `image.rs` — `GenericImage` builder.
- `container.rs` — `Container` runtime handle + `Drop`
  cleanup.
- `wait.rs` — `WaitFor` + the log-stream probe logic.
- `port.rs` — `ContainerPort` + `IntoContainerPort`.
- `error.rs` — `Error` enum.
- `concurrency.rs` — slot acquisition + the formula.
- `client.rs` — `bollard::Docker` setup with Unix-socket
  transport; the workspace's single bollard-config home.
- `tests.rs` — unit tests where they fit (e.g. the
  concurrency formula, `IntoContainerPort` impls). The
  end-to-end "actually start a real container" tests
  live as `#[ignore]`'d integration tests under
  `dockerlet/tests/integration.rs` — they require Docker
  on the host and aren't part of the default
  `cargo test` run; the same pattern the consumer crates
  already use.

#### 1.6 `dockerlet/CHANGELOG.md`

Add a `[0.1.0] - 2026-05-13` entry:

- "Initial substantive release. Provides `GenericImage`
  builder + `Container` handle + `WaitFor` readiness
  probe + cross-process concurrency-limit semaphore for
  workspace integration tests. Talks to the local Docker
  daemon over the Unix socket; no TLS at the dockerlet
  layer. Bollard feature set deliberately narrow — no
  `home`, no `ssl_providerless`, no `rustls-native-certs`."

### 2. Workspace `[patch.crates-io]` for in-workspace builds

In the **workspace-root** `Cargo.toml`, add to the
existing `[patch.crates-io]` block:

```toml
dockerlet = { path = "dockerlet" }
```

This routes the consumer crates' `dockerlet = "0.1"`
dev-deps to the local path during in-workspace builds.
Standalone clones of the consumer submodules resolve
`dockerlet = "0.1"` from crates.io once Yuka publishes
0.1.0.

### 3. Consumer migrations

Migrate each consumer's test file from
`testcontainers` / `testcontainers-modules` to dockerlet.
Update each consumer's `Cargo.toml` dev-deps: remove
`testcontainers` + `testcontainers-modules`, add
`dockerlet = "0.1"`.

The known consumers:

- `philharmonic-api/tests/e2e_mysql.rs`
- `philharmonic-api/tests/e2e_full_pipeline.rs`
- `philharmonic-policy/tests/permission_mysql.rs`
- `philharmonic-workflow/tests/engine_mysql.rs`
- `philharmonic-store-sqlx-mysql/tests/integration.rs`
- `philharmonic-connector-impl-sql-mysql/tests/common/mod.rs`
- `philharmonic-connector-impl-sql-postgres/tests/common/mod.rs`

**Grep the workspace before committing the migration
pass** to find any `tests/**` file that imports
`testcontainers` or `testcontainers_modules` and is not
on the above list. Migrate those too.

#### 3.1 The typed MySQL / Postgres convenience

`testcontainers-modules`' `Mysql::default()` carries a
specific config (user `root`, no password, port `3306`).
Replace with explicit `GenericImage::new("mysql", "8")
.with_exposed_port(3306.tcp())
.with_env_var("MYSQL_ALLOW_EMPTY_PASSWORD", "yes")
.with_wait_for(WaitFor::message_on_stderr(
"ready for connections"))` (or whichever MySQL log line
the consumers' real readiness expects — check the
existing tests' patterns).

The Postgres setup in
`philharmonic-connector-impl-sql-postgres/tests/common/mod.rs`
already uses `GenericImage::new("postgres", "16-alpine")`
directly — migrate that to dockerlet's `GenericImage`
with minimal call-site diff.

#### 3.2 Per-consumer Cargo.toml change

Each affected consumer's `Cargo.toml`:

```toml
# remove:
testcontainers = { version = "0.27", default-features = false, features = ["aws-lc-rs"] }
testcontainers-modules = { version = "0.15", default-features = false, features = ["aws-lc-rs", "mysql"] }

# add:
dockerlet = "0.1"
```

#### 3.3 Crate-version bumps

Each consumer crate whose `Cargo.toml` is touched gets a
patch-version bump (e.g. `philharmonic-api` 0.1.8 →
0.1.9). Add a `CHANGELOG.md` entry for each:

- "Dev: migrate integration-test fixtures from
  `testcontainers` / `testcontainers-modules` to
  `dockerlet 0.1`. No public-API change; runtime behaviour
  unchanged."

### 4. `deny.toml` cleanup

After the migration, `rustls-native-certs` is no longer
pulled by any path. Update `deny.toml`:

```toml
# replace this:
{ crate = "rustls-native-certs", wrappers = ["bollard"] },

# with this:
"rustls-native-certs",
```

The surrounding comment block about "one remaining
upstream-blocked path" should be updated to note D23
landed and the wrapper is no longer needed.

Verify with `./scripts/cargo-deny.sh` — must end with
`bans ok`.

## Hard requirements

- **No git operations.** Do not run any `scripts/*.sh`
  git wrapper. The codex-guard in `commit-all.sh` aborts
  under any Codex ancestor process. Claude commits +
  pushes after the dispatch.
- **No `publish-crate.sh`.** Yuka publishes dockerlet
  0.1.0 manually after diff review.
- **No `unsafe`** if it can be avoided cleanly. If the
  flock implementation requires `unsafe`, narrow it to a
  single function with a clear safety comment.
- **No panics in `dockerlet/src/`** on reachable paths
  (CONTRIBUTING.md §10.3). Tests are exempt; src/ is
  not. Drop impls log via `tracing::warn!`, never panic.
- **No `ring` reintroduction.** Verify with
  `cargo tree --workspace --invert ring -e all --target all`
  after each significant Cargo.toml change. The only
  acceptable surface is `quinn-proto` (which has its own
  wrapper in deny.toml from the prior cleanup pass).
- **`bollard` features audited.** The dockerlet
  `Cargo.toml` must list the bollard feature set
  explicitly with `default-features = false`. No
  defaults. Document the audit in the CHANGELOG (which
  features were chosen and why).
- **POSIX sh only** for any shell snippets in the
  archive's Outcome section (see §6).
- **No changes to release-binary runtime trees.** The
  workspace's published bins (`mechanics-worker`,
  `philharmonic-api-server`, `philharmonic-connector-bin`)
  must not pull dockerlet in any release-features cargo
  build. dockerlet is dev-only.

## Verification before reporting `task_complete`

Report `task_complete` only after **all** of:

1. `cargo check -p dockerlet` clean.
2. `cargo test -p dockerlet` all-pass (unit tests; the
   `#[ignore]`'d integration tests require Docker and
   aren't part of the default pass).
3. `cargo tree -p dockerlet -e all --invert rustls-native-certs --target all`
   → empty.
4. `cargo tree --workspace --invert rustls-native-certs -e all --target all`
   → empty after the consumer migrations land.
5. `cargo tree --workspace --invert ring -e all --target all`
   → only `quinn-proto` path remains (unchanged from
   pre-dispatch state).
6. `./scripts/cargo-deny.sh` → `bans ok`. `deny.toml`
   change verified.
7. `./scripts/pre-landing.sh` → exit 0 with
   `=== pre-landing: all checks passed ===`. Note this
   will include the `--ignored` testcontainer phases on
   the migrated consumer crates; those tests must pass
   against the new dockerlet helper.

If any check fails, surface the failure in the report
rather than silently continuing — including any case
where the verification expectation itself proved stale
(like D25's `cargo tree --invert hickory-proto` finding
the fixed-version still present).

## Tests

Adapt the existing consumer tests verbatim — the goal is
behaviour preservation, not test expansion. New tests in
`dockerlet/src/tests.rs` for the unit-level concerns
(`concurrency_limit()` formula, `IntoContainerPort` impls,
`WaitFor` constructors). One `#[ignore]`'d integration
test in `dockerlet/tests/integration.rs` that actually
spins up a public image (e.g. `hello-world` or `alpine`)
to validate the end-to-end Docker daemon interaction —
this stays `#[ignore]`'d to keep the default cargo-test
pass Docker-independent, matching the consumer crates'
existing pattern.

## Outcome

**Completed 2026-05-13** as a Codex + Claude-co-implemented
dispatch. Codex shipped the bulk of `dockerlet 0.1.0` plus
the six consumer migrations and the `deny.toml` cleanup;
Claude post-Codex addressed three real bugs Codex's pass
left behind, pivoted the consumer test pattern from
single-container-per-test to a warm-container-per-binary
model on Yuka's directive, and added a fail-fast path in
`dockerlet`'s readiness probe.

### Codex's contribution (single run)

Codex session: `019e2105-...` (job `bjctv0bn6`).

- `dockerlet 0.1.0`: `GenericImage` builder + `Container`
  handle + `WaitFor` readiness + `ContainerPort` +
  `IntoContainerPort` + concurrency-limit semaphore via
  `flock(2)`-on-`/tmp/dockerlet-locks-$uid/slot-N` with
  N = `min(4, available_parallelism / 4)`.
- Bollard feature audit landed `default-features = false,
  features = ["pipe"]` — no `home`, no `ssl_providerless`,
  no `rustls-native-certs`, no `ring` reintroduction.
- Six consumer migrations from
  `testcontainers` / `testcontainers-modules` to
  `dockerlet 0.1`:
  `philharmonic-api/tests/{e2e_mysql, e2e_full_pipeline}.rs`,
  `philharmonic-policy/tests/permission_mysql.rs`,
  `philharmonic-workflow/tests/engine_mysql.rs`,
  `philharmonic-store-sqlx-mysql/tests/integration.rs`,
  `philharmonic-connector-impl-sql-mysql/tests/common/mod.rs`,
  `philharmonic-connector-impl-sql-postgres/tests/common/mod.rs`.
- Each consumer Cargo.toml `testcontainers` /
  `testcontainers-modules` dev-deps dropped; `dockerlet =
  "0.1"` added; consumer patch-bumped + CHANGELOG entry.
- `deny.toml` `rustls-native-certs` switched from
  wrapper-allowed to no-wrapper full ban.

### Claude post-Codex (this turn)

1. **`tokio` runtime IO**: dockerlet's Drop runtime was
   built with `enable_time()` only; bollard's Unix-socket
   transport needs IO, panicking with "A Tokio 1.x context
   was found, but IO is disabled." Fixed: added
   `enable_io()` to the Drop runtime + bumped
   `dockerlet/Cargo.toml`'s tokio features to include
   `net` + `io-util`.
2. **`sql-postgres` `multi_thread` flavor**: pre-existing
   tests use `#[tokio::test(flavor = "multi_thread")]` but
   the crate's dev-dep tokio features didn't include
   `rt-multi-thread` — Codex's
   `testcontainers` → `dockerlet` swap inadvertently
   removed the feature unification path that previously
   pulled it in. Fixed: added `rt-multi-thread` to
   sql-postgres's dev-dep tokio.
3. **Warm-container pivot** (Yuka, 2026-05-13): the per-
   test container model was wasteful — 28 tests in
   `philharmonic-store-sqlx-mysql` each started a fresh
   MySQL container (~30s wall clock per start). Pivoted
   to a per-binary warm container shared across all
   tests, with per-test isolation by unique database
   name. Each consumer's setup() now:
   - acquires the shared container via
     `tokio::sync::OnceCell<SharedMysql/Postgres>`,
   - creates a `CREATE DATABASE <dl_t_$pid_$counter>`,
   - returns a TestContext whose Drop spawns a
     short-lived runtime to `DROP DATABASE`.

   Result on `philharmonic-store-sqlx-mysql`: 28 tests in
   ~17s wall clock (vs. timing out at 180s × 28 ÷ 4
   threads = many minutes pre-pivot).
4. **`wait_for_log` fail-fast on exit**: when a container
   dies during init (e.g. AIO exhaustion on parallel
   MySQL pulls before Yuka's `fs.aio-max-nr` bump),
   bollard's `logs`-follow stream returns historical
   lines then EOFs. The previous retry-on-stream-end
   loop reopened the stream and re-read the same dead
   logs until the deadline. Now: on stream end,
   `inspect_container` checks the running state; if
   exited, return `Error::ReadinessFailed` immediately
   with the exit code embedded.

### Verification

- `cargo check --workspace --tests` → clean.
- `./scripts/cargo-deny.sh` → `bans ok` with
  `rustls-native-certs` as a no-wrapper full ban; only
  `quinn-proto` wrapper for `ring` remains (upstream
  h3-quinn 0.0.10 feature-unification bug).
- `./scripts/pre-landing.sh` → all checks passed,
  including the `--ignored` testcontainer phases against
  the migrated consumers. (Multiple iterations during
  the post-Codex debugging session; final clean run is
  the dispatch's lands-state.)
- `cargo tree --workspace --invert rustls-native-certs -e all --target all`
  → **empty**.

### Known follow-up

**Container leak on test-binary exit.** The warm-
container pattern stores the `Container` inside a static
`OnceCell`, and Rust doesn't run `Drop` on statics at
process exit. The container stays running after the test
binary terminates; Docker reaps the underlying mysqld
process only when the container is explicitly stopped or
removed. Pragmatic effect: `docker ps` lists
`dockerlet-` containers from prior test runs until the
user manually `docker rm -f`s them.

Fix queued as a separate dispatch / commit: dockerlet
registers a process-level `libc::atexit` hook that stops
each spawned container; combined with Docker's
`auto_remove: true`, the daemon then removes them
cleanly. Not blocking for D23's "remove the
`rustls-native-certs` wrapper" objective, but a real
dockerlet quality issue to land next.

### Sequencing follow-up to D23

§3.J ordering becomes **D24 next** (workspace-wide
`default-features = false` audit). D23's atexit
cleanup ships as a `dockerlet 0.1.1` patch release
ahead of D24.

## Prompt (verbatim)

> Implement D23 per the archived prompt at
> `docs/codex-prompts/2026-05-13-0004-d23-dockerlet-testcontainers-replacement-01.md`.
>
> Replace the `dockerlet` 0.0.0 placeholder with a 0.1.0
> substantive release: a thin minimal-feature wrapper
> over `bollard` for the workspace's integration-test
> fixtures. Public surface mirrors the testcontainers
> shapes the consumers already use (GenericImage builder
> + Container handle + WaitFor probe + ContainerPort
> + IntoContainerPort trait). Concurrency-limit knob via
> a cross-process file-lock semaphore with N =
> `min(4, available_parallelism / 4)` slots. Bollard
> feature set deliberately narrow — no `home`, no
> `ssl_providerless`, no `rustls-native-certs`. Tokio
> async; Unix-socket transport only.
>
> Then migrate the six (or however many; grep the
> workspace for `testcontainers` / `testcontainers_modules`
> usage) consumer test files from `testcontainers`
> /`testcontainers-modules` to `dockerlet`, update each
> consumer's `Cargo.toml` dev-deps + bump the consumer's
> patch version + add a CHANGELOG entry, add
> `dockerlet = { path = "dockerlet" }` to the workspace
> root `Cargo.toml`'s `[patch.crates-io]` block, and
> swap the `rustls-native-certs` wrapper in `deny.toml`
> for a no-wrapper full ban entry.
>
> Run `./scripts/pre-landing.sh` end-to-end; the
> workspace's cargo-deny, fmt, check, clippy, rustdoc,
> and tests (including the `--ignored` testcontainer
> phases on the migrated consumers) must all pass. Do
> not invoke any git wrapper or `publish-crate.sh` —
> Claude commits + pushes + Yuka publishes from her own
> session.
>
> The full task spec, severity analysis, hard
> requirements, and verification steps are in the
> archived prompt; consult it instead of guessing.
