# API Layer

`philharmonic-api` is the HTTP API that tenant applications,
tenant admins, and deployment operators interact with. It holds
the `WorkflowEngine`, authenticates callers, enforces policy,
exposes CRUD on tenant resources, and mints ephemeral tokens for
tenant end users.

The crate doesn't exist yet. Design is settled (this document);
a single small open item — an atomic create-instance-and-mint
endpoint — is listed at the bottom.

## What the layer is responsible for

- HTTP transport for all tenant-facing operations.
- Authentication of callers (persistent principals with long-
  lived tokens; ephemeral subjects with ephemeral tokens).
- Authorization of requests against tenant-scoped permissions.
- Hosting the `WorkflowEngine`, including the `StepExecutor`
  (HTTP client to the mechanics fleet) and `ConfigLowerer` (the
  connector client) as plugged-in dependencies.
- Minting ephemeral tokens for tenant applications.
- CRUD on tenant-scoped resources: workflow templates,
  instances, endpoint configs, principals, roles, minting
  authorities, tenant settings.
- Audit recording for policy-relevant actions.
- Per-tenant rate limiting.
- At-rest encryption of submitted endpoint config JSON under
  the substrate credential key (SCK) before substrate storage;
  operator-visible decryption for principals with
  `endpoint:read_decrypted` permission.

What it doesn't do:

- Execute JavaScript (the mechanics executor does).
- Make connector calls (that happens inside script execution).
- Store data directly (delegates to the substrate via the
  `WorkflowEngine` and via direct queries on policy entities).
- Implement business logic beyond request translation and
  policy enforcement.

## Request routing

Topology is a **deployment concern, not a framework
requirement**. The `philharmonic-api` crate accepts a
`request → tenant-id` resolution from its deployment-supplied
configuration — whether that resolution reads a subdomain, a
path prefix, a TLS client-cert CN, a fixed tenant (for
single-tenant deployments), or something else entirely is up
to whoever builds the binary that hosts `philharmonic-api`.
The crate treats "what tenant does this request belong to?"
as an input, not a derivation.

Any of these shapes is supported:

- **Subdomain-per-tenant**: `<tenant>.api.<deployment-domain>`.
  Wildcard certificate, origin-level browser isolation.
- **Path-prefix-per-tenant**:
  `<deployment-domain>/t/<tenant>/…`. Single certificate, no
  origin-level browser isolation.
- **Single-tenant**: the resolver returns one fixed tenant ID
  regardless of request. Suitable for single-tenant
  application backends and self-hosted deployments.
- **Client-cert-per-tenant**: mTLS with the tenant ID
  encoded in the client-cert CN or SAN.
- Anything else a deployment wires up, as long as it produces
  a tenant ID by the time auth middleware runs.

