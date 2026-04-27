# Security and Cryptography

This file consolidates the cryptographic design across the
system, plus observability and error-surface conventions that
cross-cut all crates.

## Threat model

### The executor is untrusted

Boa is a complex JavaScript engine with inevitable security
defects. A Boa exploit during script execution should not
compromise:

- External service credentials (API keys, DB passwords, SMTP
  credentials).
- Capability-bearing URLs (webhook URLs with embedded tokens,
  signed object-storage URLs, internal endpoints).
- Tenant-private configuration (internal hostnames, vendor
  identifiers, rate-limit settings).
- Per-tenant authorization decisions.

**What an exploit can access:** anything in `MechanicsConfig`
in plaintext, anything passed as the script's argument
(`context`, `args`, `input`, `subject`), anything returned
from connector calls during the exploit's window.

**What an exploit cannot access:** anything encrypted to a
realm's KEM public key (without that realm's private key), the
lowerer's signing key, the API layer's signing key, the
substrate credential key, the connector service's realm
private key.

**Implication:** anything sensitive transits through the
executor as ciphertext, decrypted only at the connector
service.

### The API is the trust boundary for tenant callers

Tenant callers authenticate to the API layer, which verifies
tokens and applies policy before invoking the workflow engine.
Deeper layers trust that the API has authenticated and
authorized the request.

### The substrate is semi-trusted

Substrate compromise should not yield plaintext credentials or
other tenant-private endpoint configuration. Per-tenant
endpoint configs are encrypted at rest as whole blobs; the
substrate sees only the tenant reference, metadata scalars,
and opaque ciphertext. Neither the destination realm nor the
implementation name is visible in cleartext. A compromise
exposes workflow history, template sources, and audit trails —
but not the contents of `TenantEndpointConfig` entries.

### Harvest-now-decrypt-later is in scope

Credentials and tenant data may retain value for years. An
adversary recording encrypted traffic today to decrypt later
(via a cryptographically-relevant quantum computer) is a
threat the design must address. Asymmetric encryption uses
hybrid post-quantum KEM from day one.

## Cryptographic primitives

### KEM: ML-KEM-768 hybridized with X25519

For encrypting payloads to connector services.

- **ML-KEM-768**: NIST-standardized post-quantum KEM, ~AES-192
  security level. FIPS 203.
- **X25519**: classical ECDH as a hedge against PQ scheme
  flaws.
- **Hybrid construction**: both KEMs produce shared secrets;
  the two are combined via HKDF-SHA256 to derive the symmetric
  key. An attacker must break both to recover plaintext.

### Symmetric encryption: AES-256-GCM

For encrypting payloads under the KEM-derived key, and for
substrate at-rest endpoint config encryption.

- Authenticated encryption; integrity failures are detectable.
- 256-bit key against quantum-Grover attacks (~128-bit
  post-quantum security).

### Signing: Ed25519 for v1

For signing authorization tokens.

- Well-understood, fast, widely supported in the RustCrypto
  ecosystem.
- Token lifetimes are short, limiting
  harvest-now-forge-later exposure.
- **Path to PQ signing:** COSE is algorithm-agile. Hybrid
  ML-DSA + Ed25519 can be adopted later by registering a new
  algorithm identifier and updating signing/verification code.
  No protocol rework.

### Hashing: SHA-256

For content addressing, for binding tokens to payloads, for
hashing long-lived API tokens before storage.

### Token format: COSE

- **COSE_Sign1** for signed tokens.
- **COSE_Encrypt0** for encrypted payloads.

Rationale: algorithm agility via structured algorithm
identifiers, first-class key ID fields, binary CBOR encoding
(compact in HTTP headers), IETF PQC work landing in COSE
first.

Rust implementation: the `coset` crate handles the structural
format; crypto operations use RustCrypto primitives (`ml-kem`,
`x25519-dalek`, `aes-gcm`, `ed25519-dalek`).

## Token types

### Connector authorization tokens

