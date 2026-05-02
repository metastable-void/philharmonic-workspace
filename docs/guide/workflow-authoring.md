# Workflow Authoring Guide

This guide explains how to create, deploy, and execute workflows.

## Concepts

A **workflow template** defines reusable automation logic. It
consists of:

- **Script source** — an ECMAScript module whose default export
  is called for each execution step. Runs in a sandboxed Boa
  JavaScript engine.
- **Abstract config** — a JSON object mapping endpoint names to
  **endpoint config UUIDs**. Each UUID references a
  `TenantEndpointConfig` entity created via the Endpoints API.

A **workflow instance** is a running copy of a template, created
with specific **args** (input parameters). Instances maintain
**context** (mutable state that persists across steps) and
progress through a lifecycle: Pending → Running → Completed (or
Failed/Cancelled).

Each **step** is one execution of the script. Steps receive input
and produce output. The script decides whether the workflow is
done (`done: true`) or needs more steps.

## Setting Up Endpoints

Before creating a workflow that calls external services, configure
the endpoint connections:

### 1. Create an endpoint config

The `config` JSON shape depends on which connector implementation
will handle the endpoint. All configs are encrypted at rest using
the SCK — API keys and secrets are never stored in plaintext.

#### LLM (OpenAI-compatible)

```
POST /v1/endpoints
Content-Type: application/json

{
  "display_name": "My LLM",
  "implementation": "llm_openai_compat",
  "config": {
    "base_url": "https://api.openai.com/v1",
    "api_key": "sk-...",
    "dialect": "openai_native",
    "timeout_ms": 60000
  }
}
```

| Field | Required | Description |
|---|---|---|
| `base_url` | yes | Provider base URL |
| `api_key` | yes | Bearer token for the provider |
| `dialect` | yes | `"openai_native"` or `"tool_call_fallback"` |
| `timeout_ms` | no | Per-attempt timeout (default: 60000) |

#### HTTP Forward (generic HTTP)

```json
{
  "display_name": "External API",
  "implementation": "http_forward",
  "config": {
    "endpoint": {
      "method": "post",
      "url_template": "https://api.example.com/v1/{resource}",
      "url_param_specs": {
        "resource": {}
      },
      "headers": { "Authorization": "Bearer ..." },
      "response_body_type": "json",
      "response_max_bytes": 1048576
    }
  }
}
```

The `endpoint` field is a `mechanics-config` `HttpEndpoint`
definition (see `mechanics-core/ts-types/mechanics-json-shapes.d.ts`
for the full TypeScript shape). Every `{name}` placeholder in
`url_template` must have a matching entry in `url_param_specs`
— validation rejects templates with missing specs. The script
later supplies values via the `urlParams` option to
`endpoint(...)`.

#### SQL (PostgreSQL / MySQL)

```json
{
  "display_name": "Analytics DB",
  "implementation": "sql_postgres",
  "config": {
    "connection_url": "postgres://user:pass@host/db",
    "max_connections": 10,
    "default_timeout_ms": 30000,
    "default_max_rows": 10000
  }
}
```

#### Text Embedding

```json
{
  "display_name": "Embedder",
  "implementation": "embed",
  "config": {
    "model_id": "bge-m3",
    "max_batch_size": 32,
    "timeout_ms": 10000
  }
}
```

#### Vector Search

```json
{
  "display_name": "Searcher",
  "implementation": "vector_search",
  "config": {
    "max_corpus_size": 1000,
    "timeout_ms": 2000
  }
}
```

Response: `{ "endpoint_id": "<uuid>" }`

### 2. Use the endpoint UUID in abstract config

```json
{
  "my-llm": "<endpoint-uuid-from-step-1>"
}
```

The keys (`"my-llm"`) become the endpoint names your script uses.

## Creating a Template

### Via the API

```
POST /v1/workflows/templates
Authorization: Bearer pht_...
X-Tenant-Id: <tenant-uuid>
Content-Type: application/json

{
  "display_name": "Echo Bot",
  "script_source": "export default function(arg) { return { output: arg.input, done: true }; }",
  "abstract_config": {}
}
```

For a template that calls an endpoint:

```json
{
  "display_name": "LLM Chat",
  "script_source": "...",
  "abstract_config": {
    "llm": "<endpoint-config-uuid>"
  }
}
```

### Via the WebUI

Navigate to **Templates → Create**. Enter a display name, paste
the script source, and provide the abstract config as JSON.

