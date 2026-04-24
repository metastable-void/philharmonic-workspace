# Philharmonic — Project Overview

## What it is

Philharmonic is a workflow orchestration system built as a family of
Rust crates. It runs JavaScript-based workflows against persistent
state, with JavaScript execution happening in sandboxed Boa runtimes
and connector services mediating all external I/O under per-step
authorization.

The crates are released publicly under `Apache-2.0 OR MPL-2.0` and
designed to be usable across a range of deployment shapes —
single-user self-hosted, single-tenant application backend,
multi-tenant SaaS, research platform, scheduled-job runner, and
others. The layered design (see `02-design-principles.md`) lets
consumers pick up only the layers they need: the storage substrate
and execution substrate are each usable standalone, and the policy
layer is optional for deployments that don't require multi-tenancy.

Our own deployment (Menhera®) happens to be multi-tenant SaaS on
our own infrastructure, and some concrete shapes described in these
docs reflect that. Deployment-specific material is fenced under
sections explicitly labeled as such; crate-level design does not
depend on it.

## The architectural picture

Three subsystems, deliberately decoupled:

**Storage substrate.** Append-only, content-addressed,
entity-centric storage backed by MySQL-family databases. Workflow
templates, instances, step records, per-tenant endpoint configs,
and content blobs all live here. The substrate knows nothing about
the domain; it provides storage primitives and is reusable outside
Philharmonic.

**Execution substrate.** JavaScript jobs in stateless Boa runtimes,
exposed as an HTTP service. Workers are fungible and horizontally
scalable; no cross-job state. Scripts use a defined host-function
API to call out to connectors, otherwise they're pure compute. The
executor is a general-purpose JS-job runner and is independent of
the rest of the stack.

**Connector layer.** HTTP-accessible services that scripts reach
through for all external I/O — LLM calls, HTTP requests, SQL,
email, vector search, and anything else a consumer builds
implementations for. Connector services are deployed per "realm"
(isolation domain) and hold per-realm private keys for decrypting
per-step configuration. The executor never sees plaintext
credentials or sensitive URLs.

The **workflow layer** orchestrates the three: reads state from
storage, lowers the template's abstract configuration to a concrete
per-step configuration (with tokens and encrypted payloads), ships
the job to the executor, records the result.

The **policy layer** sits alongside the workflow layer for
deployments that need multi-tenancy, per-tenant credentials, roles,
and token-minting. Single-tenant or single-user deployments may
skip it or replace it with something simpler; the workflow layer
receives the relevant context opaquely and does not require the
full policy layer to be present.

## How a workflow runs

The description below uses the full set of concepts — tenants,
authorization, lowering. In deployments that don't include the
policy layer, tenant-scoped resolution degenerates to a trivial
case and authorization is whatever the deployment decides; the
workflow layer itself receives a `SubjectContext` opaquely.

1. An authorized author publishes a workflow **template**: a JS
   script plus an abstract configuration that maps the script's
   local endpoint names to per-tenant `TenantEndpointConfig`
   entity UUIDs.
2. Via the API, a caller creates an **instance** of the template
   with caller-supplied arguments. The orchestrator writes the
   instance to storage with status `Pending`.
3. Via the API, a caller (or a scheduler) invokes `execute_step`
   with the instance ID and per-step input.
4. The orchestrator reads the instance's current state and the
   template's abstract config, then asks the **lowerer** to produce
   a concrete configuration for this step. For each endpoint name
   the script references, the lowerer fetches the referenced
   `TenantEndpointConfig` by UUID, decrypts its encrypted blob
   with the substrate credential key, re-encrypts the byte-
   identical plaintext to the target realm's KEM public key,
   mints a signed authorization token binding the payload hash
   to the step context, and returns the concrete config.
5. The orchestrator ships a job to the execution substrate: the
   script source, the assembled `{context, args, input, subject}`
   argument, the concrete config.