Minted by the lowerer to authorize a specific script-to-
connector call against one `TenantEndpointConfig`.

- **Format**: COSE_Sign1.
- **Signing key**: held by the lowerer. Public key held by
  connector services (for verification).
- **Lifetime**: short, in minutes.
- **Claims**:
  - `iss` — issuer (the lowerer / deployment).
  - `exp` — expiry.
  - `iat` — issued-at (Unix milliseconds); authoritative mint
    time, surfaced as `ConnectorCallContext.issued_at`.
  - `kid` — signing key ID.
  - `realm` — destination realm name.
  - `tenant` — tenant UUID.
  - `inst` — workflow instance UUID.
  - `step` — step sequence within the instance.
  - `config_uuid` — `TenantEndpointConfig` UUID (audit).
  - `payload_hash` — SHA-256 of the COSE_Encrypt0 payload
    bytes.

No `impl` claim. Dispatch at the connector service uses the
decrypted payload's `impl` field, not the token. The token
records identity, binding, and routing; the payload records
everything else.

One token is minted per config per step. A step touching three
configs produces three tokens.

- **Revocation**: natural expiry only. Short lifetime makes
  revocation-by-list unnecessary.

### Ephemeral API tokens

Minted by the API layer when a minting authority calls the
minting endpoint. Authenticates subsequent API calls by
ephemeral subjects.

- **Format**: COSE_Sign1.
- **Signing key**: held by the API layer. Distinct from the
  lowerer's signing key.
- **Lifetime**: up to system-wide maximum (24h),
  per-minting-authority configurable below that.
- **Claims**: as listed in `09-policy-and-tenancy.md`.
  Injected claims capped at 4 KB.
- **Revocation**:
  - **Individual tokens**: natural expiry.
  - **Mass revocation under an authority**: bump
    `epoch` on `MintingAuthority`.
  - **Mass revocation under a tenant**: bump every minting
    authority's epoch for that tenant. Automatic on tenant
    suspension.

### Long-lived API tokens

Bearer credentials held by persistent principals (including
minting authorities acting on their own authority).

- **Format**: `pht_<43-char base64url, no padding>` encoding
  32 random bytes from a CSPRNG. Total length 47 characters.
  Prefix enables grep-based leak detection. Full format
  specified in `09-policy-and-tenancy.md`.
- **Storage**: SHA-256 hash of the full token (including
  prefix) stored in the `Principal` entity's credential slot;
  plaintext never persisted.
- **Lifetime**: long (deployment-configurable; weeks to
  months).
- **Revocation**: removing the hash from the principal entity
  (or marking the principal retired). API layer looks up the
  principal on every request.

A `Principal.epoch` scalar is reserved for a future migration
to self-contained COSE_Sign1 long-lived tokens with
epoch-based mass revocation (pattern identical to
`MintingAuthority.epoch`). Not consumed in v1.

## Encryption systems

### Per-realm KEM (connector payloads)

For encrypting sensitive configuration to connector services.
The executor ships the ciphertext through unchanged; only the
destination realm can decrypt.

- **Key type**: hybrid KEM keypair per realm (ML-KEM-768 +
  X25519).
- **Public keys**: held by the lowerer as a registry indexed
  by realm, each entry tagged with `kid` for rotation.
- **Private keys**: held by connector service binaries in the
  realm, loaded at startup from deployment secret storage.
- **Encryption format**: COSE_Encrypt0 with `kid`, hybrid
  algorithm identifier, ciphertext encrypted under HKDF
  (ML-KEM shared secret || X25519 shared secret) with
  AES-256-GCM.
- **Rotation**: new keypair per realm; lowerer switches to new
  public key for new payloads; connector services accept both
  during overlap; retire old private key after overlap.

Encryption happens per config per call — each call encrypts a
fresh payload to its config's declared realm.

### Substrate at-rest endpoint config encryption

For `TenantEndpointConfig` entities.