## Writing Scripts

Scripts are ECMAScript modules executed by the Boa JavaScript
engine. The default export must be a function that receives a
single argument object and returns a result object. The function
may be `async`.

### Script argument

```javascript
{
  context: { /* mutable state from previous steps */ },
  args:    { /* instance creation arguments */ },
  input:   { /* step-specific input */ },
  subject: { /* caller identity information */ }
}
```

### Return value

```javascript
{
  output:  { /* step output (stored as a step record) */ },
  context: { /* updated context for the next step */ },
  done:    true | false
}
```

- `done: true` — marks the instance as Completed.
- `done: false` — keeps the instance Running; another step can
  be executed later.

### Calling external endpoints

Use the built-in `mechanics:endpoint` module. It exports a single
default function:

```javascript
import endpoint from "mechanics:endpoint";

export default async function(arg) {
  const response = await endpoint("my-llm", {
    body: {
      model: "gpt-4o-mini",
      messages: [
        { role: "user", content: arg.input.question }
      ],
      output_schema: {
        type: "object",
        properties: { answer: { type: "string" } },
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

Note: the `llm_generate` capability returns a normalized
`{output, stop_reason, usage}` shape and **requires**
`output_schema` in the request — see the connector
architecture for the full wire protocol. The transport
envelope (`{body, headers, status, ok}` returned by
`endpoint(...)`) wraps that response, so the script reads the
LLM's structured result via `response.body.output.<field>`,
not provider-native fields like `choices[0].message`.

The first argument is the endpoint name (must match a key in the
abstract config). The second argument is an options object:

| Option | Type | Description |
|---|---|---|
| `body` | object \| string \| `Uint8Array` | Request body. Objects are sent as JSON. |
| `headers` | `{ name: value }` | Extra request headers (only those in `overridable_request_headers`). |
| `urlParams` | `{ name: value }` | Values for `{param}` placeholders in the URL template. |
| `queries` | `{ name: value }` | Query string parameters. |

The response object:

| Field | Type | Description |
|---|---|---|
| `body` | object \| string \| `Uint8Array` \| null | Response body, decoded per `response_body_type`. |
| `headers` | `{ name: value }` | Response headers (only those in `exposed_response_headers`). |
| `status` | number | HTTP status code. |
| `ok` | boolean | `true` if status is 2xx. |

### Built-in modules

Scripts have access to these built-in modules. Full TypeScript
definitions are in `mechanics-core/ts-types/`.

#### `mechanics:endpoint`

```typescript
import endpoint from "mechanics:endpoint";
const response = await endpoint("name", {
  body?: unknown,          // JSON value, string, or ArrayBufferView
  headers?: Record<string, string>,
  urlParams?: Record<string, string>,
  queries?: Record<string, string>,
});
// response: { body, headers, status: number, ok: boolean }
```

#### `mechanics:base64`

```typescript
import { encode, decode } from "mechanics:base64";
encode(buffer: ArrayBuffer | ArrayBufferView, variant?: "base64" | "base64url"): string;
decode(encoded: string, variant?: "base64" | "base64url"): Uint8Array;
```

#### `mechanics:hex`

```typescript
import { encode, decode } from "mechanics:hex";
encode(buffer: ArrayBuffer | ArrayBufferView): string;
decode(encoded: string): Uint8Array;
```

#### `mechanics:base32`

```typescript
import { encode, decode } from "mechanics:base32";
encode(buffer: ArrayBuffer | ArrayBufferView, variant?: "base32" | "base32hex"): string;
decode(encoded: string, variant?: "base32" | "base32hex"): Uint8Array;
```

#### `mechanics:form-urlencoded`

```typescript
import { encode, decode } from "mechanics:form-urlencoded";
encode(record: Record<string, string>): string;  // deterministic key order
decode(params: string): Record<string, string>;  // leading '?' accepted
```

#### `mechanics:rand`

```typescript
import fillRandom from "mechanics:rand";
fillRandom(buffer: ArrayBuffer | ArrayBufferView): void;  // fills in-place
```

#### `mechanics:uuid`

```typescript
import uuid from "mechanics:uuid";
uuid(variant?: "v3"|"v4"|"v5"|"v6"|"v7"|"nil"|"max",
     options?: { namespace: string, name: string }): string;
