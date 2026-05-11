# Workflow Authoring Guide

This guide explains how to create workflow templates, bind them
to endpoints and embedding datasets, run instances, and author
scripts that work with the WebUI chat testing flow.

Implementation files are authoritative for runtime behavior. The
main script argument is currently:

```javascript
{
  context,
  args,
  input,
  subject,
  data
}
```

`data` is always present. It is `{}` when the template has no
`data_config`, and it contains workflow-bound data such as
`data.embed_datasets.<name>` when the template declares data
bindings.

## What workflows are

A **workflow template** is reusable automation logic. It stores:

| Field | Meaning |
|---|---|
| `display_name` | Admin-visible name. |
| `script_source` | ECMAScript module source. The default export runs once per step. |
| `abstract_config` | JSON map from script-local endpoint names to endpoint config UUIDs. |
| `data_config` | Optional JSON data bindings, currently `embed_datasets`. |

A **workflow instance** is a running copy of a template. It is
created with immutable `args`, carries mutable `context`, and is
bound to the template revision current at creation time. Later
template edits do not change existing instances.

A **step** is one execution of the template script. Each step
receives `input`, returns `output`, and may update `context`.
Successful steps write step records. Script failures write a
failed step record and move the instance to `Failed`; executor
transport failures are retryable and do not write records.

Instance statuses are stable integer states in the workflow
layer:

| Status | Meaning |
|---|---|
| `Pending` | Created, no successful step has run. |
| `Running` | At least one step ran and the instance is not terminal. |
| `Completed` | Caller completed the instance, or a script returned `done: true`. |
| `Failed` | Script execution failed or returned malformed output. |
| `Cancelled` | Caller cancelled the instance. |

## Setting up endpoints

Endpoint configs are tenant resources created with
`POST /v1/endpoints`. The API request shape is:

```http
POST /v1/endpoints
Authorization: Bearer pht_<token>
X-Tenant-Id: <tenant-uuid>
Content-Type: application/json
```

```json
{
  "display_name": "Endpoint name",
  "implementation": "llm_openai_compat",
  "config": {}
}
```

The `implementation` field is plaintext metadata. The `config`
blob is encrypted at rest under the substrate credential key.

> **WebUI users**: the per-implementation snippets below show
> the full API body. The WebUI's endpoint Create form has
> separate inputs for **Display name** and **Implementation**;
> only paste the **inner `config` object** (e.g.
> `{"model_id": "bge-m3", ...}`, **not** the whole
> `{"display_name": ..., "implementation": ..., "config":
> {...}}` envelope) into the **Config JSON** editor.
> Connectors set `#[serde(deny_unknown_fields)]` on their
> config structs, so a doubled envelope produces an
> `unknown field 'config'` error.

### `llm_openai_compat`

Use this implementation for providers that expose an
OpenAI-compatible chat-completions API, including vLLM and
providers that require extra fixed headers.

```json
{
  "display_name": "Primary chat model",
  "implementation": "llm_openai_compat",
  "config": {
    "base_url": "https://api.openai.com/v1",
    "api_key": "sk-...",
    "dialect": "openai_native",
    "timeout_ms": 60000,
    "custom_headers": {
      "OpenAI-Organization": "org_..."
    }
  }
}
```

| Field | Required | Notes |
|---|---|---|
| `base_url` | yes | Provider base URL. |
| `api_key` | yes | Sent as `Authorization: Bearer <api_key>` upstream. |
| `dialect` | yes | `openai_native`, `vllm_native`, `tool_call_fallback`, or `tool_call_fallback_auto`. The two `tool_call_fallback*` variants both ship the same `emit_output` function-tool fallback for providers without native structured-output support; the `_auto` variant sends `tool_choice: "auto"` instead of the forced `{type: "function", function: {name: ...}}` literal, for upstreams that reject the forced form. |
| `timeout_ms` | no | Defaults to `60000`. |
| `custom_headers` | no | Extra fixed upstream headers. Reserved headers such as `Authorization`, `Content-Type`, `Content-Length`, `Host`, `Transfer-Encoding`, and `Connection` are rejected. |

The connector normalizes successful `llm_generate` responses to:

```json
{
  "output": {},
  "stop_reason": "end_turn",
  "usage": {
    "input_tokens": 0,
    "output_tokens": 0
  }
}
```

Every LLM request must include `output_schema`. Scripts read the
structured result through `response.body.output`.

**Request body** (passed as the `body` option to
`endpoint(...)`):

```json
{
  "model": "string",
  "messages": [
    { "role": "system", "content": "string" },
    { "role": "user", "content": "string" },
    { "role": "assistant", "content": "string" }
  ],
  "output_schema": { },
  "max_output_tokens": 0,
  "temperature": 0.0,
  "top_p": 0.0,
  "stop": ["string"]
}
```

| Field | Required | Notes |
|---|---|---|
| `model` | yes | Provider model identifier. |
| `messages` | yes | Chronological conversation. Allowed roles: `system`, `user`, `assistant` only. |
| `output_schema` | yes | JSON Schema. The connector enforces structured output via the dialect's native or fallback path. |
| `max_output_tokens` | no | Generation token cap. |
| `temperature` | no | Sampling temperature. |
| `top_p` | no | Nucleus-sampling parameter. |
| `stop` | no | Stop sequences. |
| any other field | — | Rejected at deserialize time (`deny_unknown_fields`). |

**Response body** (read as `response.body`):

```json
{
  "output": { },
  "stop_reason": "end_turn",
  "usage": { "input_tokens": 0, "output_tokens": 0 }
}
```

