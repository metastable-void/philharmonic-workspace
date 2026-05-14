# Execution Substrate

Three published crates: `mechanics-config` (Boa-free schema
types), `mechanics-core` (Rust library wrapping Boa), and
`mechanics` (HTTP service). Use `./scripts/crate-version.sh
--all` for current versions.

## `mechanics-config`

Schema types describing the executor's runtime configuration.
Published separately from `mechanics-core` so consumers that
need the types but not the JS runtime (notably
`philharmonic-connector-client`) can depend on the schema without
pulling in Boa transitively.

### Types

- **`MechanicsConfig`** — `HashMap<String, HttpEndpoint>`. The
  `String` is the script-local endpoint name (the key the
  script uses to refer to this endpoint); the `HttpEndpoint`
  is the runtime endpoint configuration.
- **`HttpEndpoint`** — rich structure: method, `url_template`
  with parameterized slots, `url_param_specs`, `query_specs`,
  `headers`, `overridable_request_headers`,
  `exposed_response_headers`, `retry_policy`, `timeout_ms`,
  `response_max_bytes`.
- Supporting types: URL template AST, slot specs, retry policy,
  header name types.

### Validation

`MechanicsConfig::validate` and `HttpEndpoint::validate_config`
perform purely structural validation — URL template parsing,
slot specs, header name format, bounds checks. No I/O, no
reachability checks. Validation is a function of the config's
contents alone.

This validation logic moves with the types; it has no
dependencies on Boa, so the `mechanics-config` crate carries
everything needed to produce and validate configurations.

### Dependencies

No philharmonic crates; no Boa. Depends on `serde`, `serde_json`,
standard library, and whatever small utility crates the types
need. Intentionally lightweight.

## `mechanics-core`

JavaScript execution library using Boa 0.21. Thread-pooled
runtimes, stateless per-job realm isolation, JSON-in/JSON-out
API.

### Core types

- **`MechanicsPool`** — thread pool of Boa runtimes. Constructed
  from `MechanicsPoolConfig`. Provides `run(MechanicsJob)` which
  returns a `Result<JsonValue, MechanicsError>`.
- **`MechanicsJob`** — constructed via
  `MechanicsJob::new(module_source, arg, config)`. Carries the
  script source, the JSON argument, and the configuration.
- **`MechanicsError`**, **`MechanicsErrorKind`** — error types
  with stable symbolic kind codes.

### Wrapper newtypes for Boa GC integration

The types in `mechanics-config` (`MechanicsConfig`,
`HttpEndpoint`) are Boa-free. `mechanics-core` provides wrapper
newtypes that add Boa's GC trait impls (`Trace`, `Finalize`,
`JsData`):

```rust
#[derive(Deref, Trace, Finalize, JsData)]
pub struct BoaMechanicsConfig(
    #[unsafe_ignore_trace] mechanics_config::MechanicsConfig
);
```

`unsafe_ignore_trace` is sound because the config types contain
no GC-managed objects — they're plain data (strings, maps,
integers). Consumers inside `mechanics-core` work with the
wrapper; consumers outside (the lowerer) work with the unwrapped
types directly via `mechanics-config`.

### The JS contract

Scripts are ECMAScript modules. The default export is an async
function with one parameter:

```javascript
export default async function main(arg) {
    // arg: JSON value passed in by the caller
    return /* JSON value */;
}
```

`mechanics-core` itself is agnostic to the argument and return
shapes; it serializes the argument into the Boa realm as a JS
value and deserializes the return value back to JSON.

Philharmonic's convention, layered on top, is:

```javascript
export default async function main({context, args, input, subject}) {
    // context: threaded state, evolves step by step
    // args:    per-instance value supplied at creation, immutable
    // input:   per-step value, varies per invocation
    // subject: authenticated caller context
    return {context: newContext, output: stepOutput, done: true};
}
```

See `07-workflow-orchestration.md` for the shape and semantics
of these fields in philharmonic's workflow context.

### Statelessness

- Each job runs in a fresh Boa realm.
- `globalThis` mutations don't persist across jobs.
- No cross-job caches, no worker affinity.
- Workers are fungible; any worker can run any job.

### Error handling

JS exceptions are stringified at the realm boundary and returned
to the caller. Scripts cannot communicate structured error
information via exceptions — structured failures go through the
return value.

### Host functions

Scripts can call host-provided functions for operations
JavaScript can't do natively or that the host needs to mediate.
This is how connector calls happen: the script invokes a host
function; the runtime makes the actual HTTP call to the
connector service using the endpoint configuration from the
`MechanicsConfig` loaded with the job.