Operator endpoints must likewise be distinguishable from
tenant requests by the resolver (a separate subdomain, a
reserved path prefix, a separate binary listening on a
separate port — the deployment's choice). The framework only
needs `"this request is operator-scoped"` vs. `"this request
is tenant-scoped, tenant=X"` as input; it is agnostic to how
that determination is made.

### Browser-embedded clients

Deployments that serve browser-embedded clients (e.g., chat-
app pattern where ephemeral tokens ride in in-browser API
calls) typically benefit from origin-level isolation between
tenants, which subdomain-per-tenant provides naturally and
path-prefix-per-tenant does not. Deployments that don't
serve browser-embedded clients are free to pick a simpler
shape. This is an informed deployment trade-off, not a
framework prescription.

## Authentication

Two authentication mechanisms for v1. No OAuth 2.0 / OIDC in v1;
no session-based authentication in v1 (the Web UI uses the same
API-token mechanism).

### Long-lived API tokens

Held by persistent principals (tenant admins, service accounts,
minting authorities). Opaque bearer credentials; SHA-256 hashed
in the corresponding `Principal` or `MintingAuthority` entity.

Format: `pht_<43-char base64url-encoded 32 random bytes, no
padding>` — 47 characters total including the prefix. Prefix
enables grep-based leak detection in logs and commit hooks.
Full specification in `09-policy-and-tenancy.md`.

Presentation: `Authorization: Bearer <token>` header.

Verification per request:
1. Extract token from header.
2. Check `pht_` prefix and length; reject malformed.
3. SHA-256 the full token (including prefix).
4. Look up the hash in the substrate (principals and minting
   authorities indexed by credential hash).
5. Check the principal/authority is not retired and its tenant
   is not suspended.
6. Attach the authenticated identity to the request context.

No authentication cache in v1 — the substrate lookup happens
on every request. If the hot-path cost becomes a bottleneck,
a short-TTL cache can be added later as an operational
optimization; not a v1 requirement.

### Ephemeral API tokens

Minted by minting authorities via the minting endpoint (below).
Bearer credentials; COSE_Sign1 signed by the API layer.

Format: COSE_Sign1 with the claims listed in
`09-policy-and-tenancy.md` (iss, exp, sub, tenant, authority,
authority_epoch, optional instance, permissions, injected
claims, kid).

Presentation: `Authorization: Bearer <COSE_Sign1>` header.

Verification per request:
1. Parse COSE_Sign1 structure.
2. Look up signing key by `kid`.
3. Verify signature.
4. Check `exp` not passed.
5. Look up the minting authority by `authority` ID.
6. Check the authority is not retired and its tenant is not
   suspended.
7. Check the authority's current `epoch` equals
   `authority_epoch` claim. (Epoch lookup can be cached with
   short TTL; mismatch after TTL expiry means one stale
   acceptance, acceptable given epoch bumps are incident-level
   operations.)
8. Check `tenant` claim matches the request's tenant subdomain.
9. If `instance` claim present, check subsequent operations
   target that instance.
10. Attach the authenticated context — subject identifier,
    tenant, minting authority, permissions, injected claims —
    to the request.

### Distinguishing authentication contexts

The authenticated context carries enough structure to distinguish
long-lived from ephemeral authentication downstream:

```rust
enum AuthContext {
    Principal {
        principal_id: EntityId<Principal>,
        tenant_id: EntityId<Tenant>,
    },
    Ephemeral {
        subject: String,
        tenant_id: EntityId<Tenant>,
        authority_id: EntityId<MintingAuthority>,
        permissions: Vec<String>,
        injected_claims: JsonValue,
        instance_scope: Option<EntityId<WorkflowInstance>>,
    },
}
```

Endpoint handlers can require one or the other:

- Tenant admin endpoints (CRUD on credentials, roles, principals,
  minting authorities) require `Principal` — ephemeral subjects
  can't mint tokens, modify credentials, etc.
- Workflow execution endpoints (`execute_step`) accept either
  context; the subject context (if ephemeral) flows to the
  workflow script.
- The minting endpoint requires `Principal` where the principal
  is a `MintingAuthority` (minting authorities authenticate as
  themselves to mint tokens).

## Authorization

### Permission enforcement

Every endpoint declares the permission atom(s) it requires. The
API layer checks the authenticated context against the required
permissions before dispatching to the handler.

For `Principal` contexts: look up role memberships within the
tenant, check whether any role grants the required permission
(see `09-policy-and-tenancy.md` for the evaluation model).

For `Ephemeral` contexts: check the `permissions` claim list
directly. The claims were already clipped to the minting
authority's envelope at mint time; no further consultation
needed.

### Instance-scope enforcement

An ephemeral token with an `instance` claim restricts which
instance it can operate on. The API checks: any endpoint taking
an instance ID rejects the request if the claim's instance
doesn't match. This is enforced at the API layer, not delegated
to the workflow engine.

### Tenant scope enforcement

Every endpoint is tenant-scoped via the subdomain. The
authenticated context's tenant must match the subdomain's
tenant. Cross-tenant requests fail at the authentication step.

Deployment-operator endpoints (outside any tenant subdomain)
require a deployment-admin principal — a special principal kind
or a tenant designated as the operator tenant.

## Endpoint surface

Organized by concern. URL paths use `/v1/` prefix (URL-path
versioning for v1).

### Workflow management

Require `Principal` authentication unless noted.

- `POST /v1/workflows/templates` — create workflow template.
  Requires `workflow:template_create`. Request body:
  `{script_source, abstract_config, display_name}`. The
  abstract_config is a `{script_name: config_uuid}` map; the
  API validates that every referenced `config_uuid` exists
  within the tenant and is not retired.
- `GET /v1/workflows/templates` — list templates in the tenant.
  Requires `workflow:template_read`.
- `GET /v1/workflows/templates/{id}` — read template.
  Requires `workflow:template_read`.
- `PATCH /v1/workflows/templates/{id}` — append a new revision
  to an existing template (new script source, new abstract
  config, or both). Requires `workflow:template_create`.
  Same template UUID; new revision. Running instances stay
  bound to their pinned template revision; new instances use
  the latest.
- `POST /v1/workflows/templates/{id}/retire` — retire template.
  Requires `workflow:template_retire`.
- `POST /v1/workflows/instances` — create instance. Requires
  `workflow:instance_create`. Accepts a template reference
  and args.
- `GET /v1/workflows/instances` — list instances. Requires
  `workflow:instance_read`.
- `GET /v1/workflows/instances/{id}` — read instance state
  (current revision, status). Requires `workflow:instance_read`.
- `GET /v1/workflows/instances/{id}/history` — read revision
  history. Requires `workflow:instance_read`.
- `GET /v1/workflows/instances/{id}/steps` — list step records
  for the instance. Requires `workflow:instance_read`.
- `POST /v1/workflows/instances/{id}/execute` — execute a step.
  **Accepts either `Principal` or `Ephemeral` context.**
  Requires `workflow:instance_execute`. Subject context (if
  ephemeral) flows to the workflow script.
- `POST /v1/workflows/instances/{id}/complete` — mark instance
  completed. Requires `workflow:instance_execute`.
- `POST /v1/workflows/instances/{id}/cancel` — cancel instance.
  Requires `workflow:instance_cancel`.

### Endpoint config management

All require `Principal` authentication. Endpoint configs are
`TenantEndpointConfig` entities; see `09-policy-and-tenancy.md`
for the data model and `08-connector-architecture.md` for how
they're consumed at step-execution time.

- `POST /v1/endpoints` — create a new endpoint config.
  Requires `endpoint:create`. Request body:
  `{display_name, config}` where `config` is the free-form
  JSON blob (conventionally `{realm, impl, config: {...}}`).
  API encrypts the blob with the substrate credential key
  before storage. No schema validation on the blob contents
  in v1; invalid configs fail at first call.
- `GET /v1/endpoints` — list endpoint configs (metadata only).
  Requires `endpoint:read_metadata`. Returns display names,
  creation times, retirement status, UUIDs.
- `GET /v1/endpoints/{id}` — read endpoint config metadata.
  Requires `endpoint:read_metadata`.
- `GET /v1/endpoints/{id}/decrypted` — read the decrypted
  config blob for operator verification. Requires
  `endpoint:read_decrypted`. Sensitive fields (credentials,
  API keys) may be display-redacted at the presentation layer
  even with this permission.
- `POST /v1/endpoints/{id}/rotate` — append a new revision to
  an existing config (new credentials, new URL, etc.).
  Requires `endpoint:rotate`. Same UUID; templates
  referencing the UUID automatically pick up the new
  revision at next step execution.
- `POST /v1/endpoints/{id}/retire` — retire a config.
  Requires `endpoint:retire`. Templates referencing a retired
  config fail at the lowerer; retirement breaks the workflow,
  which is the correct failure mode.

The submitted blob is never returned in plaintext to callers
without `endpoint:read_decrypted`. Metadata-level reads
(`endpoint:read_metadata`) return only the display name and
bookkeeping scalars.

### Principal and role management

All require `Principal` authentication with the appropriate
management permission.

- `POST /v1/principals` — create a principal. Requires
  `tenant:principal_manage`. Returns the generated long-lived
  API token **once** (not stored in plaintext; only the hash
  persists). Format: `pht_<43-char base64url>`.
- `GET /v1/principals` — list principals in the tenant.
  Requires `tenant:principal_manage`.
- `POST /v1/principals/{id}/rotate` — rotate the principal's
  long-lived token. Requires `tenant:principal_manage`.
  Returns the new token once.
- `POST /v1/principals/{id}/retire` — retire a principal.
  Requires `tenant:principal_manage`.
- `POST /v1/roles` — create a role definition. Requires
  `tenant:role_manage`.
- `GET /v1/roles` — list roles. Requires `tenant:role_manage`.
- `PATCH /v1/roles/{id}` — modify a role (adds a new
  revision). Requires `tenant:role_manage`.
- `POST /v1/roles/{id}/retire` — retire a role. Requires
  `tenant:role_manage`.
- `POST /v1/role-memberships` — assign a role to a principal.
  Requires `tenant:role_manage`.
- `DELETE /v1/role-memberships/{id}` — remove a role
  assignment (adds a retirement revision). Requires
  `tenant:role_manage`.

### Minting authority management

Require `Principal` authentication with `tenant:minting_manage`.

- `POST /v1/minting-authorities` — create a minting authority.
  Returns the long-lived authority credential **once** (same
  `pht_` format as principal tokens).
- `GET /v1/minting-authorities` — list minting authorities.
- `POST /v1/minting-authorities/{id}/rotate` — rotate the
  authority's credential.
- `POST /v1/minting-authorities/{id}/bump-epoch` — bump the
  authority's epoch, invalidating outstanding ephemeral tokens.
- `POST /v1/minting-authorities/{id}/retire` — retire the
  authority.
- `PATCH /v1/minting-authorities/{id}` — modify the permission
  envelope or minting constraints.

### Minting endpoint

This is the endpoint tenant applications call to mint ephemeral
tokens.

- `POST /v1/tokens/mint` — mint an ephemeral token.
  Authenticated by a `Principal` context where the principal is
  a `MintingAuthority`. Requires `mint:ephemeral_token`.

Request:
```json
{
  "subject": "opaque-subject-id",
  "lifetime_seconds": 3600,
  "instance_id": "optional-instance-scope",
  "requested_permissions": ["workflow:instance_execute"],
  "injected_claims": {
    "user_id": "u_12345",
    "account_tier": "pro"
  }
}
```

Response:
```json
{
  "token": "<COSE_Sign1 bytes, base64-encoded>",
  "expires_at": "2026-04-17T12:00:00Z",
  "subject": "opaque-subject-id",
  "instance_id": "if provided in request"
}
```

Processing:

1. Authenticate the calling minting authority (long-lived
   credential).
2. Check `mint:ephemeral_token` permission on the authority.
3. Validate `lifetime_seconds` against the system maximum and
   the authority's `minting_constraints`.
4. Clip `requested_permissions` to the authority's
   `permission_envelope`. Stripped permissions are logged in
   the audit trail.
5. If `instance_id` provided, verify it exists in the tenant
   and the authority has permission to scope tokens to it.
6. Validate `injected_claims` size (4 KB maximum) and shape
   (no Philharmonic-reserved claim names).
7. Construct the COSE_Sign1 token with the effective claims
   plus Philharmonic-managed fields (`iss`, `exp`, `tenant`,
   `authority`, `authority_epoch`, `kid`).
8. Sign with the API signing key.
9. Record a `TokenMintingEvent` audit record (subject
   identifier and authority ID only; not the full injected
   claims).
10. Return the token.

**No instance creation in this endpoint.** Creating a workflow
instance is a separate call (`POST /v1/workflows/instances`).
For the chat-app pattern where a tenant backend wants to create
an instance and mint a token atomically, the backend makes two
calls back-to-back. An atomic "create-instance-and-mint" endpoint
could be added later as an ergonomic helper; not in v1.

### Tenant administration (within-tenant)

- `GET /v1/tenant` — read tenant settings. Requires
  `tenant:settings_read`.
- `PATCH /v1/tenant` — update settings (new settings
  revision). Requires `tenant:settings_manage`.

Deployment-operator-level tenant management (creating
tenants, suspending them) lives on whichever ingress the
deployment designates for operator endpoints — distinct
from any tenant's routing scope.

### Audit log access

- `GET /v1/audit` — list audit events in the tenant.
  Filterable by event type, time range, initiating principal.
  Requires `audit:read`.

## Rate limiting

Per-tenant token buckets per endpoint family (workflow
operations, credential operations, minting, etc.). Enforced at
the API layer; exceeds return `429 Too Many Requests`.

**v1 implementation:** simple single-node token buckets, with
the obvious scale limitation (per-node bucket means total
throughput is `nodes × bucket_rate`). Distributed rate limiting
(Redis-backed or similar) is a post-v1 refinement.

Minting-endpoint rate limits apply per-minting-authority, not
just per-tenant — a compromised minting authority shouldn't be
able to mint millions of tokens before someone notices.

## Observability

- **Structured logging per request** — tenant, authenticated
  identity (principal or subject+authority), endpoint, status,
  duration.
- **Metrics per endpoint** — request rate, latency, error rate.
  Per-tenant metrics help identify tenants whose usage is
  anomalous.
- **Distributed tracing** — correlation IDs propagating from API
  through workflow engine through executor through connector
  services. Important enough to design early, not necessarily to
  implement fully in v1.
- **Audit log** — policy-relevant events written to the
  substrate as `AuditEvent` entities. Queryable via the API.

## API versioning

URL-path versioning: `/v1/...`. Future versions live at `/v2/...`
simultaneously during transition periods.

Breaking changes get a new major version. Additive changes within
a version are fine and don't require a version bump (new
endpoints, new optional request fields, new response fields).