| Field | Notes |
|---|---|
| `output` | JSON value matching the request's `output_schema`. |
| `stop_reason` | One of `end_turn`, `max_tokens`, `stop_sequence`, `content_filter`, `error`. |
| `usage.input_tokens` | Prompt tokens reported by the provider (u32; `0` if the provider omitted it). |
| `usage.output_tokens` | Completion tokens reported by the provider (u32). |

### `http_forward`

Use `http_forward` for generic HTTP services. Its config wraps
the shared `mechanics-config` `HttpEndpoint` JSON shape.

```json
{
  "display_name": "External API",
  "implementation": "http_forward",
  "config": {
    "endpoint": {
      "method": "post",
      "url_template": "https://api.example.com/v1/{resource}",
      "url_param_specs": {
        "resource": {
          "min_bytes": 1,
          "max_bytes": 64
        }
      },
      "headers": {
        "Authorization": "Bearer service-token"
      },
      "overridable_request_headers": ["Idempotency-Key"],
      "exposed_response_headers": ["X-Request-Id"],
      "request_body_type": "json",
      "response_body_type": "json",
      "response_max_bytes": 1048576,
      "timeout_ms": 10000,
      "allow_non_2xx_status": false
    }
  }
}
```

`response_max_bytes` is required by the implementation. Every
`{slot}` in `url_template` must have a matching
`url_param_specs` entry.

**Request body**: the script's `body` option becomes the
**upstream** HTTP request body, encoded per the endpoint
config's `request_body_type`:

- `json` (default): serialised as JSON.
- `utf8`: passed as a UTF-8 string.
- `bytes`: passed as raw bytes (`Uint8Array`).

URL slot values come from `urlParams`, query parameters from
`queries`, request headers from `headers` (subject to
`overridable_request_headers`). `http_forward` is the only
connector for which `headers`, `urlParams`, and `queries`
are meaningful — the other connectors ignore them.

**Response body** (`response.body`): for `http_forward`,
this is the `HttpForwardResponse` envelope wrapping the
upstream response — i.e. **double-nested** relative to the
mechanics-core transport envelope. Access the upstream's
body via `response.body.body`, not `response.body`.

```json
{
  "status": 200,
  "ok": true,
  "headers": { "x-trace-id": "..." },
  "body": { }
}
```

| Field | Notes |
|---|---|
| `body` | Upstream response body. Decoded per `response_body_type` (`json` → JSON value, `utf8` → string, `bytes` → `Uint8Array`, empty → `null`). |
| `headers` | Upstream response headers, filtered by `exposed_response_headers`; names lowercased. |
| `status` | Upstream HTTP status code. |
| `ok` | `true` if the upstream returned a 2xx status. |

### `sql_postgres` and `sql_mysql`

Use these implementations for SQL query endpoints.

```json
{
  "display_name": "Analytics PostgreSQL",
  "implementation": "sql_postgres",
  "config": {
    "connection_url": "postgres://user:pass@host/db",
    "max_connections": 10,
    "default_timeout_ms": 30000
  }
}
```

```json
{
  "display_name": "Analytics MySQL",
  "implementation": "sql_mysql",
  "config": {
    "connection_url": "mysql://user:pass@host/db",
    "max_connections": 10,
    "default_timeout_ms": 30000
  }
}
```

| Field | Required | Notes |
|---|---|---|
| `connection_url` | yes | PostgreSQL accepts `postgres://` or `postgresql://`; MySQL accepts `mysql://` or `mariadb://`. |
| `max_connections` | no | Defaults to `10`; must be greater than zero when provided. |
| `default_timeout_ms` | no | Defaults to `30000`; must be greater than zero when provided. |

**Request body** (passed as the `body` option to
`endpoint(...)`):

```json
{
  "sql": "SELECT name FROM users WHERE id = $1",
  "params": ["u_12345"],
  "max_rows": 1000,
  "timeout_ms": 30000
}
```

| Field | Required | Notes |
|---|---|---|
| `sql` | yes | SQL text. PostgreSQL uses `$1`, `$2`, ... positional placeholders; MySQL uses `?` positional placeholders. |
| `params` | no | Positional parameter values (JSON array). Defaults to `[]`. |
| `max_rows` | no | Per-request row cap; the effective cap is the lower of this and the endpoint's `default_max_rows` (PostgreSQL) / `max_rows` config (MySQL). |
| `timeout_ms` | no | Per-request timeout in milliseconds; clamped to the endpoint's `default_timeout_ms`. |

**Response body** (read as `response.body`):

```json
{
  "rows": [
    { "id": "u_1", "name": "Alice" }
  ],
  "row_count": 1,
  "columns": [
    { "name": "id", "sql_type": "text" },
    { "name": "name", "sql_type": "text" }
  ],
  "truncated": false
}
```

| Field | Notes |
|---|---|
| `rows` | One JSON object per row. Keys are column names; values are typed per the SQL type. Column order on the wire is not preserved in `rows`; use `columns` for the declared order. |
| `row_count` | Number of rows returned, or affected rows for DML (UPDATE/INSERT/DELETE). |
| `columns` | Ordered column metadata: `[{ "name", "sql_type" }]`. PostgreSQL `sql_type` is the Postgres type name; MySQL `sql_type` is normalized to lowercase. |
| `truncated` | `true` if `max_rows` clipped at least one row from the result. |

### `embed`

Use `embed` to turn text into embedding vectors.

