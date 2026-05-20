# mechanics-worker: Prometheus-compatible `/metrics` endpoint

**Date:** 2026-05-20 (JST)
**Slug:** `mechanics-worker-prometheus-metrics`
**Round:** 01 — initial dispatch.
**Subagent:** `codex:rescue`

## Motivation

`mechanics-worker` (the JS execution HTTP service at
[`bins/mechanics-worker`](../../bins/mechanics-worker/))
currently exposes a single endpoint: `POST /api/v1/mechanics`
for job submission. Production deployments need
Prometheus-compatible runtime metrics — pool state, job
lifecycle counters, worker restarts — for monitoring and
alerting.

Per Yuka's call (2026-05-20):

- **Separate bind.** Metrics live on a second listener,
  configured via an **optional** `bind_metrics:
  Option<SocketAddr>` config field. Omitted (the default)
  means **no metrics endpoint runs** — operators opt in
  per-deployment.
- **Unauthenticated.** Metrics endpoints are scraped by
  Prometheus on a private network; bearer tokens are not
  required. Operators control reachability via the bind
  address (`127.0.0.1`, an internal NIC, etc.) and the
  surrounding firewall.
- **Prometheus text format.** Standard exposition format
  (`# HELP …`, `# TYPE …`, sample lines) served from
  `GET /metrics`.
- **Crate choice locked:** `metrics` (the facade,
  workspace-installed via `philharmonic-core`'s upstream
  dep) + `metrics-exporter-prometheus` (the renderer).
  Mature, minimal deps, neither is on the workspace ban
  list.

## Hard constraints

- **`bind_metrics` is optional.** Omitted → no second
  listener spawned, no `/metrics` endpoint exposed. Tests
  must cover both the present and absent cases.
- **Separate listener thread.** Don't multiplex `/metrics`
  onto the main `POST /api/v1/mechanics` handler. The
  existing `MechanicsServer::run` / `run_tls` /
  `run_tls_with_h3` spawns its own listener thread; the
  metrics listener runs on its own thread alongside.
- **Unauthenticated.** No bearer-token gate on `/metrics`.
  No `X-Tenant-Id` or other headers required.
- **Plain HTTP, no TLS, no HTTP/3** on the metrics port.
  Prometheus scrapers default to plain HTTP on a private
  network; matching that simplifies the bind config.
- **No raw `cargo`, no raw `git`.** Use the wrappers
  (`rust-lint.sh --phase check -p <crate> --quiet`,
  `rust-test.sh <crate>`, `pre-landing.sh`). Cross-compile
  to verify non-Linux dead-code via the new
  `rust-lint.sh --target x86_64-unknown-freebsd` flag if
  the metrics code goes through any cfg-gated paths.
- **No `head` / `tail` on `scripts/*.sh` output** —
  soft-banned. Redirect + `grep` / `Read` to slice.
- **You do not commit or push.** Leave the working tree
  dirty across the touched crates; Claude commits + pushes.
- **No panics in library `src/`** (§10.3). The metrics
  facade's `counter!` / `gauge!` / `histogram!` macros
  are `?` -less and don't panic on internal failures
  (they no-op when no recorder is installed) — confirm
  with a quick read of the upstream docs.
- **`metrics-exporter-prometheus` install is fallible**
  (network bind can fail); surface a `Result` to the
  caller, **don't unwrap**. Failure to bind the metrics
  port is a startup failure, not silently ignored.

## Per-file scope

### `mechanics-core/` — pool / job instrumentation

Add the `metrics` crate (facade only) as a workspace dep.
The library records counters / gauges / histograms via the
facade; the binary chooses the exporter.

**Suggested metric set** (Codex tunes the exact names /
labels for Prometheus conventions — `snake_case`, suffix
`_total` on counters, `_seconds` on durations, etc.):

- `mechanics_jobs_accepted_total` (counter) — every job
  the pool successfully accepts onto the queue.
- `mechanics_jobs_completed_total{outcome="ok"|"failed"|"timeout"|"cancelled"}`
  (counter) — every terminal job state.
- `mechanics_jobs_queue_depth` (gauge) — current queue
  length, sampled per-second or per-acquire.
- `mechanics_pool_workers_total` (gauge) — total workers
  in the pool (steady state == pool size from
  `MechanicsPoolConfig`).
- `mechanics_pool_workers_busy` (gauge) — workers
  currently executing a job.
- `mechanics_worker_restarts_total{reason="panic"|"timeout"|"other"}`
  (counter) — every supervisor-driven worker restart.
