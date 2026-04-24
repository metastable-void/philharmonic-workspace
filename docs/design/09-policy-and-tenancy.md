# Policy and Tenancy

`philharmonic-policy` owns tenants, principals, per-tenant
endpoint configs, roles, and the minting authorities that issue
ephemeral API tokens. It ships with v1 of the Philharmonic
crate family.

## What the layer is for

The policy layer provides the vocabulary and storage shape for
scoping, authentication data, authorization data, encrypted
endpoint configuration, and audit. It's the layer consumers
reach for when they need any of:

- A scoping boundary that owns resources (workflow templates,
  endpoint configs, tenant settings). One scope per deployment
  is a valid use; many scopes per deployment is also a valid
  use.
- Persistent identities that authenticate to the API.
- Stored per-endpoint configuration, encrypted at rest.
- Role-based permissions within a scope.
- Short-lived tokens minted on behalf of scope-owned
  applications, for delivery to their end users.
- An append-only audit trail of policy-relevant events.

Deployment shapes this serves include multi-tenant SaaS,
single-tenant application backends, research platforms, and
single-user self-hosted installations. Consumers that need none
of the above may skip the crate entirely; the workflow and
connector layers depend only on the `Tenant` entity marker and
can work with a trivially-populated one.

What the layer explicitly doesn't own:

- Session management or authentication *logic* (API layer).
- Rate limiting (API layer or per-connector).
- Billing.
- Workflow semantics (workflow layer).

## Tenant model

### Flat tenants for v1; hierarchical deferred

Tenants form a flat namespace for v1. No parent-child
relationships, no inherited permissions, no cross-tenant scope.

Hierarchical tenancy is deferred. The entity model is shaped so
that hierarchy can be added later as an additive change —
adding a parent entity slot to `Tenant`, plus evaluation logic
that walks up the tree. Customizable roles carry much of the
flexibility hierarchy would otherwise provide for grouping
permissions.

### `Tenant` entity kind

```rust
struct Tenant;
impl Entity for Tenant {
    const KIND: Uuid = /* ... */;
    const NAME: &'static str = "tenant";
    const CONTENT_SLOTS: &'static [ContentSlot] = &[
        ContentSlot::new("display_name"),
        ContentSlot::new("settings"),  // per-tenant config
    ];
    const ENTITY_SLOTS: &'static [EntitySlot] = &[];
    const SCALAR_SLOTS: &'static [ScalarSlot] = &[
        ScalarSlot::new("status", ScalarType::I64, true),
        // 0=active, 1=suspended, 2=retired
    ];
}
```

### Tenant naming rules

Tenant names must match `[a-z0-9][a-z0-9-]{1,62}` —
lowercase alphanumerics and hyphens, no leading digit, no
consecutive hyphens, 2–63 characters. The constraint is
RFC 1035 / DNS-label compatible so that deployments which
project tenant IDs into subdomains (a common but not
required shape) can do so without a translation layer.

Reserved names that cannot be assigned to tenants:

- `admin`, `api`, `www`, `app`, `connector`. These are
  reserved across the framework because deployments
  commonly use them for non-tenant purposes (operator
  endpoints, wildcard-adjacent labels), and the reserved
  set keeps tenant IDs safe to drop into URL paths and
  subdomains interchangeably.
- Every realm name the deployment configures for connector
  routing. Realm names and tenant names share the flat
  identifier namespace as far as URL construction goes, so
  the framework keeps them disjoint.

Deployments that don't project tenant IDs into DNS or URL
paths can treat the reserved set as trivia, but the naming
rules themselves are the framework's contract and always
apply.

## Principal model

A `Principal` is a persistent identity that can authenticate
to the API. Principals are tenant-scoped.

### `Principal` entity kind