```json
{
  "display_name": "BGE embedder",
  "implementation": "embed",
  "config": {
    "model_id": "bge-m3",
    "max_batch_size": 32,
    "timeout_ms": 10000
  }
}
```

| Field | Required | Notes |
|---|---|---|
| `model_id` | yes | Must match the loaded model. |
| `max_batch_size` | no | Defaults to `32`; request validation rejects oversized batches. |
| `timeout_ms` | no | Defaults to `10000`. |

**Request body** (passed as the `body` option to
`endpoint(...)`):

```json
{ "texts": ["string", "string"] }
```

| Field | Required | Notes |
|---|---|---|
| `texts` | yes | Non-empty array of UTF-8 strings. Validation: length must be `1..=max_batch_size` (config-side cap). |
| any other field | — | Rejected at deserialize time (`deny_unknown_fields`). |

**Response body** (read as `response.body`):

```json
{
  "embeddings": [[0.1, -0.2, 0.3]],
  "model": "bge-m3",
  "dimensions": 3
}
```

| Field | Notes |
|---|---|
| `embeddings` | One vector per input text, same order as the input array. Each vector is an array of f32 components. |
| `model` | Echoed model identifier (matches the endpoint config's `model_id`). |
| `dimensions` | Vector dimensionality. Equal to `embeddings[i].length` for every `i`. |

### `vector_search`

Use `vector_search` for request-local cosine nearest-neighbor
search. It does not persist vectors; the workflow passes a
corpus on every call.

```json
{
  "display_name": "Vector search",
  "implementation": "vector_search",
  "config": {
    "max_corpus_size": 10000,
    "timeout_ms": 2000
  }
}
```

| Field | Required | Notes |
|---|---|---|
| `max_corpus_size` | yes | Maximum `corpus.length` accepted per request. |
| `timeout_ms` | no | Defaults to `2000`. |

The request body is:

```json
{
  "query_vector": [0.1, 0.2],
  "corpus": [
    {
      "id": "doc-1",
      "vector": [0.1, 0.2],
      "payload": {
        "text": "Document text"
      }
    }
  ],
  "top_k": 5,
  "score_threshold": 0.2
}
```

The response body is `{ "results": [{ "id", "score",
"payload" }] }`. `payload` is omitted from a result when the
corpus item had no payload.

**Request body** (passed as the `body` option to
`endpoint(...)`):

```json
{
  "query_vector": [0.1, 0.2, 0.3],
  "corpus": [
    {
      "id": "doc-1",
      "vector": [0.1, 0.2, 0.3],
      "payload": { "text": "Document text" }
    }
  ],
  "top_k": 5,
  "score_threshold": 0.2
}
```

| Field | Required | Notes |
|---|---|---|
| `query_vector` | yes | Embedding to score the corpus against. Array of f32. |
| `corpus` | yes | Per-request corpus of labeled vectors. Length must be `0..=max_corpus_size` (config-side cap). |
| `corpus[].id` | yes | Caller-defined stable identifier. Echoed in matching results. |
| `corpus[].vector` | yes | Candidate vector scored against `query_vector`. Must have the same dimensionality as `query_vector`. |
| `corpus[].payload` | no | Optional JSON value echoed in matching results. Omit by leaving the field out. |
| `top_k` | yes | Number of highest-scoring items to return. |
| `score_threshold` | no | Lower bound for accepted cosine scores. Range `[-1.0, 1.0]`. Results below this are filtered out. |

**Response body** (read as `response.body`):

```json
{
  "results": [
    { "id": "doc-1", "score": 0.95, "payload": { "text": "Document text" } }
  ]
}
```

| Field | Notes |
|---|---|
| `results` | Ranked nearest-neighbor matches, highest score first. May be empty. |
| `results[].id` | The matching corpus item's `id`. |
| `results[].score` | Cosine similarity in `[-1.0, 1.0]`. Higher = more similar. |
| `results[].payload` | Echoed from the matching corpus item; omitted entirely when the matched item had no payload. |

### Reserved connector names

These crates are reserved but not implemented in this workspace
version:

| Implementation | State |
|---|---|
| `email_smtp` | Reserved / pending implementation. |
| `llm_anthropic` | Reserved / pending implementation. |
| `llm_gemini` | Reserved / pending implementation. |

Do not create endpoint configs for these names until their
implementation crates ship real behavior.

## Creating a template

Create templates through the API when you need `data_config`.
The current WebUI template form exposes display name, script
source, and `abstract_config`; it does not expose a structured
`data_config` editor.

```http
POST /v1/workflows/templates
Authorization: Bearer pht_<token>
X-Tenant-Id: <tenant-uuid>
Content-Type: application/json
```

```json
{
  "display_name": "Echo",
  "script_source": "export default function(arg) { return { output: arg.input, context: arg.context, done: true }; }",
  "abstract_config": {},
  "data_config": {
    "embed_datasets": {}
  }
}
```

For a template that calls endpoints and reads an embedding
dataset:

```json
{
  "display_name": "Knowledge chat",
  "script_source": "<script source>",
  "abstract_config": {
    "llm": "<llm-endpoint-uuid>",
    "embed": "<embed-endpoint-uuid>",
    "vector_search": "<vector-search-endpoint-uuid>"
  },
  "data_config": {
    "embed_datasets": {
      "knowledge_base": "<embedding-dataset-uuid>"
    }
  }
}
```

Dataset binding names must be JavaScript-property-like names:
1 to 64 bytes, starting with an ASCII letter, `_`, or `$`, and
continuing with ASCII letters, digits, `_`, or `$`.

`PATCH /v1/workflows/templates/{id}` appends a new template
revision. Existing instances stay pinned to their original
template revision; new instances use the latest revision.

## Writing scripts

Scripts are ECMAScript modules executed by the Boa JavaScript
engine. The module default export must be a function, and may
be `async`.

```javascript
export default async function main(arg) {
  return {
    output: {},
    context: arg.context,
    done: false
  };
}
```

### Script argument

```javascript
{
  context: {}, // mutable state from previous successful steps
  args: {},    // immutable instance creation arguments
  input: {},   // per-step input
  subject: {}, // caller identity
  data: {}     // workflow-bound data, always present
}
```

`subject` serializes the authenticated caller context. Principal
callers and ephemeral-token callers both reach scripts through
this field; scripts that do not need caller identity can ignore
it.

### Return value

```javascript
{
  output: {},
  context: {},
  done: false
}
```

| Field | Required | Meaning |
|---|---|---|
| `output` | yes | Stored on the step record and returned to the caller. |
| `context` | yes | Replaces the instance context for the next step. |
| `done` | no | `true` completes the instance; absent or false keeps it running. |

Bounded workflows usually return `done: true` after producing
their final output. Chat workflows usually keep `done: false`.

### Built-in modules

Scripts can import only built-in modules exposed by
`mechanics-core/ts-types/`.

| Module | Exports |
|---|---|
| `mechanics:endpoint` | Default `endpoint(name, options)` call helper. |
| `mechanics:base64` | `encode`, `decode`; variants `base64`, `base64url`. |
| `mechanics:base32` | `encode`, `decode`; variants `base32`, `base32hex`. |
| `mechanics:hex` | `encode`, `decode`. |
| `mechanics:form-urlencoded` | `encode`, `decode`. |
| `mechanics:rand` | Default `fillRandom(buffer)` helper. |
| `mechanics:uuid` | Default UUID generator for `v3`, `v4`, `v5`, `v6`, `v7`, `nil`, or `max`. |

### Sandbox limits

Scripts run in an isolated JavaScript realm:

| Limit | Current default |
|---|---|
| Filesystem access | None. |
| Network access | Only through `mechanics:endpoint`. |
| Cross-step JavaScript globals | None; persist state in `context`. |
| Wall-clock timeout | Worker-pool or per-job configured. `mechanics-core` pool default is 30 seconds; the mechanics-worker binary can set deployment defaults. |
| Loop iteration limit | `mechanics-core` default is `1_000_000`. |
| Recursion depth limit | `mechanics-core` default is `512`. |
| Stack size limit | `mechanics-core` default is `10 * 1024` bytes. |

## Calling endpoints

Use `mechanics:endpoint` to call script-local endpoint names
from `abstract_config`.

```javascript
import endpoint from "mechanics:endpoint";

export default async function main(arg) {
  const response = await endpoint("llm", {
    body: {
      model: "default",
      messages: [
        { role: "user", content: arg.input.question }
      ],
      output_schema: {
        type: "object",
        properties: {
          answer: { type: "string" }
        },
        required: ["answer"],
        additionalProperties: false
      }
    }
  });

  return {
    output: { answer: response.body.output.answer },
    context: arg.context,
    done: true
  };
}
```

Endpoint options:

| Option | Meaning |
|---|---|
| `body` | The connector's typed request body for typed connectors (LLM, embed, vector_search, SQL); the upstream HTTP request body for `http_forward`. |
| `headers` | Extra request headers; `http_forward` only — typed connectors ignore this. Restricted to the endpoint's `overridable_request_headers` allowlist. Header matching is case-insensitive. |
| `urlParams` | URL template slot values; `http_forward` only — typed connectors ignore this. |
| `queries` | Query string slot values; `http_forward` only — typed connectors ignore this. |

Endpoint response (mechanics-core transport envelope, identical
shape for every connector):

| Field | Meaning |
|---|---|
| `body` | The JSON value the connector implementation returned. **Shape is connector-specific** — see each connector's "Response body" subsection above for the contents. For typed connectors this is the typed response struct; for `http_forward` it is the `HttpForwardResponse` envelope (double-nested — see that connector's notes). |
| `headers` | Connector-path response headers exposed by mechanics-core (lowercased names). For typed connectors, normally empty; for `http_forward`, the connector populates `response.body.headers` instead with upstream headers. |
| `status` | HTTP status code from the **connector path** call (mechanics-worker → connector-router → connector-service). `200` on a successful call. For `http_forward`, the upstream's status is `response.body.status`, not `response.status`. |
| `ok` | `true` if `status` is in `200..=299`. |