- `mechanics_job_duration_seconds` (histogram) — wall-clock
  time from job acceptance to terminal state. Bucket
  boundaries: pick something Prometheus-conventional for
  short-lived (<10s) RPCs (e.g., the
  `metrics-exporter-prometheus` default buckets, or
  custom: 0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1, 5, 10).

Instrumentation sites in `mechanics-core/src/internal/pool/`
(approximate — Codex picks based on the actual call sites):

- `pool/shared.rs` worker-thread spawn / restart points
  (the existing `catch_unwind` sites are also where
  worker-restart counters fire).
- Job acceptance / queue paths.
- Job completion / timeout / cancellation handlers.

**Don't** add new public API to `mechanics-core` for
this; the metrics facade calls (`counter!(...)`,
`gauge!(...)`, `histogram!(...)`) are recording-side
only and don't need to be exposed.

### `mechanics-worker/Cargo.toml`

- Add deps:
  - `metrics-exporter-prometheus` — renders the
    `/metrics` output.
  - The `metrics` facade is already used by
    `mechanics-core`; mechanics-worker pulls it
    transitively or adds it directly if more ergonomic.
- No new feature flags needed; the metrics endpoint is
  controlled by the runtime config field.

### `mechanics-worker/src/config.rs`

- Add `pub bind_metrics: Option<SocketAddr>` field.
  Default `None`. Document in the TOML schema.
- Update `DEFAULT_CONFIG` literal in `main.rs` to mention
  the field with a commented-out example, e.g.:

  ```toml
  # Optional: serve Prometheus-compatible runtime metrics
  # at `GET /metrics` on this bind. Omit to disable.
  # bind_metrics = "127.0.0.1:9100"
  ```

### `mechanics-worker/src/main.rs`

- After `MechanicsServer::new(...)` and before / after the
  main listener starts, install the prometheus exporter:

  ```rust
  // Pseudocode — Codex picks the exact API shape from the
  // metrics-exporter-prometheus docs.
  if let Some(bind_metrics) = config.bind_metrics {
      PrometheusBuilder::new()
          .with_http_listener(bind_metrics)
          .install()
          .map_err(|e| format!("failed to bind metrics listener at {bind_metrics}: {e}"))?;
      eprintln!("mechanics-worker metrics listening on {bind_metrics}");
  }
  ```

  The exporter's `install()` spawns its own listener
  thread under the global tokio runtime (or pin it to
  the same multi-thread runtime the main server uses —
  Codex picks; document the choice).
- Hot-reload note: on SIGHUP reload of the config, if
  `bind_metrics` changes (added / removed / re-bound),
  the simplest policy is to log a warning that the
  metrics bind requires a restart to change. The
  `metrics-exporter-prometheus` install is one-shot per
  process; replacing the bind at runtime is non-trivial.
  Document the limitation in `config.rs`'s field doc.

### Tests

- `mechanics-core/tests/` (or wherever the existing pool
  tests live) — at least one test that:
  - Installs a `metrics-util::debugging::DebuggingRecorder`
    (or equivalent) as the recorder.
  - Submits a small job, lets it complete.
  - Asserts the expected counters / gauges fired
    (`mechanics_jobs_accepted_total == 1`, etc.).
- `mechanics-worker/tests/` (if a harness fits) — at
  least one `#[ignore]`-gated integration test that:
  - Boots the worker with `bind_metrics` set to a free
    port.
  - HTTP-GETs `/metrics` (use the workspace's
    `mechanics-http-client` — NOT reqwest, which is
    banned).
  - Asserts the response is `200 OK`, content-type
    `text/plain; version=0.0.4` (Prometheus convention),
    and the body contains at least one of the documented
    metric names + a `# HELP` line.
- Cover the `bind_metrics = None` case: with the field
  omitted, no second listener is spawned and `/metrics`
  returns a connection-refused at the (would-be) port.

## Verification

```sh
./scripts/pre-landing.sh
```

Must print `=== pre-landing: all checks passed ===`.

If the metrics code adds any cfg-gated paths:

```sh
./scripts/rust-lint.sh mechanics-worker --phase check --target x86_64-unknown-freebsd --quiet
```

Cross-target dead-code clean.

## Hand-off shape — Codex does not commit

Leave the working tree dirty across `mechanics-core/`,
`mechanics/`, `mechanics-worker/` (parent `bins/`), and
parent `Cargo.lock`. Claude commits + pushes after
reviewing.

- No `./scripts/commit-all.sh` (any flags).
- No raw `git commit` / `push` / `add` / `reset` /
  `rebase` / `amend`.
