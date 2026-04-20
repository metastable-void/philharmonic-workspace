# Philharmonic — Execution Substrate

This document covers the execution side of the system: the
JavaScript runtime hosted by `mechanics-core`, the HTTP service
`mechanics` that exposes it, and the contract between them and the
orchestration layer that calls them.

The execution substrate is the second of the system's two pillars.
It runs JavaScript jobs and returns results. It does not persist
anything, does not coordinate workflows, does not know about
storage. It exists to be a stateless compute resource that the
orchestrator dispatches work to.

## What the substrate is

`mechanics-core` is a Rust library wrapping the Boa JavaScript
engine. It provides a worker pool of Boa runtimes and an API for
submitting jobs to them. Each job is a complete unit of work: an
ECMAScript module's source code, a JSON argument, and a host-side
configuration. The library runs the module's default export against
the argument and returns the result as JSON, or returns an error
string if execution failed.

`mechanics` is the HTTP service exposing `mechanics-core` over the
network. It accepts job requests as HTTP POST bodies, runs them
through the worker pool, and returns the results as HTTP responses.
This is what gets deployed on worker nodes; the orchestrator talks
to one or more `mechanics` instances over HTTP rather than embedding
the engine directly.

The split between library and service is the same shape as the
split between the storage substrate's trait crate and SQL backend
crate: a reusable core, plus a deployment-specific wrapper. A
deployment that wanted to run JS jobs in-process (no HTTP, no
network) could depend on `mechanics-core` directly. A deployment
that wants the standard horizontal-scaling model uses `mechanics`
the binary.

## The job contract

Every job has the same shape: a JavaScript module, a JSON argument,
a host-side configuration. The orchestrator assembles these from
its own state (template script, runtime context plus per-step
input, template config) and submits them; the executor runs them
and returns the result.

The module's default export is an async function with a single
parameter:

```javascript
export default async function main({context, args, input}) {
    // ... script logic ...
    return {context: newContext, output: stepOutput};
}
```

The parameter is destructured into three named fields. The script
is expected to handle each:

- **`context`** is the workflow's evolving state — the substrate
  reads the latest revision's context content blob and passes its
  parsed JSON value here. On the first invocation of an instance,
  context may be `null` or an empty object (the orchestrator's
  choice; the script must handle both).

- **`args`** is the per-instance argument supplied at workflow
  creation time and immutable thereafter. The orchestrator stores
  it as a content blob on the instance and passes the same value on
  every step invocation.

- **`input`** is the per-step argument from the orchestrator's
  caller (an API request, a scheduler tick, etc.). Different on
  every step.

The script's return value is also a single object with two named
fields:

- **`context`** is the new state to persist on the instance's next
  revision. The orchestrator content-addresses this and writes it
  to the instance's `context` content slot.

- **`output`** is the step's result, persisted on the step record's
  `output` content slot. Whatever shape the script produces; the
  substrate doesn't validate it.

This contract is the entire interface between the script and the
orchestrator. No other names are reserved; no other return-shape
fields are interpreted. The script's `globalThis` is fresh per job;
no cross-job state survives.

## What the executor doesn't validate

The executor runs the script and returns whatever the script
returned, faithfully. It does not validate that the return value
has the `{context, output}` shape. A script that returns `42` will
have `42` returned to the orchestrator, which will reject it as
malformed and record a step failure. A script that returns
`{foo: "bar"}` likewise.

The validation lives in the orchestrator because the orchestrator
is what defines the contract. The executor is generic compute — it
doesn't know that the JSON it processes is supposed to follow a
particular shape. A different orchestrator using mechanics for
different work would have a different contract; the executor
shouldn't bake any specific contract into its API.

This puts a small validation cost on every step result in the
orchestrator. The cost is real but small (one shape check per
step), and it's where the cost belongs.

## Errors as strings

When a script throws an exception or returns from an exception
handler with a rejected promise, the executor catches it,
stringifies it, and returns it as an error to the orchestrator.

The stringification happens at the JS realm boundary because
JavaScript exceptions can be anything: an `Error` object with a
message and stack, a string, a number, an arbitrary object that
doesn't even derive from `Error`. The only universal operation
across all of these is "convert to a string." The executor performs
that conversion and reports the resulting string.

This means script-level errors are always strings, never structured
JSON. The orchestrator's error-handling logic doesn't need to parse
error contents; it stores the string in the step record's `error`
content slot (wrapped in a small JSON envelope for storage
consistency) and moves on.