Connector-side errors (HTTP 4xx/5xx from the connector path,
e.g. an upstream LLM returning 400, an invalid SQL query) reach
the script as a **rejected promise**. Use `try { await
endpoint(...) } catch (e) { ... }` to handle them — the engine
trusts the script's outcome once `main()` fulfills, so a
caught rejection is genuinely caught (see `mechanics-core`
0.4.0 release notes).

## Reading embedding datasets

Embedding datasets let admins precompute vectors for a tenant
corpus and bind the resulting corpus to workflow templates.
Scripts then receive the corpus at step-execution time.

### Dataset setup flow

1. Create an `embed` endpoint.
2. Create the embedding dataset with source items.
3. Wait for status `Ready`, or handle absence defensively.
4. Bind the dataset UUID in the workflow template's
   `data_config.embed_datasets`.
5. Re-submit items with the update endpoint when corpus content
   changes.

Create a dataset:

```http
POST /v1/embed-datasets
Authorization: Bearer pht_<token>
X-Tenant-Id: <tenant-uuid>
Content-Type: application/json
```

```json
{
  "display_name": "Product knowledge base",
  "embed_endpoint_id": "<embed-endpoint-uuid>",
  "items": [
    {
      "id": "returns-001",
      "text": "Returns are accepted within 30 days with a receipt.",
      "payload": {
        "title": "Return policy",
        "source_url": "https://example.test/policies/returns",
        "chunk_index": 0,
        "text": "Returns are accepted within 30 days with a receipt."
      }
    }
  ]
}
```