```rust
struct Principal;
impl Entity for Principal {
    const KIND: Uuid = /* ... */;
    const NAME: &'static str = "principal";
    const CONTENT_SLOTS: &'static [ContentSlot] = &[
        ContentSlot::new("credential_hash"),
        // SHA-256 of the long-lived API token; plaintext never
        // stored. Rotated by appending a new revision.
        ContentSlot::new("display_name"),
    ];
    const ENTITY_SLOTS: &'static [EntitySlot] = &[
        EntitySlot::of::<Tenant>("tenant", SlotPinning::Pinned),
    ];
    const SCALAR_SLOTS: &'static [ScalarSlot] = &[
        ScalarSlot::new("kind", ScalarType::I64, true),
        // 0=user, 1=service_account
        ScalarSlot::new("epoch", ScalarType::I64, true),
        // Reserved for future self-contained long-lived tokens.
        // See Note on Principal.epoch below.
        ScalarSlot::new("is_retired", ScalarType::Bool, true),
    ];
}
```

**Note on `Principal.epoch`.** For v1, long-lived API tokens
are opaque bearer strings validated by substrate-hash lookup;
the `epoch` scalar is unused. Present on the entity so that a
future migration to self-contained COSE_Sign1 long-lived tokens
— following the same epoch-bump revocation pattern as
`MintingAuthority` — requires no schema change. Zero cost to
include now; additive to consume later.

### Principals vs. ephemeral subjects

**Persistent principals** — represented by `Principal`
entities, authenticate with long-lived API tokens, own durable
resources (workflow templates, endpoint configs, role
assignments). Tenant admins and service accounts are
persistent principals.

**Ephemeral subjects** — callers authenticated via ephemeral
tokens minted by a minting authority. Have no persistent entity
representation. Their identity is opaque to Philharmonic (an
identifier the minting authority injects into the token's
claims); their permissions are carried in the token.

Ephemeral subjects can execute steps on workflow instances but
don't own resources. Ownership goes to the tenant; audit
attribution captures the minting authority that vouched for
the subject.

### Authentication

Persistent principals authenticate with long-lived API tokens
(opaque bearer credentials; SHA-256 hashed in the principal's
`credential_hash` slot). Token rotation is a tenant-admin
operation: a new revision with a new hash invalidates the old
token.

Ephemeral subjects authenticate with ephemeral tokens
(COSE_Sign1 signed by the API layer). See "Minting authorities
and ephemeral tokens" below.

#### Long-lived API token format

Concrete format, so every crate that generates, parses, or
displays one agrees:

```
pht_<43-char base64url-encoded 32 random bytes, no padding>
```

- Prefix `pht_` (short for *philharmonic token*) is
  grep-friendly for leak detection (log scanners, pre-commit
  hooks) and makes a stray token visually recognizable.
- 32 bytes (256 bits) of random material from a CSPRNG.
- Base64url encoding without padding — 43 characters, URL-safe,
  no ambiguous characters. Total token length is 47 characters.
- The whole token (including the `pht_` prefix) is SHA-256
  hashed for storage. The hash is what `Principal.credential_hash`
  and `MintingAuthority.credential_hash` store; the plaintext
  token is returned to the client once at creation or rotation
  and never again.

Tokens are displayed to the creating client exactly once. Lost
tokens are not recoverable; rotate instead.

## Customizable roles

Permissions attach to principals via role memberships. Roles
are customizable per tenant — a tenant admin defines the roles
that make sense for that tenant, which permissions they grant,
and which principals hold them.

### `RoleDefinition` entity kind

```rust
struct RoleDefinition;
impl Entity for RoleDefinition {
    const KIND: Uuid = /* ... */;
    const NAME: &'static str = "role_definition";
    const CONTENT_SLOTS: &'static [ContentSlot] = &[
        ContentSlot::new("permissions"),
        // JSON array of permission atom strings.
        ContentSlot::new("display_name"),
    ];
    const ENTITY_SLOTS: &'static [EntitySlot] = &[
        EntitySlot::of::<Tenant>("tenant", SlotPinning::Pinned),
    ];
    const SCALAR_SLOTS: &'static [ScalarSlot] = &[
        ScalarSlot::new("is_retired", ScalarType::Bool, true),
    ];
}
```

### `RoleMembership` entity kind