6. The worker runs the script in an isolated Boa realm. When the
   script calls a configured capability, the runtime POSTs to the
   connector service with the token and encrypted payload.
7. The connector service verifies the token, decrypts the payload
   to obtain the implementation name and per-call configuration
   (including credentials), dispatches to the named implementation,
   which makes the actual external call and returns the response.
8. The script returns `{context, output}` (or `{context, output,
   done: true}`).
9. The orchestrator validates the result, records a step record,
   appends a new instance revision with the next status
   (`Running`, `Completed`, or `Failed`).
10. The cycle repeats until the instance reaches a terminal status.

## Our own deployment topology

This section describes the production shape Menhera® runs. It is
not a requirement of any crate; consumers deploy however suits
them. Other files in this directory should reference this section
rather than re-describe the topology.

Our deployment targets our own infrastructure (on-premises or
IaaS), not arbitrary cloud SaaS. The shape looks like:

- `https://<tenant>.app.our-domain.tld/` — Web UI, tenant-scoped
  by subdomain.
- `https://<tenant>.api.our-domain.tld/` — API, same scope.
- `https://<realm>.connector.our-domain.tld/` — connector router
  per realm, forwarding to connector services in that realm.

Wildcard HTTPS certificates handle the subdomains. The tenant-in-
subdomain pattern gives origin-level isolation in browsers, which
matters for browser-delivered ephemeral tokens.

Connector services: one static binary per realm, containing all
implementations supported in that realm. Horizontally scaled
behind the realm's connector router.

Executors: horizontally scaled fleet of `mechanics` worker nodes.
Load balancer in front; orchestrator talks to the fleet via the
LB.

Storage: MySQL-family DB (MySQL 8, MariaDB, Aurora MySQL, or
TiDB). Shared across the API and orchestrator tiers.

Single region, single AZ (or multi-AZ within region, as
operationally appropriate).

Nothing above is baked into the crates. Subdomain-based routing,
per-realm connector binaries, the specific MySQL variant, and the
choice of on-premises hosting are all operational choices.
Consumers may route tenants by path prefix or header, collapse
realms into one binary, swap in a different storage backend once
`philharmonic-store` has additional implementations, run the whole
thing in-process for a single user, and so on.

## What this system is not

- **Not a general-purpose database.** The substrate is shaped for
  entity-centric append-only data.
- **Not a JavaScript runtime inside the orchestrator.** JS runs on
  separate worker nodes, reached over HTTP.
- **Not a message queue or event bus.** Workflow steps are
  triggered by explicit calls.
- **Not a deployment framework.** The crates are libraries; the
  deployment model is the consumer's responsibility.
- **Not a tool-calling agentic platform.** LLM connectors use
  structured output only; agentic loops are composed in JavaScript
  by the workflow author.
- **Not an LLM-first or AI-first system.** LLM connectors are one
  connector implementation among others and are not a first-class
  citizen of the architecture. Consumers may build AI-heavy
  applications on top, but the system stays intentionally
  non-AI-centric. Using LLMs as structured-output generators
  naturally follows from this stance.

## Status summary

- Storage substrate: substantially complete and published.
- Execution substrate: `mechanics-core` published; `mechanics` HTTP
  service published; `mechanics-config` published.
- Workflow layer: `philharmonic-workflow` 0.1.0 published
  2026-04-22.
- Policy layer: `philharmonic-policy` 0.1.0 published
  2026-04-22.
- Connector layer: triangle (client / router / service) 0.1.0
  published 2026-04-23; `philharmonic-connector-impl-api`
  0.1.0 + `philharmonic-connector-impl-http-forward` 0.1.0 +
  `philharmonic-connector-impl-llm-openai-compat` 0.1.0 all
  published 2026-04-24 (Phase 6 complete). Remaining
  per-implementation crates land in Phase 7.
- API layer: designed; not yet implemented (Phase 8+).
