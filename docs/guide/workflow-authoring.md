# Workflow Authoring Guide

This guide explains how to create, deploy, and execute workflows.

## Concepts

A **workflow template** defines reusable automation logic. It
consists of:

- **Script source** — JavaScript code that runs in a sandboxed
  engine (Boa). The script's default export is called for each
  execution step.
- **Abstract config** — JSON describing which external service
  endpoints the script can call (LLM APIs, databases, HTTP
  services, etc.).

A **workflow instance** is a running copy of a template, created
with specific **args** (input parameters). Instances maintain
**context** (mutable state that persists across steps) and
progress through a lifecycle: Pending → Running → Completed (or
Failed/Cancelled).

Each **step** is one execution of the script. Steps receive input
and produce output. The script decides whether the workflow is
done (`done: true`) or needs more steps.

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

### Via the WebUI

Navigate to **Templates → New Template**. Enter a display name,
paste the script source, and provide the abstract config as JSON.

## Writing Scripts

Scripts are ES2024 JavaScript modules. The default export must be
a function that receives a single argument object and returns a
result object.

### Script argument

The function receives:

```javascript
{
  context: { /* mutable state from previous steps */ },
  args:    { /* instance creation arguments */ },
  input:   { /* step-specific input */ },
  subject: { /* caller identity information */ }
}
```

### Return value

The function must return:

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

Scripts can call external HTTP services configured in the abstract
config using the built-in `mechanics:endpoint` module:

```javascript
import { call } from "mechanics:endpoint";

export default async function(arg) {
  const response = await call("my-endpoint", {
    body: JSON.stringify({ prompt: arg.input.question })
  });

  return {
    output: { answer: response.body },
    context: arg.context,
    done: true
  };
}
```

The endpoint name (`"my-endpoint"`) must match a key in the
abstract config.

### Sandboxing

Scripts run in an isolated JavaScript realm:

- No filesystem or network access except through
  `mechanics:endpoint`.
- No cross-job state — each step starts with a fresh realm.
- Execution timeouts and memory limits are enforced by the
  worker pool configuration.

## Abstract Config

The abstract config maps endpoint names to their connection
parameters. The exact shape depends on the connector
implementation:

```json
{
  "my-endpoint": {
    "connector": "llm-openai-compat",
    "url": "https://api.example.com/v1/chat/completions",
    "model": "gpt-4",
    "api_key_env": "OPENAI_API_KEY"
  }
}
```

At execution time, the abstract config is **lowered** by the API
server into concrete, encrypted endpoint configurations that are
sent to the connector service. This separation means the script
never sees raw API keys or connection strings.

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