- **Key type**: deployment-level symmetric AES-256 key (the
  substrate credential key, SCK). Envelope encryption with a
  KMS is a deployment option.
- **Scheme**: AES-256-GCM.
- **Who holds the key**: the API layer (encrypts on submit,
  decrypts on operator read) and the lowerer (decrypts at
  step-execution time). Both components run in the same
  process in v1 deployments.
- **Scope of the encrypted blob**: the entire admin-submitted
  config JSON — including realm, implementation name, and
  credentials — as a single AES-256-GCM ciphertext. The
  substrate sees only ciphertext plus metadata scalars.
- **Rotation**: new key generated, all `TenantEndpointConfig`
  entities re-encrypted via new revisions, `key_version`
  scalar updated, old key retired.

### Lowerer re-encryption — pure byte forwarding

When the lowerer produces a per-step payload, it decrypts the
SCK-encrypted blob and re-encrypts it byte-identical to the
target realm's KEM public key. No field extraction,
substitution, or reshaping. The bytes the admin submitted are
the bytes the implementation deserializes. The only field the
lowerer reads from the plaintext is `realm`, to pick the
destination KEM key.

Consequence: the lowerer is structurally simple. It's an
encryption-boundary translator, not a config builder. Adding a
new implementation does not require any lowerer changes.

### Ephemeral API token signing key rotation

Distinct from the lowerer's signing key; same rotation
pattern:

- Generate new keypair.
- API layer starts signing with new key.
- Verifiers accept both old and new during overlap.
- Retire old after maximum ephemeral token lifetime has
  elapsed.

## Encrypted payload flow

Per-step lifecycle for one `TenantEndpointConfig`, from
storage to external call:

1. **At rest in substrate**: Config stored as a
   `TenantEndpointConfig` entity with `encrypted_config`
   content slot. AES-256-GCM ciphertext under SCK. Substrate
   sees only tenant reference, metadata scalars, ciphertext.

2. **At step mint**: Lowerer fetches config by UUID, decrypts
   with SCK. In-memory plaintext briefly (includes realm,
   impl, and credentials).

3. **At encapsulation**: Lowerer reads `realm` from the
   plaintext; looks up the realm's KEM public key. Encrypts
   the plaintext — byte-identical — with COSE_Encrypt0 to
   that realm's KEM key using the hybrid ML-KEM + X25519
   construction.

4. **At token mint**: Lowerer hashes the encrypted payload
   bytes, includes the hash in the COSE_Sign1 token's
   `payload_hash` claim alongside realm, tenant, inst, step,
   config_uuid. Signs with the lowerer's Ed25519 key.

5. **In `MechanicsConfig`**: Token in the `Authorization`
   header; encrypted payload in `X-Encrypted-Payload` (or
   equivalent). Both opaque strings to the executor.

6. **In transit**: Ciphertext and token travel through
   executor memory, the HTTP request to the connector router,
   and the router to a connector service.

7. **At verification**: Connector service verifies COSE_Sign1
   signature by `kid`, checks `exp`, matches `realm` against
   its own realm, verifies `payload_hash`.

8. **At decapsulation**: Connector service decrypts with its
   realm private key by `kid`.

9. **At dispatch**: Service parses the decrypted JSON, checks
   the inner `realm` against the token's `realm` claim, looks
   up the inner `impl` in its implementation registry, passes
   the `config` sub-object to the handler.

10. **At use**: Implementation deserializes `config`, uses the
    plaintext credentials to call the external service.

11. **At disposal**: Decrypted blob discarded after the
    external call. No persistent storage in the connector
    service.

Plaintext credentials exist only in lowerer memory (steps 2–3)
and in connector service memory (steps 8–10).

## Key management

### Where keys live

- **Lowerer signing key** (connector tokens): deployment
  secret storage. Loaded by whichever process hosts the
  lowerer (the same process that hosts the API crate, in
  the typical deployment shape that co-locates them).
- **Lowerer public key**: distributed to connector services
  via deployment configuration.
