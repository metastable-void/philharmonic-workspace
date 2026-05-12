# Philharmonic compared

Philharmonic is not a Zapier clone, not a Temporal clone, and not
an AI-agent framework. It is a secure workflow substrate for
running JavaScript workflows against persistent state, with all
external I/O mediated by tenant-scoped connectors.

## At a glance

| System type | Optimized for | Philharmonic difference |
|---|---|---|
| Zapier / n8n | broad app automation | fewer built-in app connectors, but stronger tenant-scoped capability isolation |
| Temporal | durable distributed workflows | lighter step execution model, stronger sandboxed JS + connector boundary |
| LangGraph / agent frameworks | agentic LLM applications | LLMs are ordinary connectors; agent loops are explicit JS |
| AWS Step Functions | cloud-native state machines | framework-level portability and code-first JS workflows |
| Serverless functions | arbitrary backend code | constrained execution, persistent context, content-addressed records, no direct secrets |

## Why generic HTTP integrations matter

Many third-party integrations are HTTP APIs. In Philharmonic
those are tenant-scoped endpoint configurations rather than new
executor privileges or hardcoded secrets — the same lowering and
authorization pipeline applies whether the connector is a typed
LLM client or a generic HTTP forwarder. See
[Connector architecture](08-connector-architecture.md).

## Security model

The executor runs workflow code but does not own credentials.
Connectors receive encrypted per-step configuration and
short-lived authorization tokens bound to the step's payload
hash. See [Security and cryptography](11-security-and-cryptography.md)
and [Policy and tenancy](09-policy-and-tenancy.md).

## AI workflows without AI lock-in

LLM, embedding, vector search, SQL, email, and HTTP are all
connector implementations. The framework does not privilege LLMs
architecturally; agentic patterns are composed in JavaScript by
the workflow author, with the LLM connector acting as a
structured-output generator (see
[Project overview → What this system is not](01-project-overview.md#what-this-system-is-not)).

## When to choose Philharmonic

Choose it when you need tenant-aware, auditable,
connector-mediated workflows over persistent state — especially
when secrets must not be visible to workflow code, and when AI
capabilities should compose alongside SQL, HTTP, email, and
vector search rather than dominate the architecture.

## When not to choose Philharmonic

Do not choose it only because you want the largest marketplace
of prebuilt SaaS integrations, or a fully no-code business
automation product today.
