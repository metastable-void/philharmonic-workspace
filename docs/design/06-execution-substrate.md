# Execution Substrate

Three crates after the planned schema extraction:
`mechanics-config` (Boa-free schema types), `mechanics-core`
(Rust library wrapping Boa), and `mechanics` (HTTP service).
Core is published at 0.2.2; HTTP service is published too.
`mechanics-config` doesn't exist yet — extraction is pending.

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

## Schema extraction (settled)

**Decision**: extract `MechanicsConfig`, `HttpEndpoint`, and
supporting types to a new `mechanics-config` crate. Keep the Boa
GC trait impls in `mechanics-core` via wrapper newtypes. Pure
structural validation logic moves with the types.

**Why**: the `philharmonic-connector-client` crate needs to
produce `MechanicsConfig` values. If the schema types live in
`mechanics-core`, the lowerer transitively depends on Boa, which
is a heavyweight dependency for code that never runs JavaScript.
Extraction lets the lowerer stay Boa-free.

**Migration plan**:

1. Create the `mechanics-config` crate with the extracted types
   and validation logic.
2. Update `mechanics-core` to depend on `mechanics-config` and
   re-export the extracted types for backward compatibility.
3. Add wrapper newtypes in `mechanics-core` that implement Boa's
   GC traits over the `mechanics-config` types.
4. Consumers of the schema (notably the lowerer, not yet
   implemented) depend on `mechanics-config` directly.

**Doesn't require a breaking release of `mechanics-core`** —
re-exports keep existing consumers working without code changes.

## Status

`mechanics-core` and `mechanics` are published and functional.
`mechanics-config` extraction is pending. Extraction should
happen before or alongside the connector client implementation;
it doesn't block the connector layer's design work but does
block its implementation.