Update source items and trigger re-embed:

```http
POST /v1/embed-datasets/{id}/update
Authorization: Bearer pht_<token>
X-Tenant-Id: <tenant-uuid>
Content-Type: application/json
```

```json
{
  "items": [
    {
      "id": "returns-001",
      "text": "Returns are accepted within 45 days with a receipt.",
      "payload": {
        "title": "Return policy",
        "source_url": "https://example.test/policies/returns",
        "chunk_index": 0,
        "text": "Returns are accepted within 45 days with a receipt."
      }
    }
  ]
}
```

The API rejects updates with `409 Conflict` while a dataset is
already `Embedding`.

### Runtime data shape

Template binding:

```json
{
  "data_config": {
    "embed_datasets": {
      "knowledge_base": "<embedding-dataset-uuid>"
    }
  }
}
```

Script access:

```javascript
const corpus = arg.data.embed_datasets?.knowledge_base || [];
```

Each corpus item is:

```typescript
type CorpusItem = {
  id: string;
  vector: number[];
  payload?: unknown;
};
```

`vector` is a JSON number array of f32 components. `payload` is
the optional JSON value from the source item and is omitted
entirely when absent.

### Availability states

Any data field can be absent without failing the workflow
engine. JavaScript decides whether absence is recoverable.

| Dataset state | What the script sees |
|---|---|
| First embed in progress (`Created` or `Embedding`, no prior corpus) | Dataset key is omitted. |
| Re-embed in progress with prior corpus | Previous corpus remains visible. |
| Latest re-embed failed with prior corpus | Previous corpus remains visible indefinitely until the next success. |
| First embed failed with no prior corpus | Dataset key is omitted. |
| Retired dataset | Dataset key is omitted. |

Defensive workflow pattern:

```javascript
function getCorpus(arg, name) {
  const datasets = arg.data.embed_datasets || {};
  const corpus = datasets[name];
  return Array.isArray(corpus) ? corpus : [];
}
```

## Authoring a chat workflow

The WebUI chat tab detects compatibility at runtime. It executes
the instance once with empty input:

```json
{
  "input": {}
}
```

If the returned step output has a `messages` array, and every
message is an object with non-empty string `role` and string
`content`, the workflow is treated as chat-compatible.

The chat UI sends user turns as:

```json
{
  "input": {
    "content": "Hello"
  }
}
```

The workflow must return the full transcript every time:

```javascript
return {
  output: {
    messages: [
      { role: "assistant", content: "Hello." },
      { role: "user", content: "What can you do?" }
    ]
  },
  context: { messages },
  done: false
};
```

`role` may be `user`, `assistant`, `system`, or another
non-empty string. Extra per-message fields are preserved by the
WebUI parser as opaque passthrough.

One extra field has special UI treatment: on **assistant**
turns, a `name` field (OpenAI-style assistant-persona
identifier) — when present and a non-empty string — is
rendered as the bubble's role label in place of the generic
"Assistant" / "アシスタント" string. Other roles' `name`
fields are kept in the transcript object but not surfaced in
the bubble label. Use this when the workflow distinguishes
multiple assistant personas, e.g. a triage agent vs. a
specialist agent in a multi-step bot.

### Empty-content semantics

The empty input branch is dual-purpose:

| Instance state | Expected behavior |
|---|---|
| Fresh instance with no transcript | Generate the opening turn, usually a greeting. |
| Existing instance with transcript | Return the current transcript unchanged. |

Chat workflows generally do not return `done: true`. The chat
tab keeps the conversation open. If the conversation has a
natural end, use the instance detail page's Complete action or
return a final assistant message with `done: true` knowing that
the chat tab will stop being useful after terminal state.

### Full D13-compatible chat script

This script uses the state-driven accumulator pattern.

```javascript
import endpoint from "mechanics:endpoint";

function normalizeMessages(value) {
  return Array.isArray(value) ? value.slice() : [];
}

function transcript(messages) {
  return {
    output: { messages },
    context: { messages },
    done: false
  };
}

export default async function main(arg) {
  const messages = normalizeMessages(arg.context.messages);
  const content = typeof arg.input.content === "string" ? arg.input.content.trim() : "";

  if (content.length === 0) {
    if (messages.length === 0) {
      messages.push({
        role: "assistant",
        content: "Hello. Ask me a question and I will answer concisely."
      });
    }
    return transcript(messages);
  }

  messages.push({ role: "user", content });

  const response = await endpoint("llm", {
    body: {
      model: "default",
      messages,
      output_schema: {
        type: "object",
        properties: {
          reply: { type: "string" }
        },
        required: ["reply"],
        additionalProperties: false
      }
    }
  });

  messages.push({
    role: "assistant",
    content: response.body.output.reply
  });

  return transcript(messages);
}
```

Template:

