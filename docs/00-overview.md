# Philharmonic — Overview

Philharmonic is a workflow orchestration system built as a family of
Rust crates. This document is the conceptual entry point: what the
system is, what mental model to bring to it, and where to read for
specifics. The component-by-component breakdown lives in
`02-00-components.md`; this overview stays at the level of "what is
this and why does it exist."

## What the system does

A Philharmonic deployment runs JavaScript-based workflows. A workflow
is defined by a script and a configuration; an instance of a workflow
is a running execution that threads state across multiple step
invocations. Each step takes the current context plus a step-specific
input, runs JS in a sandboxed runtime, and returns updated context
plus an output. The system persists every state transition and every
step's input/output as an append-only history.

The intended deployment shape: a small number of orchestrator
processes talking to a clustered MySQL-family database for
persistence, and a horizontally-scaled fleet of stateless JavaScript
worker nodes for execution. Connector services run alongside the
worker fleet to provide HTTP-mediated capabilities (external API
calls, LLM access) that scripts can use under per-step authorization.
The orchestrator does not embed the JS engine; it reaches the workers
over HTTP. This separation lets the storage and compute sides scale
independently.

## The two pillars

The system has two foundational subsystems, deliberately decoupled,
plus a connector layer adjacent to the execution side.

**Storage substrate.** Append-only, content-addressed,
entity-centric storage. Every workflow template, every workflow
instance, every step record, every byte of content the system needs
to persist sits on this substrate. The substrate doesn't know about
workflows or any other domain — it provides storage primitives, and
consumers layer their semantics on top.

**Execution substrate.** JavaScript jobs in stateless Boa runtimes,
exposed as an HTTP service. Each job takes a script, a JSON argument,
and a host-side configuration; runs the script's default export;
returns the result as JSON or an error string. The service is
horizontally scalable: any worker can run any job, no state persists
across jobs, no worker affinity is required.

**Connector layer.** HTTP-accessible services that scripts reach
through for capabilities they can't perform locally — external HTTP
requests, LLM access, future connector kinds. The capabilities
available to any given script are determined by the host-side config,
which the orchestrator produces per-step from the workflow template's
abstract configuration. Connector authorization is signed-token-based:
the config carries short-lived tokens that connector services verify
on each request.

The orchestration layer ties the three together. It reads workflow
state from the storage substrate, lowers the template's abstract
config to a concrete runtime config (with tokens minted for the
current step), dispatches jobs to the execution substrate, and writes
results back to storage. The orchestration layer is the only one that
depends on all three.

## How the pieces connect

A workflow runs roughly like this:

1. A caller creates an instance of a template through the
   orchestration layer. The orchestrator mints an identity, creates
   the instance entity in the storage substrate, and writes the
   initial revision (status: pending, context: null,
   args: caller-supplied).

2. The caller (or a scheduler, or some external trigger) invokes
   `execute_step` on the orchestrator with the instance ID and a
   step-specific input.

3. The orchestrator reads the instance's latest revision (for current
   context), reads the template (for script and abstract config), and
   passes the abstract config to the lowerer.

4. The lowerer produces a concrete runtime config with capability
   tokens minted for this specific step, scoped to the connector
   services this template is allowed to use.

5. The orchestrator assembles the job — script source, combined
   `{context, args, input}` argument, concrete config — and sends it
   to a worker over HTTP.

6. The worker runs the script in an isolated Boa realm. When the
   script calls a configured capability (e.g., fetches an external
   URL), the runtime issues an HTTP request to the connector service
   named in the config, attaching the signed token for verification.

7. The worker returns the result (a JSON value with `context` and
   `output` fields, optionally `done: true`) or an error string.

8. The orchestrator validates the result shape, content-addresses the
   new context and the output, appends a new revision to the instance
   (with the appropriate next status), and creates a step record
   entity capturing the input, output, and outcome.

9. The cycle repeats until the instance reaches a terminal status
   (completed, failed, or cancelled).

The storage substrate doesn't know about workflows. The execution
substrate doesn't know about persistence. The connector services don't
know about workflows. The orchestration layer is where the meaning
lives, and it's the only layer that depends on all of them.

## Design philosophy

A few commitments shape decisions across the entire system. They are
covered in detail in `01-principles.md` but summarized here.

**Append-only.** Storage operations add data; they never modify or
delete. Soft-delete is expressed as a new revision with a deletion
scalar, not as removal. This collapses concurrency concerns and gives
every entity a complete audit trail by default.

**Content-addressed.** Anything that can be deduplicated and named by
content is stored as bytes keyed by SHA-256 hash. JSON content is
canonicalized (RFC 8785, JCS) before hashing so that semantically-
equal JSON produces equal hashes regardless of key order or whitespace.

**Backend-agnostic interfaces.** The storage substrate is defined as
traits, not as a concrete implementation. The orchestrator depends on
abstractions (storage trait, executor trait, lowerer trait), not on
specific implementations. Multiple backends can coexist; consumers
choose at construction time.

**Vocabulary collapses misuse paths.** Types in the cornerstone are
deliberately narrow. There is no `ScalarType::Str` because strings
should live in content blobs, in `i64` enum encodings, or in entity
references — never as ad-hoc scalar columns. The substrate refuses to
bless patterns that lead to bad designs.

**LCD MySQL.** The SQL implementation uses only features common to
MySQL 8, MariaDB, Aurora MySQL, and TiDB. No JSON columns, no
vendor-specific operators, no declared foreign keys. This makes
deployment portable across the MySQL-compatible ecosystem.

**Capability via signed tokens.** The orchestrator authorizes each
step's connector access by minting short-lived signed tokens for the
specific capabilities the template needs. Connector services verify
tokens on each request; policy enforcement happens at token-mint time
upstream, not at runtime in the dispatch path.

## What this system is not

A few things worth being explicit about.

**Not a general-purpose database.** The substrate is shaped
specifically for entity-centric, append-only, revision-logged data. It
would be the wrong tool for relational analytics, full-text search, or
high-throughput key-value workloads.

**Not a JavaScript runtime in the orchestrator.** The orchestrator
process does not execute JS. It coordinates persistence and dispatches
jobs to the executor service. Embedding a JS engine in the
orchestrator would tie scaling of state management to scaling of
compute, which the two-pillar separation deliberately avoids.

**Not a message queue or event bus.** Workflows are step-driven by
explicit calls (from API requests, schedulers, etc.). The system does
not provide messaging primitives. If a deployment needs queuing
between external triggers and workflow execution, that's an
infrastructure concern handled outside Philharmonic.

**Not a deployment framework.** Crate consumers assemble their own
binaries and choose their own deployment topology. Philharmonic
provides the libraries; how to package, configure, monitor, and run
the resulting services is the deployer's responsibility.

## Where to read next

The remaining documents in this directory:

- `01-principles.md` — design commitments shared across all subsystems.
- `02-00-components.md` — concrete inventory of components and crates.
- `02-01-cornerstone.md` — the cornerstone vocabulary crate.
- `02-02-storage.md` — storage substrate design.
- `02-03-execution.md` — execution substrate and JS contract.
- `02-04-connectors.md` — connector layer and capability tokens.
- `02-05-workflow.md` — orchestration layer design.
- `03-boundaries.md` — what each layer doesn't know about.
- `04-deferred.md` — explicitly out-of-scope features and why.
- `05-conventions.md` — workspace-wide practices.

For a new contributor, the natural reading path is in this order. For
someone evaluating the system or looking for a specific concern, the
list above maps documents to topics.