```rust
struct RoleMembership;
impl Entity for RoleMembership {
    const KIND: Uuid = /* ... */;
    const NAME: &'static str = "role_membership";
    const CONTENT_SLOTS: &'static [ContentSlot] = &[];
    const ENTITY_SLOTS: &'static [EntitySlot] = &[
        EntitySlot::of::<Principal>("principal", SlotPinning::Pinned),
        EntitySlot::of::<RoleDefinition>("role", SlotPinning::Pinned),
        EntitySlot::of::<Tenant>("tenant", SlotPinning::Pinned),
    ];
    const SCALAR_SLOTS: &'static [ScalarSlot] = &[
        ScalarSlot::new("is_retired", ScalarType::Bool, true),
    ];
}
```

Binding is three-way: principal, role, tenant. The tenant is
redundant with the role's tenant and the principal's tenant
but is stored explicitly to make tenant-filtered queries cheap.

## Permissions model

### Permission atoms

Permissions are string identifiers in a namespaced vocabulary.
The full v1 set:

**Workflow template operations:**
- `workflow:template_create` — create new workflow templates.
- `workflow:template_read` — read template definitions and
  history.
- `workflow:template_retire` — retire templates.

**Workflow instance operations:**
- `workflow:instance_create` — create new instances of any
  template in the tenant.
- `workflow:instance_read` — read instance state, history,
  and step records.
- `workflow:instance_execute` — execute steps, mark instances
  complete.
- `workflow:instance_cancel` — cancel running instances.

**Endpoint config operations** (the `TenantEndpointConfig`
entity family):
- `endpoint:create` — create new configs.
- `endpoint:rotate` — append a new revision to an existing
  config (credential rotation, URL change, etc.).
- `endpoint:retire` — mark a config as retired.
- `endpoint:read_metadata` — read config metadata
  (display_name, creation time, retirement status) without
  decrypting the blob. Suitable for listing configs in UI.
- `endpoint:read_decrypted` — read the decrypted config blob
  for operator verification. Strictly stronger than
  `endpoint:read_metadata`. Sensitive fields may be
  display-redacted even with this permission.

**Principal and role management:**
- `tenant:principal_manage` — create / list / rotate / retire
  principals in the tenant.
- `tenant:role_manage` — create / modify / retire role
  definitions; create / remove role memberships.

**Minting authority management:**
- `tenant:minting_manage` — create / list / modify / rotate /
  retire minting authorities, and bump their epochs.
- `mint:ephemeral_token` — use the minting endpoint to mint
  an ephemeral token. Granted on minting authorities
  themselves (not on persistent principals in general);
  enforced against the authority's permission envelope at
  mint time.

**Tenant-wide settings:**
- `tenant:settings_read` — read tenant-wide configuration.
- `tenant:settings_manage` — modify tenant-wide configuration.

**Audit access:**
- `audit:read` — query the tenant's audit log. Dedicated
  permission rather than overloading `tenant:settings_read`
  because audit access and settings management are
  operationally distinct concerns.

**Deployment-operator permissions** (granted only to
principals in the operator tenant, enforced on whichever
ingress the deployment designates for operator endpoints —
separate subdomain, reserved path prefix, or separate
listener; not relevant for tenant
users):
- `deployment:tenant_manage` — create / suspend / resume /
  retire tenants.
- `deployment:realm_manage` — add or retire realms, rotate
  realm keys.
- `deployment:audit_read` — query cross-tenant audit events.

#### Permission → endpoint mapping

For cross-reference at API-layer implementation time. The
endpoints are defined in `10-api-layer.md`; this table shows
the required permission for each.

