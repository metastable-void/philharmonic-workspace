# End-user session flow: instance creation, ephemeral tokens, re-mint after 24h

This guide is for the **integrator** — the tenant application's
backend that creates workflow instances on behalf of end users
and mints short-lived ephemeral API tokens scoped to those
instances. It is not for workflow script authors; see
[`workflow-authoring.md`](workflow-authoring.md) for the
JavaScript-side concerns.

Three flows:

1. **Create a workflow instance** for an end-user session.
2. **Mint an ephemeral token bound to that instance** so the
   end-user's session can execute steps on it without holding
   tenant-wide credentials.
3. **Re-mint on session resume beyond 24 hours**, because
   ephemeral token lifetime is hard-capped at 24h.

The workflow instance itself survives indefinitely — `context`
and step history are bound to the instance UUID, not to any
token. Tokens come and go; the instance is the durable
session anchor on the Philharmonic side.

## Roles and credentials in this flow

Your tenant deployment owns three kinds of credential:

| Credential | Holder | Lifetime | Used for |
|---|---|---|---|
| **Principal `pht_` token** | A long-lived service principal (the tenant app's backend service account) | Until rotated / retired | Authenticating the tenant app's backend to Philharmonic for any tenant-scoped operation. |
| **Minting-authority `pht_` token** | A `MintingAuthority` entity rather than a regular `Principal` | Until rotated / retired | Authenticating the `POST /v1/tokens/mint` call. The authority's permission **envelope** caps what the minted ephemeral token can do. |
| **Ephemeral token (COSE_Sign1)** | An end user, for one session | ≤ 24 hours (system cap) | Letting the end user's session execute workflow steps on the instance bound at mint time. |

The tenant app's backend uses the **principal `pht_` token** to
create instances, and the **minting-authority `pht_` token** to
mint ephemeral tokens. The end-user client only ever sees the
**ephemeral token**.

> The principal-token and minting-authority-token credentials
> live in the tenant app's backend secret store; never expose
> them to end-user devices. Only the ephemeral token belongs
> on the end-user's side, and it expires automatically.

## Step 1 — Create the workflow instance

```http
POST /v1/workflows/instances
Authorization: Bearer pht_<principal-token>
X-Tenant-Id: <tenant-uuid>
Content-Type: application/json
```

```json
{
  "template_id": "<template-uuid>",
  "args": {
    "end_user_id": "u_12345",
    "locale": "ja-JP"
  }
}
```

| Field | Required | Notes |
|---|---|---|
| `template_id` | yes | UUID of the workflow template the end-user session runs against. |
| `args` | yes | Per-instance JSON arguments. Immutable for the life of the instance. Available to every script step as `arg.args`. |

Response:

```json
{ "instance_id": "<instance-uuid>" }
```

Permissions required on the principal: `workflow:instance_create`.

Save the `instance_id` alongside the end-user's session in your
tenant app's session store. This is the durable handle for
everything that follows — including the re-mint flow when the
ephemeral token expires.

## Step 2 — Mint the workflow-run-only ephemeral token

```http
POST /v1/tokens/mint
Authorization: Bearer pht_<minting-authority-token>
X-Tenant-Id: <tenant-uuid>
Content-Type: application/json
```

```json
{
  "subject": "u_12345",
  "lifetime_seconds": 3600,
  "instance_id": "<instance-uuid>",
  "requested_permissions": [
    "workflow:instance_execute",
    "workflow:instance_read"
  ],
  "injected_claims": {
    "user_id": "u_12345",
    "tier": "pro",
    "locale": "ja-JP"
  }
}
```

| Field | Required | Notes |
|---|---|---|
| `subject` | yes | Opaque string identifier you choose to bind to this end-user session. Surfaces in the workflow script as `arg.subject.id` and in `event_data.subject` of every `TOKEN_MINTED` audit row. Common shape: your end-user user ID. |
| `lifetime_seconds` | yes | Token validity in seconds from issue. Clamped to the **lesser** of the minting authority's `max_lifetime_seconds` and the system-wide hard cap of `86_400` (24 hours). Requesting more than 86 400 returns `400`. |
| `instance_id` | no (but **load-bearing for this flow**) | When set, the token can only execute steps on this specific instance. Omitting it produces a tenant-wide token (which you almost certainly don't want for end-user sessions — see "Why bind to the instance" below). |
| `requested_permissions` | yes | Subset of the authority's permission envelope. Anything outside the envelope is clipped silently. For workflow-run-only sessions, the typical set is `workflow:instance_execute` (run steps) plus `workflow:instance_read` (let the WebUI chat tab refresh transcripts). |
| `injected_claims` | yes (may be `{}`) | Free-form JSON, max 4 KiB after canonical-JSON encoding. Surfaces to scripts as `arg.subject.claims.<field>`. Tenant-defined; Philharmonic does not interpret it. **Not** recorded in the audit log — see "Privacy of injected claims" below. |

Response:

```json
{
  "token": "<base64url COSE_Sign1>",
  "expires_at": "2026-05-12T03:24:00Z",
  "subject": "u_12345",
  "instance_id": "<instance-uuid>"
}
```

Hand the `token` string to the end-user's client. The client
sends it as `Authorization: Bearer <token>` on every API call.

### Why bind to the instance

An ephemeral token without `instance_id` is a **tenant-wide**
token: with `workflow:instance_execute` it can execute steps
on **any** instance in the tenant. That's a much larger blast
radius than the end-user session needs.

Binding the token to a specific `instance_id` enforces, at
the route-protector layer
([`philharmonic-api/src/routes/workflows.rs`](../../philharmonic-api/src/routes/workflows.rs)),
that the token's holder can act on **only** that one instance.
A leaked ephemeral token gives an attacker access to one
end-user's session at most, for at most the token's remaining
lifetime — never more. This is the "workflow-run-only"
property the title refers to.

If your end-user session needs to access multiple instances
(e.g. a multi-tab app where each tab has its own conversation),
mint one ephemeral token per instance rather than one
tenant-wide token shared across instances.

### Privacy of injected claims

`injected_claims` is **runtime-visible to scripts** (via
`arg.subject.claims`) but **never recorded in the audit log**.
Per the design decision in
[`docs/design/09-policy-and-tenancy.md` §Audit trail](../design/09-policy-and-tenancy.md#audit-trail),
audit rows for `TOKEN_MINTED` events store only
`{"subject_id", "authority_id"}`. The claims themselves are
treated as tenant-private application data that should not
leak into the operator-visible audit surface.

This means you can put end-user identifiers, account tiers,
locale, feature flags, etc. into `injected_claims` without
those values appearing in operator audit views. Place them
there rather than in `subject` (which IS auditable) when the
distinction matters.

### Permissions required on the minting authority

The principal calling `POST /v1/tokens/mint` must be a
`MintingAuthority` entity, not a regular `Principal`. Use
its `pht_` token directly; the route checks the entity kind
of the caller. A regular principal calling this endpoint
gets `403`.

The minting authority's own `permission_envelope` caps what
the minted token can do — anything in `requested_permissions`
outside the envelope is silently clipped, never returned as
an error. Configure the authority's envelope conservatively
(typically just `workflow:instance_execute` and
`workflow:instance_read` for the workflow-run-only case).

## Step 3 — Re-mint after 24h on session resume

The token lifetime is hard-capped at 24 hours by
[`philharmonic_policy::MAX_TOKEN_LIFETIME_MILLIS`](../../philharmonic-policy/src/api_token.rs).
You cannot issue a token valid longer than 24h regardless of
the minting authority's settings. End-user sessions that
span more than a day need to re-mint.

The re-mint flow assumes your tenant app's backend (not the
end-user client) drives the operation:

```
End user returns to the app (cookie / OAuth / etc. authenticates them)
        │
        ▼
Tenant app backend looks up the end user's session row
        │
        ▼
Session row carries the workflow instance UUID from Step 1
        │
        ▼
Backend calls POST /v1/tokens/mint AGAIN with:
  - same instance_id
  - same subject
  - same requested_permissions (or refreshed)
  - injected_claims regenerated from current end-user state
        │
        ▼
New ephemeral token returned to the end user's client;
the instance's context and history continue from where they
left off (instance state survives across token renewals)
```

Concrete second mint:

```http
POST /v1/tokens/mint
Authorization: Bearer pht_<minting-authority-token>
X-Tenant-Id: <tenant-uuid>
Content-Type: application/json
```

```json
{
  "subject": "u_12345",
  "lifetime_seconds": 3600,
  "instance_id": "<same-instance-uuid-as-before>",
  "requested_permissions": [
    "workflow:instance_execute",
    "workflow:instance_read"
  ],
  "injected_claims": {
    "user_id": "u_12345",
    "tier": "pro",
    "locale": "ja-JP"
  }
}
```

The `instance_id` is the **same** UUID returned by Step 1.
The workflow instance's `context` (accumulated chat
transcript, embedding-dataset bindings, etc.) survives the
token expiry; the new token just authorises the end-user's
session to continue executing steps on the same instance.

### Audit-trail picture across re-mints

Every mint produces a `TOKEN_MINTED` audit row
(`event_type = 40`) with `event_data` containing
`principal_id` (the minting authority's principal),
`subject.subject_id` (your end-user identifier),
`subject.authority_id`, `route`, and `correlation_id`. A
session spanning multiple days produces one such row per
re-mint. The audit log is the operator's view of "this end
user was active in this period"; the workflow instance's
step history is the engine's view of "what they did".

### Failure modes worth handling explicitly

| Failure | Cause | Handling |
|---|---|---|
| `400 instance_id not found in tenant` | The instance was retired or never existed under this tenant. | The end user's session is dead. Recreate the instance via Step 1, save the new UUID, mint anew. |
| `400 lifetime_seconds exceeds maximum` | Requested > 86 400 or > authority's `max_lifetime_seconds`. | Request a shorter lifetime. The 24h cap is non-negotiable. |
| `403 forbidden` on `/v1/tokens/mint` | The caller is a regular `Principal`, not a `MintingAuthority`. | Use the minting-authority's `pht_` token, not a principal's. |
| `403 forbidden` on `/v1/workflows/instances/{id}/execute` | The ephemeral token was bound to a different `instance_id`. | The end user's client is using a token meant for a different session. Re-issue the right token. |
| Token expired between session activity bursts | Normal — 24h cap. | Re-mint per the flow above. |

### Don'ts

- **Do not** hand the minting authority's `pht_` token to the
  end-user client. End users only ever hold short-lived
  ephemeral tokens.
- **Do not** mint a token without `instance_id` for an
  end-user session, unless you genuinely want the token to
  span every instance in the tenant.
- **Do not** put data in `injected_claims` that your
  workflow scripts shouldn't be able to see at runtime —
  the script reads them via `arg.subject.claims`.
- **Do not** put data in `subject` that you don't want in
  operator audit views — `subject` is the auditable
  identifier. Use `injected_claims` for tenant-private
  application data.
- **Do not** rely on storing the ephemeral token at rest.
  It's a session bearer; if the end-user device loses it,
  re-mint rather than persisting it to a database.

## See also

- [`workflow-authoring.md`](workflow-authoring.md) — the
  JavaScript-side concerns once the workflow is running
  (script-arg shape, return value, endpoint calls, sandbox
  limits).
- [`docs/design/09-policy-and-tenancy.md` §Audit trail](../design/09-policy-and-tenancy.md#audit-trail)
  — the `TOKEN_MINTED` audit-row contract and the privacy
  decision for `injected_claims`.
- [`docs/design/10-api-layer.md`](../design/10-api-layer.md)
  — the API surface, route families, authentication
  middleware, and `pht_` token format.
- [`docs/design/11-security-and-cryptography.md`](../design/11-security-and-cryptography.md)
  — COSE_Sign1 token signing, key rotation, the 14-step
  verification.