The host-function API is `mechanics-core`'s responsibility;
precise details are in the crate's own documentation.

### Realm surface (no non-ES globals)

**Hard rule:** the Mechanics realm exposes only ECMAScript-spec
globals plus the `mechanics:*` synthetic modules. Web Platform /
WHATWG surface is **not** part of the contract — neither as
realm globals nor as `mechanics:*` module exports. That
explicitly excludes (non-exhaustive):

- `setTimeout`, `setInterval`, `clearTimeout`, `clearInterval`
- `requestAnimationFrame`, `cancelAnimationFrame`,
  `requestIdleCallback`
- `queueMicrotask` (microtasks are implicit via Promise
  chaining; no explicit user-facing scheduling primitive)
- `fetch`, `XMLHttpRequest`, `WebSocket` (outbound HTTP goes
  through `mechanics:endpoint` only)
- `window`, `document`, `navigator`, `location`, `history`,
  `localStorage`, `sessionStorage`, `IndexedDB`, `crypto`
  (the WHATWG one — JS-engine `Math.random` is fine; Web
  Crypto isn't), `performance`, and anything else with a
  `[Exposed]`-to-the-Web IDL annotation
- Any node-built-in (`process`, `require`, `Buffer`, …)

Operational reason: workflows that reach for these surfaces
typically violate Mechanics's stateless-per-job contract or
the connector framework's centralised side-effect routing.
Forbidding them at the realm boundary prevents whole classes
of bad script authoring (fire-and-forget timer callbacks that
escape the workflow's response fence, direct outbound HTTP
that bypasses connector tokens + payload encryption, DOM-
emulation libraries like `jsdom` accidentally working and
then carrying state that doesn't survive worker fungibility).

Engine-internal job-queue / microtask plumbing is fine — Boa
uses `TimeoutJob` internally to drive Promise resolution and
microtask draining. The constraint is on the **script-visible
surface**, not the host's implementation details.

The D18 module batch (`mechanics:mime` / `mechanics:url` /
`mechanics:console` / `mechanics:html`) shipped in
mechanics-core 0.6.0 under this constraint: ECMAScript-shaped
APIs (named exports for stateless functions, default-exported
classes for `URL`/`URLSearchParams`/`console`), no implicit
globals, and no re-export of WHATWG-shaped surfaces even where
the underlying Rust crate mirrors one. Any future
`mechanics:*` module must follow the same posture.

### Tail-promise polling

When the script's top-level resolves (sync return, or an awaited
promise fulfilled), `mechanics-core` serialises the result and
returns the run-job response immediately. Any pending promises
left inside the realm — unawaited `mechanics:endpoint(...)` calls
and fire-and-forget `Promise.then(...)` chains — continue to be
polled in the background until they settle.

**The Mechanics realm provides no timer surface.** Scripts have no
`setTimeout` / `setInterval` / `requestAnimationFrame` /
`queueMicrotask` globals and no `mechanics:*` module exporting any
such function — see the "Realm surface (no non-ES globals)" hard
rule above. The host's underlying job queue (Boa's `TimeoutJob` +
microtask plumbing) is an implementation detail used internally
to drive Promise resolution; it is **not** exposed to script
authors and not a legitimate source of tail work.

This is the **hardcoded default**. There is no per-job knob and
no per-worker knob; every script execution gets the same
behavior.

Lifetime is bounded by the per-job `max_execution_time`:

- The deadline is **shared** between main execution and
  tail-poll. If main takes 3 s of a 10 s budget, tail-poll has
  7 s remaining.
- When the deadline trips during tail-poll, the realm and all
  in-flight futures are dropped, and one `tracing::warn!` line
  is emitted naming the job ID and the count of in-flight +
  queued tail jobs at abort time.
- The realm stays alive on the worker tokio task that started
  the job until tail-poll exits (quiescence or deadline). The
  worker tokio slot is occupied for that duration; existing
  worker-pool slot limits self-regulate backpressure — no new
  cap is introduced.

Side effects in tail-poll still complete: real HTTP requests to
connector services still go out, real Promise chains still
settle. What changes is that the run-job response is no longer
held open by them. The script's `return` is the response fence;
quiescence is not.

Tail-promise outcomes are **not** recorded in the workflow step
record — fire-and-forget. Unhandled rejections during tail-poll
increment the internal `pending_unhandled_rejections` counter
(see `RuntimeHostHooks`); if and when external metrics emission
is added to `mechanics-core`, that counter is the natural place
to bridge tail rejections into telemetry. Until then, only the
deadline-abort `tracing::warn!` is visible.

