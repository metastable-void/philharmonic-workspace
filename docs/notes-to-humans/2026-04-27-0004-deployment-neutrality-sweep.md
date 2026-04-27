# Deployment-neutrality + framework-genericity sweep

**Author:** Claude Code · **Audience:** Yuka · **Date:** 2026-04-27 (Mon) JST evening

You asked, before going home, for an audit + cleanup of any
docs that assume:
- "our own SaaS-ish deployments" (URL shapes, process
  isolation models, HTTP boundaries)
- LLM-centered framing (the framework is general-purpose
  automation, not LLM-specific)

This is the result. Single commit landed at `74c82ce`.

## What I changed

### `ROADMAP.md`

- **§Phase 5 wave B (lowerer task list)** — the example URL
  was `<realm>.connector.our-domain.tld`. Changed to
  describe the URL as deployment-supplied configuration,
  with the subdomain shape labeled as one example among
  others.
- **§Phase 5 wave B (router task list)** — same
  reframing: the router fronts the connector services for
  one realm "configured by the deployment to listen on
  whichever URL the deployment chose (subdomain-per-realm,
  path-prefix-per-realm, host-header dispatch, etc. — the
  router doesn't prescribe a shape)".
- **§Phase 8 task list** — was prescribing
  `<tenant>.api.our-domain.tld/v1/...` and
  `admin.our-domain.tld/v1/...` directly. Rewrote as: the
  crate exposes a `RequestScope` trait that the deployment
  implements to map each request to either
  `Tenant(tenant_id)` or `Operator`. The implementation can
  read a subdomain, path prefix, TLS client-cert SAN/CN,
  fixed-tenant constant, or anything else — the framework
  is agnostic. Doc 10 §"Request routing" is referenced for
  the enumeration of common shapes.
- **§Phase 8 closing paragraph (new)** — added: "The crate
  ships as a library that exposes an `axum::Router` (or
  equivalent constructor) plus the trait surfaces it needs
  plugged in. Whether a deployment runs it as one process,
  splits it across many, or embeds it in-process for a
  single user is the deployment's choice; the framework
  does not prescribe."
- **§Phase 9 reference deployment** — was prescribing
  TLS certs for `*.api.our-domain.tld` etc. as if those
  were the canonical shape. Rewrote to label the reference
  deployment as "one example shape; chosen for end-to-end
  exercise of the crates, not as a prescription", and the
  TLS certs as "for whatever URL shape the reference
  deployment picks".
- **§Phase 9 chat-app flow** — rewrote to frame chat-app
  as "one example application" of the ephemeral-token
  pattern, alongside non-browser callers. The mechanism
  is generic; the flow exercises it; chat-app is not
  privileged.
- **§Phase 9 reference deployment closing** — added that
  the working workflow exercised end-to-end "is one
  LLM-driven flow for the agent-style use case — chosen
  because it stresses ephemeral tokens, instance-scope
  permission enforcement, and per-step credential
  decryption — but the framework is general-purpose
  workflow orchestration; non-LLM workflows are equally
  supported".

### [`docs/design/10-api-layer.md`](../design/10-api-layer.md)

- **§"Co-location with workflow engine"** → renamed to
  **§"Hosting the workflow engine"**. Was framed as "API
  processes hold the engine" — sounded like a deployment
  prescription. Reworked to: this is a *crate boundary*
  statement, not a deployment-shape statement; whatever
  process hosts `philharmonic-api` also holds the engine,
  and "whether to run the API crate as one process type,
  split it across many, embed it in-process for a single
  user, or run it inside another binary entirely is a
  deployment choice the crate does not prescribe".
- The "API processes need X" bullets are now "Whatever
  runs the API crate needs X".

### [`docs/design/11-security-and-cryptography.md`](../design/11-security-and-cryptography.md)

- **§"Where keys live"** — was saying "Loaded by API
  processes (which host both the API layer and the lowerer
  in v1)". Changed to "Loaded by whichever process hosts
  the lowerer (the same process that hosts the API crate
  when the two are co-located, which is the typical
  shape)". Same change for the other key-location bullets.
  Removes the "v1 = one binary" framing without losing the
  practical recommendation.

## What I deliberately left alone

These are already framework-correct and don't leak SaaS or
LLM-centrism:

- **`docs/design/01-project-overview.md`** — gold standard.
  Already explicitly says "designed as a generic framework
  — not a SaaS product — usable across a range of
  deployment shapes" and lists single-user self-hosted,
  single-tenant backend, multi-tenant SaaS, research
  platform, and scheduled-job runner as equal shapes. The
  §"Deployment topology (example shapes)" block does
  exactly the labeling we want. URL examples there are
  presented as "kinds of shapes a deployment might take.
  None of them is privileged".
- **`docs/design/02-design-principles.md`** §105+ — the
  framework principle "**LLM connectors are not
  first-class citizens**" is already stated explicitly.
  No code path, vocabulary, or entity kind is
  LLM-specific. This is the canonical place for that
  framing and it's solid.
- **`docs/design/09-policy-and-tenancy.md`** §26-28 — lists
  multi-tenant SaaS as one of several supported shapes
  alongside single-tenant backends, research platforms,
  and self-hosted installs. §431 lists "browser-based chat
  applications, scheduled job runners, partner
  integrations, and CLI tools" as supported — chat is one
  of four explicit shapes.
- **`docs/design/10-api-layer.md`** §44-§75 ("Request
  routing") — already says "topology is a deployment
  concern, not a framework requirement" and enumerates
  subdomain / path-prefix / single-tenant / mTLS / "anything
  else" as equally supported. This is the gold-standard
  framing for the rest of the docs.
- **`docs/design/12-deferred-decisions.md`** §"Tool calling
  in LLM connectors" / §"Streaming LLM responses" — these
  are deferred features for the LLM connector specifically,
  framed as "one connector category among many". Doesn't
  leak.
- **`docs/design/14-open-questions.md`** §"Workflow
  authoring patterns documentation" — references a chat-app
  consumer in the exploratory section. Acceptable framing
  (not a hard prescription, just an exploratory note).
- **`docs/design/15-v1-scope.md`** §480 — "the chat-app
  end-to-end path" as a test target for ordering rationale.
  Acceptable shorthand.
- **`README.md`** — only mentions LLM in the
  Claude/Codex-workflow context (e.g. "LLM-written
  summary"). Not framework framing.
- **`CONTRIBUTING.md`** — same; LLM mentions are about how
  Claude/Codex read the workspace, not framework
  positioning.

## What might still need revisiting

A few smaller framing notes worth thinking about, but not
urgent:

- **The phrase "tenant's own application"** appears in
  several places (e.g. doc 09 §422). It's correct in a
  multi-tenant framing where each tenant runs their own
  app on top of the framework. But it does subtly imply
  "tenant = customer of the deployer". For now this is
  consistent with multi-tenancy framework idioms and I
  haven't touched it. If you want a more neutral term
  ("each tenant's downstream consumer"? "the
  tenant-side caller"?), say so and I'll do a sweep.
- **`docs/design/07-workflow-orchestration.md`** §384
  mentions "browser-driven use cases (like the chat-app
  pattern)". Currently labeled as one example in
  parentheses; not prescriptive. Leaves as-is unless you
  want it weakened further.
- **`philharmonic-connector-impl-llm-openai-compat`** etc.
  carry per-implementation domain knowledge by design;
  their crate names mention LLM, which is correct (they
  ARE LLM connectors, not first-class framework
  components). No change needed.

If you want either of the first two pulled into a follow-up
sweep, flag them in the morning and I'll do another pass.

## Commit

Single commit `74c82ce` ("docs: deployment-neutrality sweep —
ROADMAP/§Phase 5/8/9 + doc 10 + doc 11"). 3 files, 122
insertions / 56 deletions. Pushed to origin.