- No raw read-only `git status` / `log` / `diff` —
  wrappers only (`status.sh`, `log.sh`, `heads.sh`).
- No `./scripts/push-all.sh`, no
  `./scripts/publish-crate.sh`.
- No `HUMANS.md` edits.

## Codex report (encouraged)

If anything non-obvious surfaces — a quirky
`metrics-exporter-prometheus` API shape, a histogram
bucket choice that needs design input, a hot-reload edge
case worth Yuka's attention — write a short report to
`docs/codex-reports/2026-05-20-0003-mechanics-worker-prometheus-metrics.md`
per [`docs/codex-reports/README.md`](../codex-reports/README.md).

## Outcome

Implemented across `mechanics-core/`, `bins/mechanics-worker/`, parent `Cargo.lock`, and this prompt archive: core now depends on `metrics` and records `mechanics_jobs_accepted_total`, `mechanics_jobs_completed_total{outcome="ok"|"failed"|"timeout"|"cancelled"}`, `mechanics_jobs_queue_depth`, `mechanics_pool_workers_total`, `mechanics_pool_workers_busy`, `mechanics_worker_restarts_total{reason="panic"|"timeout"|"other"}`, and `mechanics_job_duration_seconds` from `mechanics-core/src/internal/pool/{api,shared,drop_impl,constructor,metrics}.rs`. `mechanics-worker` now has optional `bind_metrics: Option<SocketAddr>` defaulting to `None`; when set, it binds a dedicated plain-HTTP listener thread using `metrics-exporter-prometheus`'s `PrometheusBuilder::install_recorder()` / `PrometheusHandle::render()` instead of adding `/metrics` to the existing mechanics API listener. SIGHUP reload compares the running `bind_metrics` with the reloaded config and logs that changing it requires a restart; it never attempts in-process rebind. Tests added `mechanics-core/tests/pool_metrics.rs` recorder assertions and ignored `bins/mechanics-worker/tests/metrics_endpoint.rs` subprocess tests for enabled and omitted `bind_metrics`; focused wrapper checks and final `pre-landing.sh` passed, with a temporary writable `CARGO_HOME` because the sandbox home cargo cache is read-only. Residual risk: no current pool path restarts workers for ordinary job timeouts, so the `reason="timeout"` restart series is registered for schema completeness but only `panic` and `other` restart reasons are emitted by current supervisor flows; hand-off SHAs are parent `4ce4df9` and `mechanics-core` `2a81cf5`.