The trade is real: callers can't introspect script errors
programmatically. If a script wants to communicate structured
failure information, it has to *return* that information as part of
its `output`, not throw it. This is intentional — the system treats
exceptions as "something unexpected happened" and structured
returns as "the step completed with this result." Mixing the two
would invite scripts to throw objects-with-codes that callers parse
and act on, which couples error contents to caller logic. Better to
keep exceptions opaque and force structured failures through the
return path.

What this means for diagnosis: the error string usually contains
enough information for a human reader (Boa's exception stringifier
includes the message and often the JS stack trace). Tooling can
read step records' error blobs and surface them. Programmatic
recovery from script errors is not a supported pattern; if a script
might "fail recoverably," it returns a structured success with a
failure indicator.

## Stateless execution

Each job runs in a fresh Boa realm. `globalThis` mutations don't
persist across jobs. In-process caches don't either. Workers don't
have affinity to particular workflows; any worker can run any job.

This is the executor's most important property and the reason for
several of its design choices.

**Why isolated realms.** Boa supports multiple realms per `Context`,
and creating a fresh realm per job is cheap relative to script
execution. The isolation prevents leaks: a script that mutates
`globalThis.foo` doesn't affect the next job, even if the next job
runs on the same worker thread. Test reproducibility, security
boundaries, and operational simplicity all follow.

**Why no caching.** Caching scripts (so re-execution avoids
re-parsing) would be a clear performance win, but it requires
cross-job state. The decision is to push caching outside the worker
process if needed. Most jobs are small enough that re-parsing is
fast; if a deployment finds parsing dominates, a separate caching
layer (Redis-backed, content-hash-keyed) sits between the
orchestrator and the executor.

**Why no warm pools.** A "warm" runtime (with a script pre-loaded
and ready) would speed up repeated executions of the same script,
but again requires cross-job state. The system is designed for
horizontal scaling: more workers, not warmer workers.

**Why fungible workers.** Any worker can run any job. The
orchestrator picks a worker (round-robin, least-connections, or
whatever the load balancer in front of the worker fleet decides);
no routing based on workflow ID or script content is needed. This
is what makes "more workers" the answer to scaling.

The cost: per-job overhead is higher than it would be with
stateful workers (parsing on every job, no warm caches). The
benefit: scaling is unbounded and operationally trivial.

## The HTTP envelope

The `mechanics` service accepts jobs over HTTP. The exact wire
format is owned by the `mechanics` crate, not specified by this
document, but the shape is approximately:

**Request:** A POST with a JSON body containing `module_source`
(string), `arg` (any JSON value), and `config` (any JSON value).