- **API layer signing key** (ephemeral tokens): deployment
  secret storage. Loaded by whichever process hosts the API
  crate. Used for signing and self-verification.
- **Realm KEM public keys**: deployment configuration of the
  lowerer.
- **Realm KEM private keys**: deployment secret storage.
  Loaded by connector service binaries for the realm at
  startup.
- **Substrate credential key (SCK)**: deployment secret
  storage. Loaded by whichever process hosts the lowerer
  (the same process that hosts the API crate when the two
  are co-located, which is the typical shape).

No key material is committed to source control.

### Key identifiers

Every key has a stable `kid`. Rotation:

1. Generate new keypair with new `kid`.
2. Distribute new public key / register new private key.
3. Switch signing / encryption to new key.
4. Keep old key available for verification / decryption
   during overlap ≥ maximum token/payload lifetime.
5. Retire old key.

### Key compromise blast radius

- **Lowerer signing key compromise**: attacker can forge
  connector authorization tokens. Short lifetime and specific
  binding (instance, step, config UUID, payload hash) narrow
  the attack. Mitigation: urgent key rotation.
- **API layer signing key compromise**: attacker can forge
  ephemeral API tokens. Up-to-24h lifetime. Mitigation:
  urgent rotation; epoch bumps on every minting authority
  invalidate outstanding tokens.
- **Realm KEM private key compromise**: all encrypted payloads
  ever sent to that realm are decryptable. Mitigation: rotate
  all credentials possibly transmitted to the realm (tenant-
  driven re-issuance at upstream providers); rotate realm
  key.
- **SCK compromise**: all stored endpoint configs are
  recoverable. Mitigation: rotate every credential at the
  upstream provider; rotate SCK via re-encryption migration.
- **Minting authority credential compromise**: scoped to one
  tenant. Mitigation: bump epoch, rotate the authority
  credential.

Per-realm KEM keys are the primary blast-radius containment:
realm isolation means a connector-service compromise in one
realm doesn't affect others.

## Defense in depth

Layered verifications:

- **TLS**: all HTTP hops are TLS-encrypted.
- **Connector token signature verification**: connector
  services reject tokens with invalid signatures.
- **Payload decryption**: decryption failure means the payload
  wasn't encrypted to this realm; request rejected.
- **Token claim checking**: `exp`, `realm` match this binary.
- **Payload hash binding**: token `payload_hash` claim must
  match SHA-256 of ciphertext; mix-and-match attempts
  rejected.
- **Inner-realm match**: decrypted payload's `realm` field
  must match token's `realm` claim (belt-and-suspenders; AEAD
  already binds ciphertext to the realm key).
- **Implementation registry dispatch**: only impls registered
  in this binary are accepted.
- **Implementation-side validation**: shape and
  capability-specific invariants on the decrypted `config`
  sub-object.
- **Database-level privileges for SQL**: read-only users
  where applicable.
- **Per-tenant rate limiting**: limits abuse rate regardless
  of other failures.

Each layer can fail; failures are contained by the next.

## Observability

System-wide conventions so logs, metrics, and traces compose
across crates.

### Correlation IDs

Header name: `X-Correlation-ID`. UUID v4 format. Generated at
API ingress if absent on the incoming request; forwarded on
every outbound hop (API → mechanics worker → connector router
→ connector service). Also recorded on `StepRecord` entities
as a content slot, making substrate history queryable by
correlation ID.

### Logging

JSON-line output via the `tracing` crate plus
`tracing-subscriber` with a JSON formatter. One log record per
line on stdout (deployments route to their log aggregator of
choice).

Required fields on every record: `ts`, `level`,
`correlation_id`, `crate`, `msg`.

Context fields promoted to top level when present:
`tenant_id`, `instance_id`, `step_seq`, `config_uuid`, `impl`,
`realm`, `duration_ms`.

All crates use `tracing`; the formatter is configured
consistently across binaries.

### Metrics