**Claude post-Codex follow-up (2026-05-20 JST):**
- Extracted the metrics HTTP loop from `bins/mechanics-worker/src/main.rs` into a new `mechanics::metrics` module (`mechanics/src/metrics.rs`) exposing `pub fn install_prometheus_exporter(bind: SocketAddr) -> io::Result<()>`, so standalone `mechanics` consumers (not just the in-tree worker) can opt into runtime metrics from the same `mechanics-core` pool instrumentation. `mechanics-worker` now calls into the crate-tier helper; the binary's own metrics impl + the duplicate `metrics-exporter-prometheus` dep were removed. `mechanics/CHANGELOG.md` records the new module under `[Unreleased]`.
- Hardened two timing-sensitive pool tests that flaked under parallel `cargo test --workspace` CPU contention (not regressions — both pass cleanly in isolation; both tests construct `RuntimeInternal` directly and therefore never touch Codex's pool instrumentation): `d17_fire_and_forget_endpoint_replies_before_tail_completes` bumped its 200ms early-reply `recv_timeout` to 2s and its 1s release-completion `recv_timeout` to 5s; `d17_main_never_settles_preserves_default_export_timeout_error` bumped `max_execution_time` from 100ms to 500ms so the runtime reliably reaches the default-export-settle check before some other deadline path fires under load.

---

<task>
Add Prometheus-compatible `/metrics` to `mechanics-worker`
on a **separate, optional listener**.

**Hard constraints (locked):**

- `bind_metrics: Option<SocketAddr>` config field. Omitted →
  no metrics listener spawned. Default is `None` (off).
- Separate listener on its own thread; do NOT multiplex
  `/metrics` onto the existing `POST /api/v1/mechanics`
  handler in `MechanicsServer`.
- Unauthenticated. No bearer-token gate.
- Plain HTTP only (no TLS, no HTTP/3) on the metrics port.
- Crates: `metrics` (facade) + `metrics-exporter-prometheus`
  (renderer). `metrics` goes into `mechanics-core` for
  library instrumentation; `metrics-exporter-prometheus`
  goes into `mechanics-worker` for the exporter install.
- Metric set per the preamble's "Suggested metric set"
  (Codex tunes exact names / labels to Prometheus
  conventions: `snake_case`, `_total` on counters,
  `_seconds` on durations).
- No panics in library `src/` (§10.3). Metrics-facade
  macros (`counter!`, `gauge!`, `histogram!`) are
  non-fallible by design; confirm with the docs.
- `PrometheusBuilder::install` (or equivalent) is fallible
  on bind error — surface the error to the caller; don't
  unwrap.
- On SIGHUP config reload, `bind_metrics` changes require
  a process restart (log a warning if it changes mid-run).
- No raw `cargo`, no raw `git`. Use the wrappers.
- No `head` / `tail` on `scripts/*.sh` output. Redirect to
  a file and `grep` / `Read` to slice.
- You do not commit or push. Leave dirty across
  `mechanics-core/`, `mechanics/` (if touched),
  `mechanics-worker/` (parent `bins/`), parent
  `Cargo.lock`.

**Reference docs (authoritative if they contradict this prompt):**

- The full preamble above (Per-file scope, Tests,
  Verification, Hand-off).
- `CLAUDE.md` §"Hard rules vs. soft rules" + the
  exec-summary bullets for raw cargo / raw git /
  head-tail.
- `CONTRIBUTING.md` §§4, 5, 10.3, 10.16 (`catch_unwind`
  discipline — if you reach for `catch_unwind` anywhere
  in the new code, document per §10.16), 11.

**Per-file scope summary:**

- `mechanics-core/Cargo.toml` — add `metrics` dep.
- `mechanics-core/src/internal/pool/` — instrument the
  pool / job lifecycle: accepted, completed (by
  outcome), queue depth, worker restarts, job
  duration.
- `mechanics-worker/Cargo.toml` — add
  `metrics-exporter-prometheus` dep.
- `mechanics-worker/src/config.rs` — add
  `bind_metrics: Option<SocketAddr>` field, doc-commented.
- `mechanics-worker/src/main.rs` — install the exporter
  when `bind_metrics` is `Some`; log the bind; update
  `DEFAULT_CONFIG` to mention the field.
- `mechanics-core/tests/` (or the existing pool test
  module) — recorder-based test asserting expected
  counter/gauge/histogram firings on a job round-trip.
- `mechanics-worker/tests/` (or `#[ignore]`-gated
  integration test) — boot the worker with
  `bind_metrics` set, HTTP-GET `/metrics`, assert 200 +
  Prometheus content-type + body contains a documented
  metric. Also cover the `bind_metrics = None` (omitted)
  case.

**Verification (must run + pass before declaring done):**

- `./scripts/pre-landing.sh` clean
  (`=== pre-landing: all checks passed ===`).
- If metrics code adds cfg-gated paths:
  `./scripts/rust-lint.sh mechanics-worker --phase check
  --target x86_64-unknown-freebsd --quiet` clean.

<default_follow_through_policy>
Land the mechanics-core instrumentation, the
mechanics-worker exporter install, the config field, the
config docs, the tests, and any CHANGELOG entries in this
single round. "Instrumentation done, exporter pending" or
similar partials are NOT complete — keep going.

If a hard blocker surfaces (e.g., `metrics-exporter-prometheus`'s
public API doesn't match the prompt's pseudocode), STOP
and report the blocker before partial landing.
</default_follow_through_policy>

<completeness_contract>
"Complete" means:

1. `mechanics-core/Cargo.toml` includes the `metrics` dep
   with a sensible version pin and `default-features =
   false` (or the minimal feature set the facade needs).
2. `mechanics-core/src/internal/pool/` records the
   documented counters / gauges / histograms at the
   right call sites. Names use Prometheus conventions
   (`snake_case`, `_total` on counters, `_seconds` on
   durations).
3. `mechanics-worker/Cargo.toml` includes
   `metrics-exporter-prometheus`. Library deps are
   minimal; no new transitive crates flagged by
   `cargo-deny`.
4. `mechanics-worker/src/config.rs` has
   `pub bind_metrics: Option<SocketAddr>`, doc-commented
   as opt-in / no-default.
5. `mechanics-worker/src/main.rs` installs the exporter
   when `bind_metrics` is `Some`, surfaces bind errors as
   `Result<_, String>` to the caller, logs the bind to
   stderr (matching the existing main listener log style),
   and on SIGHUP reload logs a warning if `bind_metrics`
   changes (no in-process re-bind).
6. `DEFAULT_CONFIG` constant in `main.rs` includes a
   commented-out `bind_metrics` example.
7. Tests cover both the recorder side (mechanics-core
   counters fire on job round-trip) and the exporter
   side (mechanics-worker `/metrics` returns 200 +
   Prometheus content-type + at least one documented
   metric name).
8. The `bind_metrics = None` case is exercised: no
   second listener, no `/metrics`.
9. `./scripts/pre-landing.sh` passes.
10. Working tree dirty across `mechanics-core/`,
    `mechanics-worker/` (parent `bins/`), parent
    `Cargo.lock`, plus `mechanics/` if any touch was
    needed. No commits.
11. Session summary lists which submodule + parent are
    dirty + the histogram bucket choice (default or
    custom) + the exporter's runtime model (own thread
    via builder, or share the existing tokio runtime).
