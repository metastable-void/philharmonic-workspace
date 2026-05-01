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

```
POST /v1/endpoints
Authorization: Bearer pht_...
X-Tenant-Id: <tenant-uuid>
Content-Type: application/json

{
  "display_name": "My LLM Service",
  "config": {
    "method": "POST",
    "url_template": "https://api.example.com/v1/chat/completions",
    "headers": {
      "Authorization": "Bearer sk-...",
      "Content-Type": "application/json"
    },
    "response_body_type": "json"
  }
}
```

Response: `{ "endpoint_id": "<uuid>" }`

The config value is an `HttpEndpoint` definition with these
fields:

| Field | Type | Description |
|---|---|---|
| `method` | `"GET"` \| `"POST"` \| `"PUT"` \| `"PATCH"` \| `"DELETE"` | HTTP method |
| `url_template` | string | URL with optional `{param}` placeholders |
| `url_param_specs` | `{ name: { required: bool } }` | URL parameter validation |
| `query_specs` | array | Query string parameters |
| `headers` | `{ name: value }` | Fixed request headers |
| `overridable_request_headers` | `string[]` | Headers the script can override at call time |
| `exposed_response_headers` | `string[]` | Response headers visible to the script |
| `request_body_type` | `"json"` \| `"utf8"` \| `"bytes"` | Request body encoding |
| `response_body_type` | `"json"` \| `"utf8"` \| `"bytes"` | Response body decoding |
| `timeout_ms` | number | Request timeout in milliseconds |
| `response_max_bytes` | number | Maximum response body size |
| `allow_non_2xx_status` | boolean | If false, non-2xx throws an error |
| `retry_policy` | object | Automatic retry configuration |

Endpoint configs are encrypted at rest using the SCK (substrate
confidentiality key). The API key and other secrets in the config
are never exposed to the script or stored in plaintext.

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
    body: { model: "gpt-4", messages: [{ role: "user", content: arg.input.question }] }
  });

  return {
    output: { answer: response.body },
    context: arg.context,
    done: true
  };
}
```

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

Scripts have access to these built-in modules:

| Module | Exports | Description |
|---|---|---|
| `mechanics:endpoint` | `default(name, options?)` | Call a configured HTTP endpoint. |
| `mechanics:base64` | `encode(bytes)`, `decode(string)` | Base64 encoding/decoding. |
| `mechanics:hex` | `encode(bytes)`, `decode(string)` | Hexadecimal encoding/decoding. |
| `mechanics:base32` | `encode(bytes)`, `decode(string)` | Base32 encoding/decoding. |
| `mechanics:form-urlencoded` | `encode(obj)`, `decode(string)` | URL form encoding/decoding. |
| `mechanics:rand` | `default(n)` | Generate `n` random bytes as `Uint8Array`. |
| `mechanics:uuid` | `default(options?)` | Generate a UUID string. |

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
     "config": {
       "method": "POST",
       "url_template": "http://localhost:8080/v1/chat/completions",
       "headers": { "Content-Type": "application/json" },
       "response_body_type": "json"
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
    body: { model: "default", messages }
  });

  const reply = response.body.choices[0].message;
  messages.push(reply);

  return {
    output: { reply: reply.content },
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