// v3/v5 require namespace + name; default is "v4"
```

### Sandboxing

Scripts run in an isolated JavaScript realm:

- No filesystem or network access except through
  `mechanics:endpoint`.
- No cross-job state — each step starts with a fresh realm.
- Execution limits enforced by the worker pool:
  - Wall-clock timeout (default: 10 seconds)
  - Loop iteration limit (default: 1,000,000)
  - Recursion depth limit (default: 512)
  - Stack size limit (default: 10 KB)

## Running a Workflow

### 1. Create an instance

```
POST /v1/workflows/instances
Authorization: Bearer pht_...
X-Tenant-Id: <tenant-uuid>
Content-Type: application/json

{
  "template_id": "<template-uuid>",
  "args": { "question": "Hello, world!" }
}
```

### 2. Execute a step

```
POST /v1/workflows/instances/<instance-uuid>/execute
Authorization: Bearer pht_...
X-Tenant-Id: <tenant-uuid>
Content-Type: application/json

{
  "input": { "question": "What is 2+2?" }
}
```

The response includes the script's output, updated context,
and the new instance status.

### 3. Check status

```
GET /v1/workflows/instances/<instance-uuid>
Authorization: Bearer pht_...
X-Tenant-Id: <tenant-uuid>
```

### 4. View step history

```
GET /v1/workflows/instances/<instance-uuid>/steps
Authorization: Bearer pht_...
X-Tenant-Id: <tenant-uuid>
```

## Instance Lifecycle

```
Pending ──→ Running ──→ Completed
               │
               ├──→ Failed
               │
               └──→ Cancelled
```

- **Pending**: Created but no steps executed yet.
- **Running**: At least one step executed; waiting for more.
- **Completed**: Script returned `done: true`.
- **Failed**: Script threw an error or the executor was
  unreachable.
- **Cancelled**: Cancelled via the API.

## Example: Simple Echo Workflow

**Template script:**

```javascript
export default function(arg) {
  return {
    output: {
      echo: arg.input,
      step: (arg.context.step || 0) + 1
    },
    context: {
      step: (arg.context.step || 0) + 1
    },
    done: arg.input.finish === true
  };
}
```

**Abstract config:** `{}`

**Usage:**

1. Create template with the script above.
2. Create instance with `args: {}`.
3. Execute steps with `input: { message: "hello" }`.
4. Each step echoes the input and increments a counter.
5. Send `input: { finish: true }` to complete the workflow.

## Example: LLM Chat Workflow

**Setup:**

1. Create an endpoint config for your LLM service:
   ```json
   {
     "display_name": "Local LLM",
     "implementation": "llm_openai_compat",
     "config": {
       "base_url": "http://localhost:8080/v1",
       "api_key": "local-dev-key",
       "dialect": "openai_native",
       "timeout_ms": 120000
     }
   }
   ```

2. Create template with the endpoint UUID:
   ```json
   {
     "display_name": "LLM Chat",
     "abstract_config": { "llm": "<endpoint-uuid>" },
     "script_source": "..."
   }
   ```

**Script:**

```javascript
import endpoint from "mechanics:endpoint";

export default async function(arg) {
  const messages = arg.context.messages || [];
  messages.push({ role: "user", content: arg.input.message });

  const response = await endpoint("llm", {
    body: {
      model: "default",
      messages,
      output_schema: {
        type: "object",
        properties: { reply: { type: "string" } },
        required: ["reply"],
        additionalProperties: false
      }
    }
  });

  const replyText = response.body.output.reply;
  messages.push({ role: "assistant", content: replyText });

  return {
    output: { reply: replyText },
    context: { messages },
    done: arg.input.finish === true
  };
}
```

**Usage:**

1. Create instance with `args: {}`.
2. Execute steps with `input: { message: "Hello!" }`.
3. Each step sends the conversation history to the LLM.
4. Send `input: { message: "Goodbye", finish: true }` to end.

## Permissions

Workflow operations require these permission atoms in the
caller's role:

| Operation | Permission |
|---|---|
| Create template | `workflow:template_create` |
| Read templates | `workflow:template_read` |
| Retire template | `workflow:template_retire` |
| Create instance | `workflow:instance_create` |
| Read instances | `workflow:instance_read` |
| Execute step | `workflow:instance_execute` |
| Cancel instance | `workflow:instance_cancel` |
| Create endpoint config | `endpoint:create` |
| View endpoint config | `endpoint:read_metadata` |
| View decrypted config | `endpoint:read_decrypted` |
| Rotate endpoint config | `endpoint:rotate` |
| Retire endpoint config | `endpoint:retire` |