D17 (mechanics-core 0.4.1, 2026-05-12) landed this. Pre-D17
behavior was that `run_jobs()` was called unconditionally
between `main.call(...)` and reading the promise state, so
the response was held open until every queue drained to
quiescence. D17 also briefly added a `setTimeout` realm
global; that addition was reversed in D18 (mechanics-core
0.5.1) per the "no non-ES globals" hard rule above, and
tail-poll behavior continues to be exercised via Promise-
based test fixtures.

### Dependencies

Depends on `mechanics-config` (for the schema types),
`boa_engine`, `serde`, `serde_json`, `tokio`, and assorted
utilities.

## `mechanics` (HTTP service)

HTTP wrapper exposing `mechanics-core` over the network. Worker
nodes run this binary.

Wire format: POST with JSON body `{module_source, arg, config}`.
Response: JSON value (success) or error.

The API layer talks to `mechanics` over HTTP (via an
`HttpStepExecutor` implementation of the workflow engine's
`StepExecutor` trait); never depends on `mechanics-core`
directly.

## Philharmonic's narrow usage

Philharmonic's connector calls never use the rich `HttpEndpoint`
features. Every endpoint in philharmonic's `MechanicsConfig` is:

- `method: POST`
- `url_template`: literal URL, no slots
- `url_param_specs`: empty
- `query_specs`: empty
- `headers`: `{Authorization: "Bearer <COSE_Sign1 token>",
  X-Encrypted-Payload: "<COSE_Encrypt0 payload>"}`
- `overridable_request_headers`: empty
- Standard policy fields from deployment defaults

The richness exists for `mechanics-core` consumers outside
philharmonic; philharmonic uses a small slice.

## Determinism

Deferred. Current executor is non-deterministic (real
`Math.random`, real `Date.now`, real network calls). Append-only
substrate captures full history, so post-hoc diagnosis doesn't
need replay.

Future replay would need: seeded RNG via job parameter, frozen
time via job parameter, recorded I/O results. Path is clear;
adding it later is a pure addition.

## Multi-worker concurrency

- Per-instance: thread pool with multiple Boa runtimes, one job
  per runtime at a time.
- Across instances: horizontal scaling, no coordination, load
  balanced by whatever ingress is in front.
- Effective parallelism: (instances) × (workers per instance).
- The API layer sees one endpoint (the load balancer); unaware
  of worker count.

## What the executor doesn't know

- What workflow a job belongs to. Jobs arrive with
  script + arg + config; no instance ID, no correlation with
  prior jobs.
- What the caller does with results.
- Whether two jobs are related.
- What persistence the caller uses.
- What the fields inside the argument mean (`context`, `args`,
  `input`, `subject` are philharmonic conventions invisible to
  the executor).

The executor is, structurally, the simplest layer of the system:
inputs in, computation, outputs out, no memory of either.

## Schema extraction (history)

The schema types (`MechanicsConfig`, `HttpEndpoint`, supporting
URL/header/retry types, and their structural validation logic)
originally lived in `mechanics-core`. They were extracted to
`mechanics-config` so consumers that need the schema but not
the JS runtime — notably the lowerer in
`philharmonic-api-server`, which produces `MechanicsConfig`
values from `TenantEndpointConfig` decryption — could depend on
the schema without pulling in Boa.

The split that landed:

- `mechanics-config` owns the data types and structural
  validation. No Boa, no GC traits, no philharmonic-specific
  knowledge.
- `mechanics-core` depends on `mechanics-config` and adds the
  Boa GC wrapper newtypes (`BoaMechanicsConfig`,
  `BoaHttpEndpoint`) that integrate with the realm.
- The lowerer depends on `mechanics-config` directly and never
  pulls in Boa.

This is a settled, shipped split — the rest of this document
describes the current state of the three crates.

## Status

All three crates are published and in production use:

- `mechanics-config` — schema types and validation. Stable
  surface; revisions add fields rather than reshape.
- `mechanics-core` — Boa-backed runtime library. Used by the
  `mechanics` bin and not depended on directly by any
  philharmonic crate (philharmonic talks to mechanics over
  HTTP, see "`mechanics` (HTTP service)").
- `mechanics` — the HTTP worker binary. Bundles a static asset
  set and is one of the three deployment binaries.

Use `./scripts/crate-version.sh --all` to read current
versions; this document does not pin them.