| Endpoint | Permission |
| --- | --- |
| `POST /v1/workflows/templates` | `workflow:template_create` |
| `GET /v1/workflows/templates` | `workflow:template_read` |
| `GET /v1/workflows/templates/{id}` | `workflow:template_read` |
| `POST /v1/workflows/templates/{id}/retire` | `workflow:template_retire` |
| `POST /v1/workflows/instances` | `workflow:instance_create` |
| `GET /v1/workflows/instances` | `workflow:instance_read` |
| `GET /v1/workflows/instances/{id}` | `workflow:instance_read` |
| `GET /v1/workflows/instances/{id}/history` | `workflow:instance_read` |
| `GET /v1/workflows/instances/{id}/steps` | `workflow:instance_read` |
| `POST /v1/workflows/instances/{id}/execute` | `workflow:instance_execute` |
| `POST /v1/workflows/instances/{id}/complete` | `workflow:instance_execute` |
| `POST /v1/workflows/instances/{id}/cancel` | `workflow:instance_cancel` |
| `POST /v1/endpoints` | `endpoint:create` |
| `GET /v1/endpoints` | `endpoint:read_metadata` |
| `GET /v1/endpoints/{id}` | `endpoint:read_metadata` |
| `GET /v1/endpoints/{id}/decrypted` | `endpoint:read_decrypted` |
| `POST /v1/endpoints/{id}/rotate` | `endpoint:rotate` |
| `POST /v1/endpoints/{id}/retire` | `endpoint:retire` |
| `POST /v1/principals` | `tenant:principal_manage` |
| `GET /v1/principals` | `tenant:principal_manage` |
| `POST /v1/principals/{id}/rotate` | `tenant:principal_manage` |
| `POST /v1/principals/{id}/retire` | `tenant:principal_manage` |
| `POST /v1/roles` | `tenant:role_manage` |
| `GET /v1/roles` | `tenant:role_manage` |
| `PATCH /v1/roles/{id}` | `tenant:role_manage` |
| `POST /v1/roles/{id}/retire` | `tenant:role_manage` |
| `POST /v1/role-memberships` | `tenant:role_manage` |
| `DELETE /v1/role-memberships/{id}` | `tenant:role_manage` |
| `POST /v1/minting-authorities` | `tenant:minting_manage` |
| `GET /v1/minting-authorities` | `tenant:minting_manage` |
| `POST /v1/minting-authorities/{id}/rotate` | `tenant:minting_manage` |
| `POST /v1/minting-authorities/{id}/bump-epoch` | `tenant:minting_manage` |
| `POST /v1/minting-authorities/{id}/retire` | `tenant:minting_manage` |
| `PATCH /v1/minting-authorities/{id}` | `tenant:minting_manage` |
| `POST /v1/tokens/mint` | `mint:ephemeral_token` |
| `GET /v1/tenant` | `tenant:settings_read` |
| `PATCH /v1/tenant` | `tenant:settings_manage` |
| `GET /v1/audit` | `audit:read` |

The `tenant:principal_manage` atom covers both create and
retire operations on principals rather than splitting into
separate atoms. Same for `tenant:role_manage` and
`tenant:minting_manage`. Splitting is additive if finer
control is later needed.

The atom vocabulary is deployment-visible but not intended for
tenant-level extension. New atoms are added via deployment
updates.

### Role definition document

A `RoleDefinition`'s `permissions` content slot is a JSON
array of permission atom strings:

```json
[
  "workflow:template_create",
  "workflow:instance_read",
  "workflow:instance_execute",
  "endpoint:read_metadata"
]
```

Simple set membership. A role grants a set of permission atoms;
a principal has a permission if any of their roles within the
tenant grants it. No scoped permissions, no conditional
permissions, no attribute-based matching. If richer semantics
are needed later, the document shape is additive — a future
`{permissions: [...], constraints: {...}}` object is a
backward-compatible superset of the current array form.

### Evaluation

For a request authenticated by a persistent principal:

1. Identify the authenticated principal and the request's
   target tenant.
2. Reject if the principal's tenant doesn't match.
3. List the principal's active `RoleMembership` entities within
   the tenant.
4. For each role membership, read the `RoleDefinition`.
5. Check whether any role definition grants the required
   permission.
6. Allow or deny accordingly.

