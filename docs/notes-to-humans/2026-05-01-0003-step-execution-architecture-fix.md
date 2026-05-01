# Step execution architecture fix

**Date**: 2026-05-01
**Author**: Claude Code
**Severity**: Architecture bug — step execution path is wrong

## Problem

The step execution path was wired incorrectly:

- `HttpStepExecutor` sends directly to the connector service,
  ignoring the JS script entirely (`_script` parameter unused)
- The lowerer produced a single `{ token, encrypted_payload }`
  instead of a `MechanicsConfig` with multiple endpoint
  definitions
- The JS script never runs — the connector service receives the
  request and dispatches to a single Implementation

## Correct architecture

```
  API server                    Mechanics worker          Connector router      Connector service
  ──────────                    ────────────────          ────────────────      ─────────────────
  1. Workflow engine
     calls lowerer
  2. Lowerer:
     - for each endpoint UUID
       in abstract_config:
       a. decrypt stored config
          (SCK)
       b. read implementation
          name
       c. encrypt per-endpoint
          payload (COSE_Encrypt0)
       d. mint per-endpoint
          token (COSE_Sign1)
       e. build HttpEndpoint
          pointing at connector
          router URL with
          token + payload as
          fixed headers
     - return MechanicsConfig
       { endpoints: { name:
         HttpEndpoint } }
  3. Executor sends
     MechanicsJob to ──────→  4. Runs JS script
     mechanics worker            in Boa sandbox
                               5. Script calls
                                  endpoint("llm", {...})
                                  → HTTP POST with
                                  pre-baked crypto ──────→ 6. Routes by realm ──→ 7. Verifies token
                                  headers                    to upstream            Decrypts payload
                               8. Receives response ←─────────────────────────── Returns impl result
                               9. Script processes
                                  response, returns
                                  { output, context,
                                    done }
  10. Workflow engine
      records step result
```

## Changes needed

### 1. `TenantEndpointConfig` entity schema

Add `implementation` content slot — stores the connector
implementation name (e.g. `"llm_openai_compat"`, `"sql_postgres"`).
This is set at endpoint creation time and is immutable (preserved
across rotations).

### 2. Endpoint API (`philharmonic-api/src/routes/endpoints.rs`)

- `CreateEndpointRequest`: add `implementation: String`
- `EndpointMetadataResponse`: add `implementation: String`
- Create handler: store `implementation` as content
- Rotate handler: carry forward `implementation` content hash
- Retire handler: carry forward `implementation` content hash

### 3. Lowerer (`bins/philharmonic-api-server/src/lowerer.rs`)

Complete rewrite. For each endpoint in abstract_config:
- Decrypt the stored endpoint config (already done)
- Read the `implementation` content slot
- Build a per-endpoint encrypted payload:
  `{ "realm": "...", "impl": "...", "config": {...} }`
- Mint a per-endpoint COSE_Sign1 token
- Build an `HttpEndpoint` with:
  - `method: "post"`
  - `url_template`: the connector router URL
  - `headers`: `{ "Authorization": "Bearer <token-hex>",
    "X-Encrypted-Payload": "<payload-hex>" }`
  - `response_body_type: "json"`
  - `request_body_type: "json"`
- Return `MechanicsConfig { endpoints }` as the concrete config

### 4. Executor (`bins/philharmonic-api-server/src/executor.rs`)

Rewrite to send a `MechanicsJob` to the mechanics worker:
- POST to `mechanics_worker_url` (not `connector_service_url`)
- Body: `{ "module_source": script, "arg": arg, "config": config }`
- Response: the script's return value as JSON

### 5. API server config

- Add `mechanics_worker_url` field (URL of the mechanics worker)
- Keep `connector_service_url` for the connector router's
  upstream dispatch (existing `connector_dispatch` config)
- The lowerer needs the **connector router URL** — which is the
  API server's own `/connector` path prefix (self-referencing),
  or a configured connector dispatch URL

### 6. WebUI

- Add `implementation` field to endpoint create form
  (dropdown or text input with known values)

## Connector router URL question

The lowerer builds `HttpEndpoint`s pointing at the connector
router. The connector router is embedded in the API server at
`/connector`. So the URL would be something like
`http://127.0.0.1:3000/connector` (the API server's own address).
This needs a config field — the lowerer needs to know the
connector router's URL as seen from the mechanics worker.

## Valid implementation names

- `llm_openai_compat`
- `llm_anthropic`
- `llm_gemini`
- `http_forward`
- `sql_postgres`
- `sql_mysql`
- `embed`
- `vector_search`
- `email_smtp`