## Hosting the workflow engine

The API crate embeds the `WorkflowEngine` and calls into it
directly — the engine is not reached over HTTP from the API.
This is a *crate boundary* statement, not a deployment-shape
statement: whatever process hosts `philharmonic-api` also
holds the engine.

Whatever runs the API crate needs:

- A substrate connection (via `philharmonic-store-sqlx-mysql`
  with pooled connections, or any other store-trait
  implementation).
- Lowerer configuration — the lowerer's signing key, realm
  KEM public keys (indexed by `kid`), and the substrate
  credential key (SCK) for decrypting `TenantEndpointConfig`
  entries. The lowerer is plugged into the engine as the
  `ConfigLowerer` implementation.
- An executor endpoint — typically a load-balancer URL or
  in-process handle pointing at a mechanics worker (or fleet).
  An HTTP `StepExecutor` wraps the mechanics HTTP service;
  in-process executor wirings are equally supported by the
  trait.
- The API signing key for ephemeral tokens.

The crate is stateless beyond the per-request authentication
lookup (no cache in v1) and the minting-authority-epoch read
(also uncached in v1), so consumers are free to scale it
horizontally.

**Whether to run the API crate as one process type, split it
across many, embed it in-process for a single user, or run it
inside another binary entirely is a deployment choice the
crate does not prescribe.** The natural shape for many
multi-tenant deployments is one binary that hosts the API +
engine + lowerer together; that minimizes network hops but
isn't required.