For a request authenticated by an ephemeral token: the token's
claims already carry the effective permissions (clipped to the
minting authority's envelope at mint time). No substrate lookup
for role evaluation per-request; check the token's permission
claims against the required permission.

## Minting authorities and ephemeral tokens

### The pattern

A tenant's own application often needs to let its end users or
automations reach Philharmonic APIs without handing them the
tenant's long-lived credentials. The minting-authority pattern
addresses this: a tenant holds a long-lived credential for a
*minting authority*, uses it to mint short-lived ephemeral
tokens bearing claims about a specific subject, and delivers
those tokens to the subject (browser, script, partner system,
job runner, etc.) for subsequent API calls.

Supported shapes include browser-based chat applications
(tenant backend mints a per-session token; browser calls
`execute_step` directly), scheduled job runners, partner
integrations, and CLI tools.

### `MintingAuthority` entity kind

```rust
struct MintingAuthority;
impl Entity for MintingAuthority {
    const KIND: Uuid = /* ... */;
    const NAME: &'static str = "minting_authority";
    const CONTENT_SLOTS: &'static [ContentSlot] = &[
        ContentSlot::new("credential_hash"),
        // SHA-256 of the long-lived credential.
        ContentSlot::new("display_name"),
        ContentSlot::new("permission_envelope"),
        // JSON array of permission atoms; bounds ephemeral
        // token permissions.
        ContentSlot::new("minting_constraints"),
        // Max ephemeral token lifetime, allowed claim
        // namespaces, etc.
    ];
    const ENTITY_SLOTS: &'static [EntitySlot] = &[
        EntitySlot::of::<Tenant>("tenant", SlotPinning::Pinned),
    ];
    const SCALAR_SLOTS: &'static [ScalarSlot] = &[
        ScalarSlot::new("epoch", ScalarType::I64, true),
        // Bumped to invalidate outstanding ephemeral tokens.
        ScalarSlot::new("is_retired", ScalarType::Bool, true),
    ];
}
```

### Permission envelope

The envelope bounds what the authority can grant. Structurally
identical to a role definition document (JSON array of
permission atoms). At mint time, the API layer clips requested
permissions to the envelope: out-of-envelope requests are
stripped silently and audited.

### Ephemeral token claims

Ephemeral tokens are COSE_Sign1 signed by the API layer.
Claims:

- `iss` — API layer identity.
- `exp` — expiry.
- `sub` — subject identifier (opaque to Philharmonic).
- `tenant` — tenant scope.
- `authority` — minting authority entity ID.
- `authority_epoch` — the authority's `epoch` at mint time.
- `instance` — optional workflow instance UUID for
  instance-scoped tokens.
- `permissions` — effective permissions, clipped to envelope.
- `claims` — injected subject metadata (free-form,
  tenant-defined, capped at 4 KB).
- `kid` — API signing key ID.

### Instance-scoped ephemeral tokens

An ephemeral token can be scoped to a specific workflow
instance. Instance-scoped tokens have a smaller blast radius
and are the recommended default whenever a token is delivered
to a less-trusted environment (browser, third-party process,
short-lived worker).

The browser-chat flow:

1. Tenant backend creates a workflow instance for a new
   session (authenticating as a persistent principal or the
   minting authority itself).
2. Tenant backend mints an ephemeral token scoped to that
   instance, with permissions limited to `workflow:instance_execute`.
3. Downstream caller (browser) uses the token. It can only
   call `execute_step` on that instance. It cannot create new
   instances, cannot access other instances, cannot modify
   endpoint configs.

### Revocation

Three levels:

- **Natural expiry** — short lifetime (up to 24h) is the
  baseline.
- **Authority epoch bump** — invalidates all outstanding
  ephemeral tokens from the authority. Used for compromise
  response.
- **Authority retirement** — setting `is_retired: true` on the
  `MintingAuthority`. Verification fails regardless of epoch.

No per-token revocation lists for v1. Per-authority rate limits
on minting are also deferred; the per-tenant API rate limit on
the mint endpoint provides the baseline abuse guardrail.

### Subject claims flow to workflow scripts

Injected subject claims are exposed to workflow scripts as a
`subject` field alongside `context`, `args`, `input`. Claims
are free-form for v1 — the minting authority and the workflow
script coordinate the claim schema within the tenant's own
application. Philharmonic doesn't validate claim shape.

Claims are capped at **4 KB** (serialized size). Larger
injected claims are rejected at mint time.

## Tenant endpoint configs

Per-tenant configuration for one call-site. The single entity
kind `TenantEndpointConfig` replaces the earlier separate
credential / capability-authorization split: authorization to
use a call-site **is** the existence of a non-retired config,
and credentials live inside the same encrypted blob as
everything else.

### `TenantEndpointConfig` entity kind

```rust
struct TenantEndpointConfig;
impl Entity for TenantEndpointConfig {
    const KIND: Uuid = /* ... */;
    const NAME: &'static str = "tenant_endpoint_config";
    const CONTENT_SLOTS: &'static [ContentSlot] = &[
        ContentSlot::new("display_name"),
        ContentSlot::new("encrypted_config"),
        // AES-256-GCM ciphertext of the full config blob
        // (including realm, impl, and credentials).
        // Substrate never sees plaintext.
    ];
    const ENTITY_SLOTS: &'static [EntitySlot] = &[
        EntitySlot::of::<Tenant>("tenant", SlotPinning::Pinned),
    ];
    const SCALAR_SLOTS: &'static [ScalarSlot] = &[
        ScalarSlot::new("key_version", ScalarType::I64, false),
        // Which substrate credential key version encrypted
        // this; used during key rotation migrations.
        ScalarSlot::new("is_retired", ScalarType::Bool, true),
    ];
}
```

Notable absences:

- **No `endpoint_name_id` scalar.** Configs are identified by
  entity UUID. Templates reference them by UUID in their
  abstract config maps. Display names live in the
  `display_name` content slot and aren't required to be
  unique.
- **No `impl_ref` or `realm_ref` cleartext slots.** The
  implementation name and destination realm are inside the
  encrypted blob; the substrate learns neither. An operator
  inspecting the raw substrate sees "tenant X has a config"
  and not what kind of config or what external service it
  targets.

### The encrypted blob

The blob is free-form JSON submitted by an admin. The
conventional top-level shape is:

```json
{
  "realm": "llm",
  "impl": "llm_openai_compat",
  "config": {
    "base_url": "https://api.openai.com/v1",
    "api_key": "sk-...",
    "model": "gpt-4o",
    ...
  }
}
```

The API layer encrypts this on submit using the substrate
credential key (SCK) and stores the ciphertext in
`encrypted_config`. The API layer can decrypt for operators
with `endpoint:read_decrypted` permission (they can see what they
committed; see below). The lowerer decrypts at step-execution
time to re-encrypt toward a connector realm.

v1 does not validate the blob's shape at the API layer. If an
operator writes `"lm_openai_compat"` (typo), the call fails
at the connector service with "unknown impl" on first use —
loud, late, safe. Schema-driven admin UI is a later feature.

### Operator visibility

Operators with `endpoint:read_decrypted` permission can retrieve the
decrypted config via the API. The admin UI surfaces this for
"yes, this is pointing at the right place" verification.
Sensitive fields may be redacted at the display layer
(`api_key: "sk-••••xyz"`), but the decryption capability
exists for operator confidence.

This rules out a purer "lowerer is the only component that can
decrypt" design: the API layer has SCK too. Given that API
and lowerer run in the same process in v1 deployments, the
purity loss is marginal.

### Uniqueness

Configs are identified by UUID. Display names aren't required
to be unique (two configs both named "OpenAI Production" is
fine if a tenant wants that). The application layer imposes
no `(tenant, name)` uniqueness constraint on create; substrate
uniqueness is at the entity-UUID level, which is guaranteed
by construction.

## The lowerer's policy consultation

At step-execution time, for each endpoint script-name in the
template's abstract config:

1. Identify the tenant from the `WorkflowInstance`'s tenant
   slot.
2. Look up the `config_uuid` for this script-name in the
   template's abstract config.
3. Fetch the `TenantEndpointConfig` by UUID.
4. Verify `config.tenant == instance.tenant` (defense in
   depth; the API layer should have enforced this at template
   creation time).
5. Verify `config.is_retired == false`. If retired, deny.
6. Decrypt `encrypted_config` with the substrate credential
   key.
7. Parse the decrypted JSON; read the `realm` field.
8. Look up the realm KEM public key in the realm registry.
9. Re-encrypt the decrypted blob — **byte-identical** — to the
   realm's KEM public key via COSE_Encrypt0.
10. Mint the COSE_Sign1 connector authorization token with
    claims `iss, exp, kid, realm, tenant, inst, step,
    config_uuid, payload_hash`.
11. Add a `MechanicsConfig` entry keyed by the script-name,
    with URL = realm connector router, headers = token +
    encrypted payload.

The lowerer's only transformation is encryption-boundary
translation: SCK-ciphertext-at-rest → plaintext-in-memory →
realm-KEM-ciphertext-in-transit. No field extraction,
substitution, synthesis, or reshaping. The bytes the admin
submitted are the payload bytes the implementation receives.

Plaintext exists in the lowerer's memory between steps 6 and 9
and not elsewhere in the process.

## Tenant suspension

When a tenant is suspended (`status: 1`):

- New API requests from the tenant are rejected at the API
  layer.
- Every minting authority's epoch is bumped automatically,
  invalidating all outstanding ephemeral tokens.
- Running workflow instances complete their current step but
  can't start new steps (the lowerer checks tenant status at
  policy consultation).