```json
{
  "display_name": "D13 chat",
  "script_source": "<paste the script above>",
  "abstract_config": {
    "llm": "<llm-endpoint-uuid>"
  }
}
```

WebUI behavior:

| Action | Result |
|---|---|
| Templates list row action, Test in chat | Creates an instance with `{}` args and opens `/instances/{id}?tab=chat`. |
| Template detail, Test in chat | Opens the last-used chat instance or starts a new one. |
| Chat tab mount | Executes `{input: {}}` as the probe. |
| Output shape mismatch | Shows the not-chat-compatible empty state with observed top-level keys. |
| Transport or script error | Shows a toast / error state and offers fallback to the normal Execute panel. |

Permissions for this recipe:

| Operation | Permission |
|---|---|
| Create LLM endpoint | `endpoint:create` |
| Create template | `workflow:template_create` |
| Create instance | `workflow:instance_create` |
| Execute chat turns | `workflow:instance_execute` |
| Read instance / steps in WebUI | `workflow:instance_read` |

## Authoring an embedding-datasets workflow

This recipe reads a bound corpus and runs vector search against
a query vector. It returns a one-step answer and completes.

Endpoint setup:

```json
[
  {
    "display_name": "BGE embedder",
    "implementation": "embed",
    "config": {
      "model_id": "bge-m3",
      "max_batch_size": 32,
      "timeout_ms": 10000
    }
  },
  {
    "display_name": "Vector search",
    "implementation": "vector_search",
    "config": {
      "max_corpus_size": 10000,
      "timeout_ms": 2000
    }
  }
]
```

Dataset creation:

```json
{
  "display_name": "Support articles",
  "embed_endpoint_id": "<embed-endpoint-uuid>",
  "items": [
    {
      "id": "shipping-001",
      "text": "Standard shipping takes three to five business days.",
      "payload": {
        "title": "Shipping",
        "text": "Standard shipping takes three to five business days."
      }
    },
    {
      "id": "returns-001",
      "text": "Returns require a receipt and must be started within 30 days.",
      "payload": {
        "title": "Returns",
        "text": "Returns require a receipt and must be started within 30 days."
      }
    }
  ]
}
```

Full script:

```javascript
import endpoint from "mechanics:endpoint";

function corpusFrom(arg, name) {
  const datasets = arg.data.embed_datasets || {};
  const corpus = datasets[name];
  return Array.isArray(corpus) ? corpus : [];
}

function resultPayload(result) {
  if (result.payload && typeof result.payload === "object") {
    return result.payload;
  }
  return {};
}

export default async function main(arg) {
  const question = typeof arg.input.question === "string" ? arg.input.question.trim() : "";
  if (question.length === 0) {
    return {
      output: {
        error: "question is required"
      },
      context: arg.context,
      done: true
    };
  }

  const corpus = corpusFrom(arg, "knowledge_base");
  if (corpus.length === 0) {
    return {
      output: {
        error: "knowledge base is not ready"
      },
      context: arg.context,
      done: true
    };
  }

  const embedResponse = await endpoint("embed", {
    body: {
      texts: [question]
    }
  });
  const queryVector = embedResponse.body.embeddings[0];

  const searchResponse = await endpoint("vector_search", {
    body: {
      query_vector: queryVector,
      corpus,
      top_k: 3
    }
  });

  const matches = searchResponse.body.results.map((result) => {
    const payload = resultPayload(result);
    return {
      id: result.id,
      score: result.score,
      title: typeof payload.title === "string" ? payload.title : result.id,
      text: typeof payload.text === "string" ? payload.text : ""
    };
  });

  return {
    output: {
      question,
      matches
    },
    context: arg.context,
    done: true
  };
}
```

Template:

```json
{
  "display_name": "Knowledge base search",
  "script_source": "<paste the script above>",
  "abstract_config": {
    "embed": "<embed-endpoint-uuid>",
    "vector_search": "<vector-search-endpoint-uuid>"
  },
  "data_config": {
    "embed_datasets": {
      "knowledge_base": "<embedding-dataset-uuid>"
    }
  }
}
```

Execute:

```json
{
  "input": {
    "question": "How long does shipping take?"
  }
}
```

WebUI behavior:

| State | Result |
|---|---|
| Dataset first embed in progress | Dataset detail shows embedding state; workflow returns `knowledge base is not ready` from the defensive branch. |
| Dataset ready | Workflow returns ranked matches. |
| Dataset re-embedding | Workflow keeps using the prior corpus. |
| Dataset failed after previous success | Workflow keeps using the prior corpus. |
| Dataset failed before any success | Dataset key is absent; workflow returns the defensive error. |

Permissions:

| Operation | Permission |
|---|---|
| Create embed / vector endpoints | `endpoint:create` |
| Create dataset | `embed_dataset:create` |
| Read dataset status/source/corpus in WebUI | `embed_dataset:read` |
| Update dataset | `embed_dataset:update` |
| Create template | `workflow:template_create` |
| Create / execute / read instance | `workflow:instance_create`, `workflow:instance_execute`, `workflow:instance_read` |

## Authoring chat with embedding datasets

This is the common RAG shape: the chat UI provides turns, the
workflow embeds each user message, searches the bound corpus,
and asks the LLM to answer with retrieved context.

Setup requires three endpoints:

```json
[
  {
    "display_name": "Chat model",
    "implementation": "llm_openai_compat",
    "config": {
      "base_url": "https://api.openai.com/v1",
      "api_key": "sk-...",
      "dialect": "openai_native",
      "timeout_ms": 60000,
      "custom_headers": {}
    }
  },
  {
    "display_name": "BGE embedder",
    "implementation": "embed",
    "config": {
      "model_id": "bge-m3",
      "max_batch_size": 32,
      "timeout_ms": 10000
    }
  },
  {
    "display_name": "Vector search",
    "implementation": "vector_search",
    "config": {
      "max_corpus_size": 10000,
      "timeout_ms": 2000
    }
  }
]
```

Create an embedding dataset as in the previous recipe, wait for
`Ready` when possible, and bind it to the template as
`knowledge_base`.

Full script:

```javascript
import endpoint from "mechanics:endpoint";

function normalizeMessages(value) {
  return Array.isArray(value) ? value.slice() : [];
}

function transcript(messages) {
  return {
    output: { messages },
    context: { messages },
    done: false
  };
}

function corpusFrom(arg, name) {
  const datasets = arg.data.embed_datasets || {};
  const corpus = datasets[name];
  return Array.isArray(corpus) ? corpus : [];
}

function payloadText(result) {
  const payload = result.payload;
  if (payload && typeof payload === "object" && typeof payload.text === "string") {
    return payload.text;
  }
  return "";
}

function contextBlock(results) {
  const lines = [];
  for (const result of results) {
    const text = payloadText(result);
    if (text.length > 0) {
      lines.push(`- ${text}`);
    }
  }
  return lines.join("\n");
}

export default async function main(arg) {
  const messages = normalizeMessages(arg.context.messages);
  const content = typeof arg.input.content === "string" ? arg.input.content.trim() : "";

  if (content.length === 0) {
    if (messages.length === 0) {
      messages.push({
        role: "assistant",
        content: "Hello. Ask me about the knowledge base."
      });
    }
    return transcript(messages);
  }

  messages.push({ role: "user", content });

  const corpus = corpusFrom(arg, "knowledge_base");
  if (corpus.length === 0) {
    messages.push({
      role: "assistant",
      content: "The knowledge base is still being embedded. Try again after it is ready."
    });
    return transcript(messages);
  }

  const embedResponse = await endpoint("embed", {
    body: {
      texts: [content]
    }
  });
  const queryVector = embedResponse.body.embeddings[0];

  const searchResponse = await endpoint("vector_search", {
    body: {
      query_vector: queryVector,
      corpus,
      top_k: 5,
      score_threshold: 0.1
    }
  });

  const retrievedContext = contextBlock(searchResponse.body.results);

  const llmMessages = [
    {
      role: "system",
      content: `Answer using the retrieved context. If the context is empty, say you do not know.\n\n${retrievedContext}`
    },
    ...messages
  ];

  const llmResponse = await endpoint("llm", {
    body: {
      model: "default",
      messages: llmMessages,
      output_schema: {
        type: "object",
        properties: {
          reply: { type: "string" }
        },
        required: ["reply"],
        additionalProperties: false
      }
    }
  });

  messages.push({
    role: "assistant",
    content: llmResponse.body.output.reply,
    retrieved_count: searchResponse.body.results.length
  });

  return transcript(messages);
}
```

Template:

```json
{
  "display_name": "Knowledge chat",
  "script_source": "<paste the script above>",
  "abstract_config": {
    "llm": "<llm-endpoint-uuid>",
    "embed": "<embed-endpoint-uuid>",
    "vector_search": "<vector-search-endpoint-uuid>"
  },
  "data_config": {
    "embed_datasets": {
      "knowledge_base": "<embedding-dataset-uuid>"
    }
  }
}
```

What you will see in the WebUI:

| Step | Result |
|---|---|
| Click Test in chat from the template list or detail page | WebUI creates or opens an instance and navigates to the Chat tab. |
| Chat tab opens on a fresh instance | It sends `{input: {}}`; the script returns the greeting. |
| Send a message | The UI sends `{input: {content: "..."}}`; the script returns the full transcript. |
| Dataset absent | The script adds an assistant message saying the knowledge base is not ready, while preserving chat compatibility. |
| Output shape drifts | The chat tab switches to not-chat-compatible state because detection is based on `output.messages`. |

Permissions are the union of the chat and embedding-dataset
recipes: endpoint create/read as needed, `embed_dataset:create`,
`embed_dataset:read`, `embed_dataset:update` for corpus
management, and workflow template/instance permissions for
testing.

## Running a workflow

For the integrator-side flow (creating instances on behalf
of end users, minting a workflow-run-only ephemeral token
bound to the instance, and re-minting on session resume
beyond 24 hours) see
[`end-user-session-tokens.md`](end-user-session-tokens.md).
The summary below covers the bare API shapes; the linked
guide covers the credential separation, instance scoping,
and re-mint flow.

Create an instance:

```http
POST /v1/workflows/instances
Authorization: Bearer pht_<token>
X-Tenant-Id: <tenant-uuid>
Content-Type: application/json
```

```json
{
  "template_id": "<template-uuid>",
  "args": {}
}
```

Execute a step:

```http
POST /v1/workflows/instances/{id}/execute
Authorization: Bearer pht_<token>
X-Tenant-Id: <tenant-uuid>
Content-Type: application/json
```

```json
{
  "input": {
    "content": "Hello"
  }
}
```

Read instance state:

```http
GET /v1/workflows/instances/{id}
Authorization: Bearer pht_<token>
X-Tenant-Id: <tenant-uuid>
```

Read revision history:

```http
GET /v1/workflows/instances/{id}/history
Authorization: Bearer pht_<token>
X-Tenant-Id: <tenant-uuid>
```