**Response:** On success, a JSON value (the script's return value).
On script error, an error response with the stringified exception.
On infrastructure error (timeout, malformed request), an HTTP error
status with appropriate context.

The orchestrator's `StepExecutor` implementation translates between
the orchestrator's domain model and the HTTP wire format. The
executor itself is unaware that it's being driven by an
orchestrator; it just runs jobs.

## Configuration as opaque pass-through

The `config` parameter on a job is a JSON value that the executor
passes through to the JavaScript runtime's host environment. It
controls what the script is *allowed* to do — which endpoints it
can reach, which credentials are available to it, what the runtime
configuration is.

The orchestrator doesn't interpret the config. It reads the
template's `config` content blob, deserializes it as JSON, and
passes it to the executor. The executor (or, more precisely, the
mechanics service) uses it to configure the host environment for
the job.

The shape of the config is owned by `mechanics-core`. If a future
version changes the config schema, templates authored against the
old schema may stop working — the script's host setup will fail or
behave differently. The design accepts this: configs are pinned to
the `mechanics-core` version they were authored against, and major
upgrades may require re-authoring.

The orchestrator could in principle validate configs against the
expected schema before sending them, but doing so would couple the
orchestrator to the executor's config schema and require updates
every time the schema changes. The current design defers
validation: a malformed config produces an executor error on first
use, which the orchestrator records as a failed step.

## Endpoints and what scripts can reach

Scripts can call into a small set of host-provided functions for
operations that JavaScript can't do natively or that the host
needs to mediate: random number generation, time, base64/base32/hex
encoding, network requests to allowlisted endpoints, and
LLM-style endpoints (when configured).

The exact set of host functions is `mechanics-core`'s API surface,
not this document's. The principle is that scripts get host
functions for cross-cutting capabilities (randomness, encoding) and
for connector-mediated I/O (HTTP, LLMs), and the config controls
which connectors are available for any given job.

This is the layer where the connector concept from the original
design doc lives. A script asks the host to fetch a URL; the host
checks the config's allowlist; if allowed, the request goes
through. Connectors are routed by the executor's host environment,
not by the orchestrator.

The orchestrator is therefore unaware of connectors. It assembles
jobs and ships them off; whatever connectors the job needs are
configured by the template's config blob and resolved by the
executor's host. This keeps the orchestrator's surface focused on
state management.

## Determinism is deferred

A natural extension to the executor would be to make jobs
deterministic: seed `Math.random` from a job parameter, freeze
`Date.now`, intercept all external I/O. Combined with the
substrate's append-only revision log, this would enable replay:
re-execute a workflow from any past state and get identical
results.

This is not done. The current executor is non-deterministic in
practice — `Math.random` returns fresh entropy, `Date.now` returns
the wall clock, network calls hit the actual network. Replays
won't reproduce.

The deferral is deliberate. Determinism requires interceptable
host functions for every source of non-determinism (clock, RNG,
all I/O), and getting that right is more work than the current
needs justify. The substrate's append-only log captures every
state transition regardless of determinism, so replay isn't
needed for diagnosis — the history is already there.

If determinism becomes important later, the path is clear:
`mechanics-core` adds seeded RNG and frozen time as job
parameters, the orchestrator records those parameters in the
step record, and replay re-runs jobs with the recorded seeds.
The substrate doesn't need to change. The executor's API gains
fields for `rand_seed: u64` and `time_freeze: Option<UnixMillis>`
or similar; existing callers can ignore them.

The deferral is documented in `07-deferred.md` along with other
similarly-scoped not-yet-done features.

## Multi-worker concurrency

A `mechanics-core` instance hosts a worker pool. Multiple jobs run
in parallel across the pool's workers. Within a single worker, one
job runs at a time (Boa is not internally parallel; one realm
processes one script at a time).

Across multiple `mechanics` HTTP service instances, scaling is
horizontal: each instance has its own pool, and the orchestrator's
load balancer distributes work across instances. There is no
coordination between instances; each is independent.

This means the system's effective parallelism is `(instances)
× (workers per instance)`. Tuning either dimension is an
operational decision based on workload and host capacity.

The orchestrator doesn't know how many instances or workers exist.
It sees the executor as a single endpoint (whatever the load
balancer presents) and submits jobs against it. Scaling decisions
are made by adding workers behind the load balancer, not by
changing orchestrator code.

## What the executor doesn't know

The executor processes jobs. It doesn't know:

**What workflow a job belongs to.** Jobs arrive with a script, an
arg, and a config. There's no instance ID, no template ID, no
correlation with prior or future jobs. The executor's stateless
property requires this — knowing which workflow a job belongs to
would imply state that the executor would have to maintain.

**What the orchestrator does with results.** The executor returns
the result; the orchestrator decides what it means. Persistence,
state transitions, follow-up jobs — all happen in the orchestrator
without the executor's involvement.

**Whether two jobs are related.** Sequential jobs in the same
workflow look identical to the executor as parallel jobs in
different workflows. The orchestrator might issue them in order,
or it might issue them concurrently if the workflow doesn't
require strict sequencing; the executor doesn't care.

**What persistence the orchestrator uses.** The executor doesn't
talk to the substrate. It doesn't know whether the orchestrator is
backed by MySQL, an in-memory store, or anything else. It just
runs jobs.

The executor is, structurally, the simplest layer of the system:
input in, computation, output out, no memory of either. This is
what makes it horizontally scalable and what makes the
orchestrator's coordination logic clean (the orchestrator is the
only stateful actor in the workflow lifecycle).

## What the orchestrator doesn't know

Symmetrically, the orchestrator's view of the executor is narrow.

**It doesn't know which worker ran a job.** Whatever the load
balancer chose. There's no concept of "worker affinity" or "this
instance always runs jobs for tenant X."

**It doesn't know the runtime's internals.** Boa version, worker
count, memory limits, GC behavior — all opaque. The orchestrator
sees a network endpoint that accepts jobs and returns results.

**It doesn't know about Boa-specific errors.** When a script error
comes back, it's a string. The orchestrator doesn't try to parse
"is this a syntax error" vs. "did the script throw" — it records
the string and treats the step as failed.

**It doesn't know about host functions or endpoints.** The script
might call out to an LLM, fetch a URL, or do nothing but
arithmetic. The orchestrator sees only the script's final return
value (or its error). What the script did along the way is
visible only in the script's own logging or the executor's
logs — not in the orchestrator's data.

This boundary is the contract. Both sides hold up their end:
the executor runs jobs faithfully and returns results, the
orchestrator persists state and dispatches follow-up work. Neither
reaches into the other's internals.