12. `## Outcome` section of this prompt file updated
    with files touched, the chosen metric names + their
    sites, the exporter API shape, residual risks,
    hand-off SHAs.

If any of (1)–(11) is incomplete, the dispatch is
INCOMPLETE. Report INCOMPLETE clearly with what's done
and what's left, and STOP.
</completeness_contract>

<verification_loop>
Mid-iteration:

  ./scripts/rust-lint.sh mechanics-core --phase check --quiet
  ./scripts/rust-lint.sh mechanics-worker --phase check --quiet
  ./scripts/rust-test.sh mechanics-core
  ./scripts/rust-test.sh mechanics-worker

Final:

  ./scripts/pre-landing.sh

No raw `cargo`. The wrappers cover fmt + check + clippy +
doc + test with the right `CARGO_TARGET_DIR`.
</verification_loop>

<missing_context_gating>
Before editing:

  ./scripts/status.sh

Parent + every submodule should print `(clean)`. If
anything else is dirty, STOP and report.

If `metrics-exporter-prometheus`'s public API differs
substantially from the pseudocode in the preamble — e.g.,
the `install()` method has a different signature, or
binds via a different mechanism — adapt to the real API
and document the adaptation in the session summary.
Don't STOP for a minor API-shape difference; do STOP if
the crate doesn't support the "separate HTTP listener"
posture at all (which would mean a different crate is
needed and the prompt's locked choice is wrong).
</missing_context_gating>

<action_safety>
- You do not commit, push, or publish.
- Use the script wrappers; no raw cargo, no raw git.
- POSIX-ish host: no bash-only constructs.
- JST is the workspace timezone; today is 2026-05-20 (Wed).
- No `head` / `tail` on `scripts/*.sh` output.
- No edits to `HUMANS.md`.
- `CARGO_TARGET_DIR=target-main` is set by the wrappers.
- `mechanics-worker` is unpublished (`publish = false`),
  but `mechanics-core` is published — bumping the
  `mechanics-core` crate version is NOT part of this
  dispatch; the change rides in `[Unreleased]` until
  Yuka cuts the next release.
</action_safety>

<structured_output_contract>
Return:

1. **Summary** (3-4 sentences): the recorded metrics, the
   exporter install shape, the bind / threading model,
   the histogram bucket choice.
2. **Touched files**: grouped by submodule + parent.
3. **Metric names + sites**: paste the list of metric
   names + the file:line where each one is recorded.
   Include the type (counter / gauge / histogram) and
   any labels.
4. **`metrics-exporter-prometheus` install snippet**:
   paste the actual install code so the reviewer can
   confirm error handling + thread model.
5. **`bind_metrics` config field**: paste the struct field
   + doc comment + the `DEFAULT_CONFIG` line.
6. **SIGHUP reload behaviour**: describe what happens
   when the reload sees a changed `bind_metrics`.
7. **Test coverage**: list new test names + what they
   assert. Confirm both recorder-side and exporter-side
   tests landed, including the `bind_metrics = None`
   case.
8. **Verification results**:
   - `pre-landing.sh` PASS / FAIL.
   - Cross-target check (if cfg-gated paths added):
     PASS / FAIL.
9. **Working-tree state at hand-off**: dirty submodules
   + parent. No commits.
10. **Codex report**: presence / skipped.
11. **Residual risks**: anything Yuka should know — e.g.,
    a metrics-exporter-prometheus version pin that
    diverges from the rest of the workspace, a
    deny.toml ban we got close to triggering, a
    Prometheus-convention naming call I should re-review.
12. **Outcome paragraph** for the prompt-archive file:
    4-6 sentences ready to paste into `## Outcome`.
</structured_output_contract>
</task>