- Endpoint configs remain in storage (suspension is
  reversible; deletion isn't).

## Audit trail

Policy-relevant events are recorded as `AuditEvent` entities:

- Endpoint config created, rotated, retired.
- Role definition created, modified, retired.
- Role membership created, removed.
- Minting authority created, modified, retired.
- Minting authority epoch bumped.
- Ephemeral token minted (records subject identifier and
  minting authority only — full injected claims are not
  persisted, by design).
- Tenant status changes.
- Principal created, credential rotated, retired.

The append-only substrate gives entity-level history
automatically. `AuditEvent` adds contextual metadata that
entity changes don't capture: who initiated the change, via
which API call, correlation ID.

### `AuditEvent` entity kind

```rust
struct AuditEvent;
impl Entity for AuditEvent {
    const KIND: Uuid = /* ... */;
    const NAME: &'static str = "audit_event";
    const CONTENT_SLOTS: &'static [ContentSlot] = &[
        ContentSlot::new("event_data"),
    ];
    const ENTITY_SLOTS: &'static [EntitySlot] = &[
        EntitySlot::of::<Tenant>("tenant", SlotPinning::Pinned),
    ];
    const SCALAR_SLOTS: &'static [ScalarSlot] = &[
        ScalarSlot::new("event_type", ScalarType::I64, true),
        ScalarSlot::new("timestamp", ScalarType::I64, true),
    ];
}
```

## Relationship to other layers

- **Substrate**: policy entities are stored here and queried
  via the existing substrate trait surface. No special
  substrate support needed.
- **Workflow layer**: policy-naive, but `WorkflowInstance` has
  a tenant entity slot (so the workflow crate depends on
  `philharmonic-policy` for the `Tenant` entity marker).
- **Connector layer**: the lowerer consults policy to fetch
  and decrypt `TenantEndpointConfig` entries by UUID.
- **API layer**: exposes policy management endpoints (CRUD on
  tenants, principals, endpoint configs, roles, minting
  authorities), handles authentication, enforces permissions.

## Open questions

- **Minting endpoint request/response shape** — the *shape* is
  settled: request carries authority credential + requested
  claims + lifetime + optional instance scope; response
  carries the token string plus expiry. Exact field names
  are locked down when `philharmonic-api` begins
  implementation (Phase 8).

## What this crate doesn't do

- **Authentication logic.** The API layer performs
  authentication; policy provides the data.
- **Session management.** API layer concern.
- **Rate limiting.** API layer or per-connector.
- **Billing.** Operational tooling reads policy data if
  needed.
- **Workflow semantics.** The workflow layer owns those.

## Status

**Not yet implemented.** Design is substantially settled. On
the v1 critical path: the workflow crate takes `Tenant` as a
type-level marker, the connector client consults policy to
produce tokens, and the API layer uses policy data for
authentication and authorization.
