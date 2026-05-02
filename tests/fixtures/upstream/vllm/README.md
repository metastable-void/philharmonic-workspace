# Upstream vLLM test fixtures

Pure-JSON extractions from vLLM's own test suite. These fixtures
are the authoritative source for the `vllm_native` dialect's
wire-shape tests in the upcoming
`philharmonic-connector-impl-llm-openai-compat` crate (Phase 6
Task 2) — see
[`ROADMAP.md`](../../../docs/ROADMAP.md#phase-6--first-implementations)
§"Phase 6 → Task 2 → Testing discipline".

## Why vLLM upstream?

vLLM's own test suite exercises the `structured_outputs={"json":
<schema>}` path end-to-end against a running vLLM server. If
their tests say the wire shape is X, our Rust adapter matches
upstream-by-construction when it produces byte-equivalent
request bodies against their inputs. Zero network access
required on any contributor's side; verification is "diff the
bytes our adapter emits against these JSON files", nothing more.

We do **not** commit any Python from vLLM — these JSON
extractions are the data vLLM's Python fixtures produce. One
data source per file; each traceable to a single upstream
Python literal at a pinned commit.

## Pinned source

Upstream repository: https://github.com/vllm-project/vllm
(Apache-2.0).

**Commit**: `cf8a613a87264183058801309868722f9013e101`
(authored 2026-04-24; fetched 2026-04-24 JST)

File blob SHAs at that commit (for tamper-evident re-
verification):

- `tests/conftest.py` → `9ec31d83c757fb1b41d683338c468fc575453e50`
- `tests/entrypoints/openai/chat_completion/test_chat.py` →
  `212839f78d5ca270e0a037aad83c576fb5341842`

## Files

### `sample_json_schema.json`

The employee-profile JSON Schema. Literal Python → JSON
transcription of
[`tests/conftest.py:sample_json_schema`](https://github.com/vllm-project/vllm/blob/cf8a613a87264183058801309868722f9013e101/tests/conftest.py#L82-L128)
with only the mechanical Python→JSON changes (`True`/`False` →
`true`/`false`, trailing-comma removal, quote style
harmonization, `None` → `null` — none of which appear in this
particular fixture but noted for the general rule).

Exercises a full cross-section of JSON-Schema features:
`type: object`, required fields, nested arrays of objects with
their own schemas, string patterns, number min/max, integer
fields, `additionalProperties: false`, `minItems` / `maxItems`,
`minProperties` / `maxProperties`. A Rust schema-validation
adapter that handles this fixture end-to-end has implicitly
handled most of what any real LLM structured-output caller
throws at it.

### `structured_outputs_json_chat_request.json`

The on-the-wire request body that the openai Python client
produces for
[`test_structured_outputs_json_chat`](https://github.com/vllm-project/vllm/blob/cf8a613a87264183058801309868722f9013e101/tests/entrypoints/openai/chat_completion/test_chat.py#L488-L510)
— after its `extra_body=dict(structured_outputs={"json": ...})`
merge. Demonstrates two claims simultaneously:

1. `structured_outputs` lives at the **top level** of the HTTP
   body, not nested under `extra_body` (which is a Python
   client detail that disappears at the HTTP boundary). This
   confirms doc 08's "not `extra_body`" phrasing against
   upstream behaviour.
2. The inner shape is `{"json": <schema>}` — a single-key
   dispatch object, not a direct schema. The sibling forms
   `{"choice": [...]}` / `{"regex": "..."}` appear elsewhere
   in the same test file and follow the same pattern; lift
   those if they become relevant.

The user `content` field was lightly redacted (inline schema
replaced with a placeholder string) to keep the fixture file
readable; the schema itself lives in its own file, and our
adapter tests reconstruct the full prompt by string-
interpolation at test time.

## Extraction vs. copy

The distinction matters under Apache-2.0:

- We **extract data values** that were Python dicts / lists /
  literals in the upstream source. The data is not creative
  expression; the transcription from Python-literal to JSON is
  mechanical.
- We **do not copy code**. The test function itself, the
  imports, the pytest wiring, `MODEL_NAME = "..."`, etc. stay
  upstream.
- We attribute on the README and in every commit message that
  lands an extracted fixture. Apache-2.0 is permissive in both
  directions; attribution and license-file carry-over are the
  standard protections.
- If upstream drifts (the schema changes, the wire shape
  changes), the pinned SHAs above are the anchor for
  re-extraction. Re-verification is
  `diff $(curl raw-upstream) $(our-fixture)` against the
  literal block in upstream conftest/test_chat.

## Adding new fixtures

When the Task 2 spec identifies another vLLM-upstream data
point that needs extraction (likely candidates: the
`{"choice": [...]}` and `{"regex": "..."}` sibling forms of
`structured_outputs`, or any guided-decoding fixtures from
`tests/v1/entrypoints/` for the v1 engine):

1. Bump the pinned commit SHA above if fetching from a newer
   commit (update all three blob SHAs in lockstep).
2. Extract one file per upstream data point; keep the
   file name aligned with the upstream fixture name
   (`<fixture-name>.json`).
3. Record in this README under `## Files` the exact upstream
   location (file + line range) and what the fixture
   demonstrates.
4. Commit with a message that names the upstream SHA + file
   paths. No submodules — these are static data files in our
   tree.