Splitting the API surface from the workflow engine across
process boundaries (running them as separate services) is
possible — the trait surfaces are network-pluggable — but
adds a network hop without immediate benefit for the common
deployment shape. Not designed against in v1.

## What's explicitly out of scope for v1

- **OAuth 2.0 / OIDC.** API tokens only. Federation can come
  later if tenant use cases demand it.
- **Session-based authentication for a Web UI.** The Web UI
  uses the same API token mechanism (or ephemeral tokens for
  user-scoped operations).
- **Outbound webhooks.** Notifying tenants of instance
  completion, step failures, etc. Useful; deferred.
- **Distributed rate limiting.** Single-node token buckets for
  v1.
- **Streaming responses.** No streaming endpoints in v1.
- **GraphQL.** REST only.
- **Custom protocol (gRPC, etc.).** HTTP + JSON only.

## Open questions

- **Atomic create-instance-and-mint-token endpoint.** Ergonomic
  helper for chat-app-like patterns. Could be added in v1 or
  deferred; simpler to defer and let clients make two calls.

## Status

**Implemented.** Published as `philharmonic-api 0.1.0` on
crates.io (2026-04-28, Phase 8). All endpoint families landed:
workflow templates/instances, endpoint configs, principals,
roles, memberships, minting authorities, token minting, tenant
admin, audit log, operator endpoints. 86+ integration tests.
Real `ConfigLowerer` (COSE_Sign1 + COSE_Encrypt0) and
`StepExecutor` (HTTP dispatch) added in Phase 9 (2026-04-30).