Prometheus, via the `metrics` crate with
`metrics-exporter-prometheus`. Naming convention:

```
philharmonic_<component>_<thing>_<unit>
```

Examples: `philharmonic_api_requests_total`,
`philharmonic_workflow_step_duration_seconds`,
`philharmonic_connector_payload_encrypt_duration_seconds`,
`philharmonic_connector_call_duration_seconds`.

Labels: `impl`, `realm`, `outcome` (`success` / `failure` /
`timeout`), and others as appropriate. **No `tenant_id` as a
metric label by default** — the cardinality would explode.
Per-tenant observability goes through logs (which carry
`tenant_id` as a field and are queryable), not metrics.

### Tracing spans

Emit `tracing` spans around significant operations (API
request, step execution, connector call). No OTLP exporter
required in v1; spans primarily exist so log records reference
them. Adding an OTLP collector later requires configuration,
not code changes.

## Error surface

System-wide conventions for API-layer errors returned to
tenant callers.

### Envelope

```json
{
  "error": {
    "code": "resource_not_found",
    "message": "Workflow template does not exist.",
    "details": {
      "resource_type": "workflow_template",
      "id": "..."
    },
    "correlation_id": "..."
  }
}
```

- `code`: lowercase snake_case string.
- `message`: human-readable English. No localization v1.
- `details`: optional, code-specific, free-form object.
- `correlation_id`: the request's correlation ID, so support
  conversations have a handle.

### HTTP status → code families

- 400 → `malformed_request`, `invalid_parameters`,
  `validation_failed`.
- 401 → `authentication_required`, `invalid_credentials`,
  `token_expired`, `token_invalid`.
- 403 → `permission_denied`, `tenant_suspended`,
  `instance_scope_mismatch`.
- 404 → `resource_not_found`.
- 409 → `revision_conflict` (retryable), `duplicate_entity`
  (application-layer uniqueness fail), `instance_terminal`.
- 422 → `semantic_error` (valid syntax, business rule
  violated).
- 429 → `rate_limited` (with `Retry-After` header).
- 500 → `internal_error` (no internal details exposed;
  correlation ID is the only handle).
- 502 → `upstream_unavailable`.
- 503 → `service_unavailable`.
- 504 → `upstream_timeout`.

String codes rather than numeric: easier to read, easier to
reference in docs.

No retry guidance in the body (`Retry-After` header for 429
is the only exception). Clients decide retry policy.

No idempotency keys in v1. Duplicate creates surface as
`duplicate_entity` on application-layer uniqueness violations;
clients retry by reading current state.

## Open questions

### Revocation window for connector tokens

If a deployment wants faster incident response (lowerer key
compromise detected, reject outstanding tokens before natural
expiry), a lightweight revocation mechanism at the connector
router could help — e.g., a "minimum issuance timestamp"
rejecting tokens with `iat` older than a threshold. Deferred;
short lifetime is the baseline.

### Hardware security modules

Signing keys and SCK could live in HSMs or cloud KMS in
high-security deployments. The lowerer and API layer would
call a key-management interface rather than reading key
material directly. Affects deployment, not architecture.
Software-stored keys for v1 default; HSM-backed deployments
are an operational choice.

### Per-tenant encryption keys

v1 uses deployment-level shared SCK and per-realm KEM keys.
Per-tenant SCK would improve isolation at operational cost.
Deferred.

### Long-lived API token format

Opaque bearer with substrate-hash lookup is the v1 choice.
Self-contained COSE_Sign1 alternative keeps the door open via
the reserved `Principal.epoch` scalar. Revisit if the
per-request substrate lookup becomes a bottleneck.

## Status

Cryptographic architecture is settled. Specific operational
procedures (HSM integration, rotation cadence, revocation
windows) are deployment-level decisions that can be made
per-deployment without affecting the architecture.

Observability and error-surface conventions are settled at
the structural level. Concrete logger configuration,
Prometheus scrape endpoints, and the permission atom
enumeration will land with the crates that emit them.
