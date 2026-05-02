# Upstream OpenAI chat-completion fixtures

Captured-from-real-API fixtures for the `openai_native` and
`tool_call_fallback` dialects of the upcoming
`philharmonic-connector-impl-llm-openai-compat` crate (Phase 6
Task 2) — see
[`ROADMAP.md`](../../../docs/ROADMAP.md#phase-6--first-implementations)
§"Phase 6 → Task 2 → Testing discipline".

## Why real-API capture?

Unlike vLLM (whose upstream test suite commits the on-the-wire
request-body shape we can lift verbatim), OpenAI publishes no
equivalent tamper-evident fixtures — the
[openai-python](https://github.com/openai/openai-python) repo
has mock-response bodies under
`tests/api_resources/chat/test_completions.py` but the
requests are constructed in-test from Pydantic models, not
committed as wire bytes. The API reference is in prose.

So we capture a minimal real-API pair per dialect and commit
the bytes. Re-capturing against the same model + prompt yields
byte-comparable output modulo the well-known non-determinism
fields (`id`, `created`, `system_fingerprint`, and the
generated `content` / `tool_calls.arguments`). The wire shape
(field names, nesting, value types, header semantics) is what
the fixture anchors — and that shape is what our adapter must
produce byte-for-byte on the request side and must parse
correctly on the response side.

## Pinned provenance

**Capture command**: see the two commands under §"Re-capture"
below — use the committed
[`sample_json_schema.json`](./sample_json_schema.json) as
`--output-schema`, same prompt, same model.

**Capture date**: 2026-04-24 (金) JST — 2026-04-24 09:11 UTC

**Model**: `gpt-4o-mini` (request) / `gpt-4o-mini-2024-07-18`
(OpenAI's reported pinned version in the response). Chosen for
cost and stability; its structured-output behavior is
representative of any OpenAI chat-completion model.

**API endpoint**: `https://api.openai.com/v1/chat/completions`
(not `/v1/responses` — see decision trail in
[`2026-04-24-0003-phase-6-task-2-llm-openai-compat-impl-spec.md`](../../notes-to-humans/2026-04-24-0003-phase-6-task-2-llm-openai-compat-impl-spec.md)).

**Schema discipline**: `strict: true` everywhere.
[`sample_json_schema.json`](./sample_json_schema.json) is a
deliberately OpenAI-strict-compatible variant of the vLLM
employee-profile fixture — same field set, same structure,
but `grade` uses `enum` instead of `pattern` and the bounds
constraints (`minimum`, `maximum`, `minItems`, `maxItems`,
`minProperties`, `maxProperties`) are dropped. OpenAI's
`strict: true` rejects those features; the project's
discipline is to go strict uniformly, so the fixture schema
reflects that.

## Files

### `sample_json_schema.json`

OpenAI-strict-compatible employee-profile JSON Schema.
Semantic parity with the
[vLLM variant](../vllm/sample_json_schema.json) — same keys,
same required set, same `additionalProperties: false`
discipline — diverging only on the strict-mode-incompatible
features. Exercises `type: object`, nested arrays of objects
with their own schemas, nested `additionalProperties: false`,
nested `required`, `enum` for finite string sets.

### `openai_native/request.json`

Outbound HTTP body when the request goes through the
`openai_native` dialect: the schema lives inside
`response_format: {"type": "json_schema", "json_schema":
{"name": "output", "strict": true, "schema": <schema>}}` at
the top level of the chat-completions body.

### `openai_native/response.json`

Full response envelope. Key fields the adapter reads:
`choices[0].finish_reason == "stop"` (maps to normalized
`EndTurn`), `choices[0].message.content` is a JSON string
that parses into the output matching
`sample_json_schema.json`, `usage.prompt_tokens` +
`usage.completion_tokens` feed the normalized
`Usage {input_tokens, output_tokens}`.

### `tool_call_fallback/request.json`

Outbound HTTP body when the request goes through the
`tool_call_fallback` dialect: a synthetic
`tools: [{"type": "function", "function": {"name":
"emit_output", "strict": true, "parameters": <schema>}}]`
plus `tool_choice: {"type": "function", "function": {"name":
"emit_output"}}` at the top level. `strict: true` on the
function definition enforces token-level schema compliance
in tool-calling mode, matching the `response_format` path's
discipline.

### `tool_call_fallback/response.json`

Full response envelope. Key fields the adapter reads:
`choices[0].finish_reason == "stop"` (observed empirically —
**not** `"tool_calls"` as the OpenAI docs sometimes suggest;
when `tool_choice` forces a specific function and the model
complies, the finish_reason reported is `"stop"`, maps to
normalized `EndTurn`), `choices[0].message.content == null`,
`choices[0].message.tool_calls[0].function.arguments` is a
JSON string that parses into the output matching
`sample_json_schema.json`.

## Non-determinism carved out

OpenAI's response envelopes embed a few varying fields that
tests must not bit-assert against:

- `id` (`chatcmpl-...`): per-call request identifier.
- `created`: Unix epoch seconds at response time.
- `system_fingerprint`: OpenAI's internal
  infra-configuration hash; changes as they roll out
  updates.
- `model`: echoes the *resolved* pinned model variant
  (e.g. `gpt-4o-mini-2024-07-18` for a `gpt-4o-mini`
  request); moves when OpenAI aliases a new pin.
- `usage.prompt_tokens` / `usage.completion_tokens`:
  tokenizer drift can shift these by a few tokens over
  time.
- `choices[0].message.content` (openai_native) /
  `choices[0].message.tool_calls[0].function.arguments`
  (tool_call_fallback): the actual generated JSON output,
  which is schema-compliant but otherwise freely varying.
- `choices[0].message.tool_calls[0].id` (`call_...`): per-
  call tool-invocation identifier.

Adapter tests assert the *shape* (presence + types) of
these, never the values. Byte-exact assertion is scoped to
the **request** body only.

## Re-capture

```sh
# openai_native
./scripts/xtask.sh openai-chat -- \
    --model gpt-4o-mini \
    --system-prompt "Return an employee profile as JSON." \
    --prompt "Give an example employee profile." \
    --output-schema tests/fixtures/upstream/openai-chat/sample_json_schema.json \
    --capture-dir tests/fixtures/upstream/openai-chat/openai_native/

# tool_call_fallback
./scripts/xtask.sh openai-chat -- \
    --model gpt-4o-mini \
    --system-prompt "Return an employee profile as JSON." \
    --prompt "Give an example employee profile." \
    --output-schema tests/fixtures/upstream/openai-chat/sample_json_schema.json \
    --tool-call-fallback \
    --capture-dir tests/fixtures/upstream/openai-chat/tool_call_fallback/
```

Both commands require `OPENAI_API_KEY` in `./.env` (or the
environment). Re-running against the same inputs is a drift
probe: non-empty diff of the **request** body against
previously-committed bytes means either our xtask changed or
OpenAI changed its accepted request shape.

## Licensing

OpenAI's chat-completion API responses are not
copyrightable (data, not creative expression; generated by a
model against our inputs). The envelope keys / structure are
the API contract, documented by OpenAI. Committed here as a
fixture for interoperability testing.

The `sample_json_schema.json` fixture is authored by the
Philharmonic project (inspired by the vLLM variant but
independently written under MPL-2.0 / Apache-2.0 like the
rest of the workspace).
