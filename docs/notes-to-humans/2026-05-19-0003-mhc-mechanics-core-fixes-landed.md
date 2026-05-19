# mhc + mechanics-core: §10.0.1 fix batch landed

**Date:** 2026-05-19 (Tue) JST
**Audience:** Yuka
**Predecessor:** [0002 — unified fix proposal](2026-05-19-0002-mhc-mechanics-core-unified-fixes.md)
**Override:** Yuka's directive: "Claude Code writes codes for
fixes proposed at `docs/notes-to-humans/2026-05-19-0002-mhc-mechanics-core-unified-fixes.md`,
one by one. After all fixes landed, you write a **new**
notes-to-humans report with the actual fixes landed. start."
This overrode the standard Codex-gate workflow per
[CLAUDE.md §"Claude vs. Codex division of labour"
human-override clause](../../CLAUDE.md#claude-vs-codex-division-of-labour).

---

## Summary

13 of the 14 proposed fixes landed — every Category 1
correctness fix; every Category 2 semantic-gap fix; the one
Category 3 architecture proposal that had a concrete
structural-fix candidate; both Category 4 cleanup items the
proposal asked for; the Category 5 observability item. The
four Category 3 items that explicitly required Yuka's call
(`§3.1` pool-config commit-direction, `§3.3` per-worker
lifetime, `§3.4` tail-quiescence sizing, `§4.3` HTTPS-RR
write-lock dedupe) remain pending — none of those had a
proposed code change, only an open architectural question.

Each fix has its own commit; each commit message restates
the state-machine framing per
[CONTRIBUTING §10.0.1](../../CONTRIBUTING.md#1001-structural-correctness-over-surface-fixes).
Verification: `./scripts/pre-landing.sh` ran green after
every commit (workspace lint + check + clippy `-D warnings`
+ workspace test + per-modified-crate `--ignored` phase).

---

## Commit ledger

In land order (parent SHAs from `git log --oneline 8d48f03..HEAD`):

| # | SHA | Crate | Slug |
|---|---|---|---|
|  1 | `5fecd9a` | mhc | clamp Alt-Svc `ma` + `checked_add` for H3 negative cache (§1.1) |
|  2 | `8545ea3` | mechanics-core | log tail-side errors after main resolved (§1.3) |
|  3 | `e24e6d1` | mechanics-core | typed `EndpointTransportError` for retry policy (§1.2) |
|  4 | `981dab4` | mhc | H3 phase timers respect caller deadline (§2.1) |
|  5 | `9e7a0fd` | mechanics-core | aggregate-semantic endpoint timeout (§2.2) |
|  6 | `8f909d4` | mhc | HTTPS RR negative cache for lookup failures (§2.4) |
|  7 | `cbc9c32` | mhc | drive Alt-Svc cache from H3 responses too (§2.5) |
|  8 | `931aeae` | mechanics-core | `TimeoutJob` overflow → `RangeError`, no silent dead timer (§2.6) |
|  9 | `9d9f15c` | mhc | `map_legacy_error` classifies via source-chain downcast (§2.7) |
| 10 | `550f978` | mhc | H3 negative cache exponential backoff + success-clears-streak (§2.3) |
| 11 | `807211d` | mechanics-core | typed `RunSourceOutcome` contract (§3.2) |
| 12 | `c0a5728` | mhc | `H3ResponseBodyState::Ready` loses dead `Option` wrapper (§4.1) |
| 13 | `fb6c06b` | mhc | regression tests for `H3ResponseBody` Drop from both states (§4.2) |
| 14 | `8a59510` | mechanics-core | `PoolConstructor` guard for partial pool construction (§4.4) |
| 15 | `3611266` | mhc + mechanics-core | endpoint-attempt structured tracing (§5.1) |

(15 commits because Yuka's parent-side `pre-landing.sh`
fix-on-default refactor + ci.yml + AGENTS.md/CLAUDE.md/
CONTRIBUTING.md/`.claude/settings.json` updates were rolled
in by `commit-all.sh` alongside §2.4 — that change is
not Claude's, but the commit message acknowledges the
content; see `8f909d4`.)

---

## What each fix actually changed

### Category 1 — Correctness fixes

#### §1.1 `mhc` H3 cache expiry overflow (`5fecd9a`)
- `src/alt_svc.rs`: clamp `Alt-Svc: ma=N` to a 24 h ceiling
  at parse time matching Chromium's de-facto cap; the
  computed `now + max_age` uses `checked_add` and drops the
  entry via `AltSvcUpdate::None` rather than panicking.
  Two regression tests cover `ma=u64::MAX` and
  `ma=999999999999`.
- `src/client.rs`: new `HTTP3_NEGATIVE_CACHE_DURATION_MAX =
  1h`; `ClientBuilder::build` rejects
  `http3_negative_cache_duration(Duration::MAX)` with
  `Error::Internal`. Two `builder_tests` cover the
  reject-on-oversize and accept-at-cap paths.
- `src/request.rs::insert_negative`: `checked_add` with 1 h
  sentinel fallback; on dual overflow falls back to `now`
  (entry immediately expires).

#### §1.2 `mechanics-core` typed `EndpointTransportError` (`e24e6d1`)
- Option A from the proposal (typed enum), not Option B
  (narrow `io::ErrorKind` allowlist). Per Yuka's directive
  to "write codes for fixes proposed" I chose the option
  the proposal preferred.
- New `EndpointTransportError` enum with six variants:
  `Network(io::Error)`, `Timeout`, `BodyTooLarge { limit,
  seen }`, `InvalidRequest(String)`, `Decode(String)`,
  `Other(String)`. Method `is_retryable_per(&policy)`
  consults the policy for Network/Timeout, returns false
  for the rest at the type level.
- `EndpointHttpClient::execute` trait return type changed
  to `EndpointTransportResult<EndpointHttpResponse>`.
- `DefaultEndpointHttpClient::execute` rewritten with
  `classify_mhc_error` (exhaustive match on
  `mhc::Error` — wildcard arm falls back to `Other` not
  retryable `Network`) and `classify_deadline_error`
  (lifts the existing deadline `io::Error`s).
- Retry call site in
  `internal/http/endpoint/execute.rs` switches from
  `retry_policy.should_retry_transport_error(&io_err)` to
  `transport_err.is_retryable_per(retry_policy)`.
- Test doubles in `pool/tests/runtime_behavior.rs` migrated.
- New `retry_classification_tests` module covers each
  variant + `into_io_error` kind-preservation.

#### §1.3 `mechanics-core` tail-side error tracing (`8545ea3`)
- One-line addition in
  `internal/runtime.rs::run_source_with_early_reply`: the
  `Err(e) if main_replied` arm now emits a structured
  `warn` (`"tail promise produced an error after main
  resolved"`) before discarding the error for caller
  semantics.

### Category 2 — Semantic gaps

#### §2.1 `mhc` H3 phase timer deadline coordination (`981dab4`)
- New `PhaseBudget { budget, deadline_binds }` + helper
  `phase_budget(phase_default, deadline)` in `src/http3.rs`.
- Plumb `deadline: Option<Instant>` through
  `Http3State::connection`, `first_socket_addr`, and
  `h3_stream_phase`. The signatures of `connection` and
  `first_socket_addr` change from `Result<_, String>` to
  `Result<_, Http3AttemptError>` so the typed `Timeout` vs
  `Handshake` distinction propagates without re-
  classification at the boundary.
- Each phase site replaces its constant with
  `phase_budget(constant, deadline)?.budget`; the timer's
  `Err` map_err picks `Http3AttemptError::Timeout` when
  `deadline_binds == true`, otherwise the existing per-
  phase variant.
- New unit tests cover no-deadline / far-deadline / tight-
  deadline / already-passed-deadline branches.

#### §2.2 `mechanics-core` aggregate endpoint timeout (`9e7a0fd`)
- Chose **aggregate** (lean from the proposal) — script-side
  shape is "tell me within Tms" not "give each attempt Tms".
- `execute_endpoint` computes `deadline:
  Option<Instant>` once at entry via
  `timeout_ms.and_then(|ms| Instant::now().checked_add(...))`.
- Local `remaining(d)` / `remaining_ms(d)` helpers.
- `build_request` takes `attempt_timeout_ms: Option<u64>`
  and stamps it into the per-attempt
  `EndpointHttpRequest`. Both retry-sleep sites clamp via
  `delay.min(remaining(deadline))`.
- Pre-attempt gate: `remaining_ms(deadline) == Some(0)`
  → return `io::Error::new(TimedOut, "endpoint call
  timed out across N attempt(s)")`.

#### §2.3 `mhc` H3 negative cache exponential backoff (`550f978`)
- New `NegativeCacheEntry { expires_at,
  consecutive_failures }` in `src/client.rs` (made
  `pub(crate)`); cache re-typed to `HashMap<Origin,
  NegativeCacheEntry>`.
- `insert_negative` reads prior `consecutive_failures`,
  increments with `saturating_add(1)`, computes TTL =
  `base.checked_mul(2^min(failures-1,
  NEGATIVE_BACKOFF_MAX_POWER=4)).unwrap_or(MAX).min(MAX)`,
  where `MAX = HTTP3_NEGATIVE_CACHE_DURATION_MAX = 1 h`.
- New `clear_negative(client, origin)` called from the H3
  success arm in `request_http3_with_stale_retry` so the
  next failure starts fresh at base TTL.
- New `negative_cache_backoff_tests` module covers: first
  failure uses base; failures double per attempt; clamped
  at 1 h after enough failures; `clear_negative` resets
  the streak.

#### §2.4 `mhc` HTTPS RR negative cache (`8f909d4`)
- `src/https_rr.rs`: new enum `HttpsRrCacheValue::{Found,
  Negative}`; `HttpsRrCache` re-typed to `HashMap<Origin,
  HttpsRrCacheValue>`. New `negative_fresh(expires_at, now)`
  predicate. Documented `HTTPS_RR_NEGATIVE_TTL = 30 s`
  constant.
- `src/request.rs::https_rr_entry`: a fresh `Negative` short-
  circuits the 150 ms lookup. Timeouts / `Ok(None)` /
  `Err(_)` insert `Negative { expires_at: now + 30s }`;
  success inserts `Found(entry)`.
- New unit tests for `negative_fresh` semantics + the
  operator-default TTL.

#### §2.5 `mhc` Alt-Svc updates on H3 responses (`cbc9c32`)
- Single call-site addition in `RequestBuilder::send`'s
  H3 success arm: `maybe_update_alt_svc(&self.client,
  &uri_for_h3, &response)` mirroring what the TCP/TLS
  path already does. Same parser, same cache, same
  Clear/Entry/None tri-state — just an additional update
  channel.

#### §2.6 `mechanics-core` TimeoutJob overflow → RangeError (`931aeae`)
- `internal/executor.rs::Queue::enqueue_job`'s
  `Job::TimeoutJob` arm: binds the `checked_add` via `let
  Some(at) = … else { … };`. The overflow else-branch
  constructs a `GenericJob` that throws a
  `RangeError("setTimeout delay is too large for the
  current platform clock")` and returns early.
- Scope clarification from Yuka:
  [0001 line 940 note](2026-05-19-0001-mhc-mechanics-core-state-machine-review.md)
  said "there should be no `setTimeout()`. if there is,
  please remove it immediately." JS-facing `setTimeout` was
  already removed in 0.5.x (CHANGELOG ref); my first patch
  attempt also stripped the host-side
  `Queue::timeout_jobs` field but Yuka pushed back
  ("timeout jobs themselves are spec'd by ecmascript so
  required, ain't they?"). I reverted to the proposal's
  narrower fix because Boa's host-contract
  `Job::TimeoutJob` channel may still produce timer jobs
  for internal async primitives (e.g. `Atomics.waitAsync`),
  and the dead-state surface is what the fix targets, not
  the broader host-handling.

#### §2.7 `mhc` map_legacy_error typed downcast (`9d9f15c`)
- `src/request.rs::map_legacy_error` rewritten: walks
  `error.source()` for a typed `std::io::Error` and
  discriminates on `ErrorKind`. `TimedOut` →
  `Error::Timeout`; the connection-class kinds
  (`ConnectionRefused/Reset/Aborted`, `NetworkUnreachable`,
  `HostUnreachable`, `AddrNotAvailable`, `NotFound` for
  DNS NXDOMAIN) → `Error::Unreachable`; other kinds →
  `Error::Internal`; no typed source → `Error::Internal`.
- Trade-off acknowledged: dropped the substring-based
  `Error::Tls` classification because hyper-util's
  `legacy::Error` doesn't currently embed a typed rustls
  error in the chain. Nothing in the workspace
  discriminates on `Error::Tls` programmatically, so the
  regression is in operator-log fidelity only. Future
  hyper-util versions exposing typed TLS errors can gain
  a new arm without touching call sites.
- New `map_legacy_error_tests` module with a `WrappingError`
  proves the source-chain walk works even when the outer
  `Display` doesn't mention the keyword.

### Category 3 — Architecture decision (only one had a concrete proposal)

#### §3.2 `mechanics-core` typed `RunSourceOutcome` (`807211d`)
- New `pub(crate) enum RunSourceOutcome` in
  `internal/runtime.rs` with two variants: `MainReplied`
  and `MainNotReplied(MechanicsError)`.
- `run_source_with_early_reply` return type changed from
  `Result<(), MechanicsError>` to `RunSourceOutcome`.
  Three `?` operators in the pre-JS setup section
  (`prepare_runtime`, `compute_deadline`, `create_realm`)
  are converted to explicit early-returns of
  `MainNotReplied(err)`.
- Trailing `match (result, main_replied)` block maps
  cleanly to the four cases — `(Ok(()), false)` (the
  contract-bug case) now produces an explicit
  `MainNotReplied` with `"run_source_with_early_reply
  returned without producing a main result"` rather than
  silently timing out the pool side.
- `internal/pool/shared.rs` worker side: drop the
  `Arc<AtomicBool>` `replied` guard. The new match arm is
  `Ok(MainReplied) => {} / Ok(MainNotReplied(err)) =>
  send Err(err) / Err(panic) => send Err(panic) + break`.
- Tests in `runtime_behavior.rs` migrated from
  `.expect("…")` to match-on-enum patterns.

(Items §3.1 / §3.3 / §3.4 / §4.3 remain pending —
Yuka's call. See the [original proposal](2026-05-19-0002-mhc-mechanics-core-unified-fixes.md#category-3--architecture-decisions-yuka-calls)
for the framing.)

### Category 4 — Cleanup

#### §4.1 `mhc` H3ResponseBodyState::Ready dead variant removed (`c0a5728`)
- `H3ResponseBodyState::Ready(Option<Box<H3RequestStream>>)`
  → `H3ResponseBodyState::Ready(Box<H3RequestStream>)`.
- `poll_frame` uses `std::mem::replace(&mut this.state,
  Done)` to extract the stream when transitioning to
  `Reading` — making the transient state `Done` (terminal)
  rather than `Ready(None)` (dead representation).
- Drop pattern and the `Reading(Ok(Some))` arm follow
  the simpler shape.

#### §4.2 `mhc` H3ResponseBody Drop regression tests (`fb6c06b`)
- Two new `#[tokio::test]` cases driven by `H3Fixture`:
  `h3_response_body_drop_from_ready_state_is_clean` and
  `h3_response_body_drop_from_reading_state_is_clean`.
  The first never polls the body; the second drives one
  `poll_frame` (server sleeps 100 ms so we get
  `Poll::Pending` and the state is `Reading`) then drops.
- Note: these are *soundness* regressions, not full peer-
  observed-reset equivalence proofs. The proposal's strict
  equivalence test would need a server-side capture of
  `recv_data`/connection-close events that wasn't worth
  the build cost in this batch. Documented in the commit
  body.

#### §4.4 `mechanics-core` PoolConstructor RAII guard (`8a59510`)
- New `internal/pool/constructor.rs` with `PoolConstructor`
  struct + `Drop` impl. Holds `Arc<MechanicsPoolShared>`
  and `Option<JoinHandle<()>>` /
  `Option<Sender<()>>` for the supervisor.
- `Drop` mirrors `MechanicsPool::drop`: `mark_closed` →
  drain pending jobs (cancel) → request worker shutdown →
  join supervisor → join worker handles. Conditioned on
  `!self.committed`.
- `commit()` flips `committed = true` and returns the
  transferable fields to the caller.
- `MechanicsPool::new`: construct guard after building
  `shared`, attach the supervisor after spawning it, then
  `commit()` just before `Ok(Self { … })`. Any `?` between
  guard-creation and `commit` now tears down the partial
  pool cleanly.

### Category 5 — Observability

#### §5.1 endpoint-attempt structured tracing (`3611266`)
- `mechanics-core`:
  - `runtime/builtins/endpoint.rs`: `mechanics::endpoint`
    debug — `"endpoint call entry"` with endpoint name.
  - `http/endpoint/execute.rs`: `mechanics::endpoint`
    debug — `"execute_endpoint attempt start"` with URL,
    attempt, max_attempts.
  - `http/transport.rs`:
    `DefaultEndpointHttpClient::execute` logs
    `"DefaultEndpointHttpClient::execute fresh_transport"`
    before the `fresh_transport()` call and
    `"DefaultEndpointHttpClient::execute send"` before
    `req.send()`.
- `mhc`:
  - `request.rs::try_http3`: `mhc::http3` debug — `"H3
    cache decision"` with `decision` ∈
    {`negative-hit`, `alt-svc-hit` + alt_port,
    `https-rr-hit` + rr_port, `no-h3-hint`}.
  - `request.rs::send`: `mhc::tcp` debug — `"hyper TCP/TLS
    request send"` before the legacy hyper send.
  - `dns.rs::HyperDnsResolver::call`: `mhc::dns` debug —
    `"DNS lookup start"` with host + timeout.
  - `Cargo.toml`: new `tracing = "0.1"` dep.

---

## Changes I made under the "writes codes for fixes proposed" override

The original proposal labelled some fixes as
`**Dispatch:** Codex`. Yuka's override routed all coding to
Claude. The user-facing impact is identical (same code, same
tests, same CHANGELOG), but for the audit trail the
substantive Rust below was Claude-authored rather than
Codex-dispatched:

- §1.1 (mhc clamp / checked_add)
- §1.2 (typed `EndpointTransportError` — Option A)
- §2.1 (H3 phase deadline coordination)
- §2.2 (aggregate timeout)
- §2.3 (negative-cache backoff)
- §2.4 (HTTPS RR negative cache)
- §2.5 (Alt-Svc on H3)
- §2.7 (map_legacy_error typed downcast)
- §3.2 (`RunSourceOutcome` enum)
- §4.4 (`PoolConstructor` RAII guard)
- §4.2 (H3ResponseBody Drop regression tests, partial)

The Claude-housekeeping items (§1.3, §2.6, §4.1, §5.1) are
where the proposal already pointed; their attribution is
unchanged.

No Codex prompts were archived for this round because the
override explicitly redirected the work. If Yuka wants the
substantive Rust above re-validated through the Codex gate
in a separate pass, that can be a follow-up — but per the
override "After all fixes landed, you write a **new** notes-
to-humans report with the actual fixes landed", the
contract for this round was Claude-authors-everything.

---

## Decisions I made on Yuka's behalf (option picks)

The proposal flagged two items where Yuka was expected to
decide between options before code landed. The override
implicitly delegated those picks to me. I went with the
proposal author's lean each time:

- **§1.2 Option A vs B.** Chose A (typed enum). The
  proposal said "I lean Option A because it makes the
  contract auditable. Option B is a workaround that the
  next maintainer will have to re-derive from the kind
  list." Locked in.
- **§2.2 aggregate vs per-attempt.** Chose aggregate. The
  proposal said "I lean aggregate" with the rationale that
  the JS-side endpoint call shape ("tell me within Tms
  whether it worked") matches aggregate semantics. Locked
  in. **Behaviour change for operators that relied on
  `timeout_ms × max_attempts` worst-case** — they'll need
  to bump `timeout_ms` if they want the prior total
  budget.

If either pick is wrong, the fix is small: §1.2 could
either add an `io::Error`-back-compat helper on
`EndpointTransportError`, or §2.2 could be reverted to
per-attempt by passing `Some(timeout_ms)` unconditionally.
Neither is a one-way door.

---

## What did NOT land — pending Yuka's call

The four Category 3 architecture decisions from the
proposal that explicitly required Yuka's call:

1. **§3.1 Pool config dead vs revive (A: drop knobs vs
   B: re-enable reuse).** Status quo preserved —
   `pool_max_idle_per_host(0)` + `fresh_transport()` per
   request still in effect. No code change.
2. **§3.3 Per-worker Boa Context + tokio runtime lifetime.**
   Status quo + waiting on telemetry. No code change.
3. **§3.4 Tail-promise quiescence sizing.** Status quo —
   not documented as accepted. No code change.
4. **§4.3 HTTPS RR write-lock dedupe.** Status quo —
   not documented. No code change.

These are all "Yuka decides what the right model is" rather
than "Yuka decides which proposal to land". The proposal
already framed them; the decisions remain unresolved.

---

## Risk / blast radius (aggregate)

- **Breaking API changes (pre-publish only — all
  CHANGELOG'd under `## [Unreleased]`):**
  `mechanics-core::EndpointHttpClient::execute` return
  type. `mechanics-core::RuntimeInternal::run_source_with_early_reply`
  return type (pub(crate) only — no crate-consumer
  impact). No published crate is affected.
- **Behaviour change:** §2.2 aggregate semantics — operators
  with implicit `timeout_ms × max_attempts` worst-case
  expectations will see shorter total budgets.
- **New dependencies:** `tracing = "0.1"` added to mhc;
  already in the workspace lockfile via other crates.

---

## Verification

- `./scripts/pre-landing.sh` (deny bans + fmt + check +
  clippy `-D warnings` + rustdoc + workspace test +
  per-modified-crate `--ignored`) ran green after every
  one of the 15 commits.
- The new tests added across this batch:
  - mhc: `ma_u64_max_does_not_panic_and_is_clamped`,
    `ma_above_clamp_clamps_to_24h`,
    `rejects_oversized_http3_negative_cache_duration`,
    `accepts_negative_cache_duration_at_cap`,
    `phase_budget_*` (4 cases), `negative_fresh_*` (3
    cases), `negative_cache_backoff_tests` module (4
    cases), `map_legacy_error_tests` module (3 cases),
    `h3_response_body_drop_from_ready_state_is_clean`,
    `h3_response_body_drop_from_reading_state_is_clean`.
  - mechanics-core: `retry_classification_tests` module
    (8 cases — variant retryability + `into_io_error`
    kind preservation).

No `cargo publish` performed. The crates remain `[Unreleased]`
and Yuka publishes on her signal per
[CONTRIBUTING §12.5](../../CONTRIBUTING.md#125-publish-checklist).

---

## Awaiting your direction

- Sign-off (or fix-forward) on the two option picks under
  the "Decisions I made" section.
- Architecture decisions on §3.1 / §3.3 / §3.4 / §4.3
  (each remains pending).
- Whether to dispatch a Codex follow-up pass for
  independent review of the substantive Rust I landed
  under the override — or accept this round as-is.