Read step records:

```http
GET /v1/workflows/instances/{id}/steps
Authorization: Bearer pht_<token>
X-Tenant-Id: <tenant-uuid>
```

Complete or cancel:

```http
POST /v1/workflows/instances/{id}/complete
Authorization: Bearer pht_<token>
X-Tenant-Id: <tenant-uuid>
```

```http
POST /v1/workflows/instances/{id}/cancel
Authorization: Bearer pht_<token>
X-Tenant-Id: <tenant-uuid>
```

## Instance lifecycle

```text
Pending -> Running
Pending -> Completed
Pending -> Cancelled
Pending -> Failed
Running -> Running
Running -> Completed
Running -> Failed
Running -> Cancelled
```

Terminal states have no outgoing transitions. The engine
refuses operations on terminal instances.

## Examples

### Echo

```javascript
export default function main(arg) {
  const step = typeof arg.context.step === "number" ? arg.context.step + 1 : 1;
  return {
    output: {
      echo: arg.input,
      step
    },
    context: {
      step
    },
    done: arg.input.finish === true
  };
}
```

Template config:

```json
{
  "display_name": "Echo",
  "script_source": "<paste the script above>",
  "abstract_config": {}
}
```

### Basic LLM one-shot

```javascript
import endpoint from "mechanics:endpoint";

export default async function main(arg) {
  const question = typeof arg.input.question === "string" ? arg.input.question : "";
  const response = await endpoint("llm", {
    body: {
      model: "default",
      messages: [
        { role: "user", content: question }
      ],
      output_schema: {
        type: "object",
        properties: {
          answer: { type: "string" }
        },
        required: ["answer"],
        additionalProperties: false
      }
    }
  });

  return {
    output: {
      answer: response.body.output.answer
    },
    context: arg.context,
    done: true
  };
}
```

The full D13 chat and embedding-datasets RAG examples are in
the recipe sections above.

## Permissions

Permissions are string atoms attached to roles and ephemeral
token claims. The current policy crate recognizes these atoms:

| Area | Atom | Used for |
|---|---|---|
| Workflow | `workflow:template_create` | Create and patch templates. |
| Workflow | `workflow:template_read` | List/read templates. |
| Workflow | `workflow:template_retire` | Retire templates. |
| Workflow | `workflow:instance_create` | Create instances. |
| Workflow | `workflow:instance_read` | Read instance state, history, and steps. |
| Workflow | `workflow:instance_execute` | Execute steps and complete instances. |
| Workflow | `workflow:instance_cancel` | Cancel instances. |
| Endpoint | `endpoint:create` | Create endpoint configs. |
| Endpoint | `endpoint:rotate` | Rotate endpoint configs. |
| Endpoint | `endpoint:retire` | Retire endpoint configs. |
| Endpoint | `endpoint:read_metadata` | Read endpoint metadata. |
| Endpoint | `endpoint:read_decrypted` | Read decrypted endpoint config. |
| Embedding dataset | `embed_dataset:create` | Create datasets. |
| Embedding dataset | `embed_dataset:read` | List/read datasets, source items, and corpus. |
| Embedding dataset | `embed_dataset:update` | Update source items and trigger re-embed. |
| Embedding dataset | `embed_dataset:retire` | Retire datasets. |
| Tenant | `tenant:principal_manage` | Manage principals. |
| Tenant | `tenant:role_manage` | Manage roles and memberships. |
| Tenant | `tenant:minting_manage` | Manage minting authorities. |
| Minting | `mint:ephemeral_token` | Mint ephemeral API tokens. |
| Tenant | `tenant:settings_read` | Read tenant settings. |
| Tenant | `tenant:settings_manage` | Update tenant settings. |
| Audit | `audit:read` | Read tenant audit events. |
| Deployment | `deployment:tenant_manage` | Manage tenants at operator scope. |
| Deployment | `deployment:realm_manage` | Manage realms at operator scope. |
| Deployment | `deployment:audit_read` | Read deployment audit events. |

Long-lived API tokens use the `pht_` format and are returned
once at creation or rotation. Ephemeral tokens are minted
through `POST /v1/tokens/mint`; requested permissions are
clipped to the minting authority's permission envelope.

API rate limits are enforced at the API layer. Endpoint
rotation keeps the same endpoint UUID, so templates referencing
that UUID pick up the new endpoint revision at the next step.

## Cross-references

- [Workflow orchestration](../design/07-workflow-orchestration.md)
  covers templates, instances, steps, status transitions, and
  engine responsibilities. Its script-argument section is stale
  for `data`; the implementation and design 16 now include it.
- [Connector architecture](../design/08-connector-architecture.md)
  covers capabilities, implementation dispatch, the
  `llm_generate` normalized shape, transport envelope semantics,
  and header allowlists.
- [Policy and tenancy](../design/09-policy-and-tenancy.md)
  covers permission evaluation, principals, minting authorities,
  API token semantics, and tenant scoping.
- [API layer](../design/10-api-layer.md) covers route families,
  authentication, rate limiting, and endpoint rotation.
- [Security and cryptography](../design/11-security-and-cryptography.md)
  covers SCK encryption, COSE connector tokens, encrypted
  payloads, and the trust boundary.
- [Embedding datasets](../design/16-embedding-datasets.md)
  covers dataset lifecycle, CBOR storage, carry-forward
  behavior, and template `data_config`.
- [ROADMAP](../ROADMAP.md) tracks pending implementation work,
  including reserved connector implementations.
