# Phase 7 Tier 1 — `embed` + `vector_search` impl spec

**Author**: Claude Code
**Date**: 2026-04-24 (金)
**Audience**: Yuka — answer the open-questions section at the
bottom (grouped by crate); once resolved I'll split this into
two Codex dispatch prompts and send them in parallel.
**Status**: **draft — 9 open questions (4 for `embed`, 5 for
`vector_search`) awaiting resolution**.
**Crates**:
- [`philharmonic-connector-impl-embed`](https://github.com/metastable-void/philharmonic-connector-impl-embed)
- [`philharmonic-connector-impl-vector-search`](https://github.com/metastable-void/philharmonic-connector-impl-vector-search)

## Purpose

Doc 08 §"Embedding and vector search" is 14 lines and only
settles the split decision (two capabilities, two crates).
Doc 14 explicitly flags both as under-specified:

> `embed` and `vector_search` — output vector format;
> nearest-neighbor result shape.

This doc lifts both out of under-specified territory: proposes
concrete wire protocols, names a first implementation per
crate, flags the genuine design questions that need Yuka's
input, and recommends answers for each.

Both crates are Phase 7 Tier 1 per the priority tiers Yuka
captured 2026-04-24. Once the questions below are resolved,
the two crates can dispatch to Codex in parallel (independent
submodules, no file contention) alongside the already-running
SQL pair (`sql-postgres` + `sql-mysql`).

## Shared context

- `embed` and `vector_search` are **separate capabilities**,
  each with its own normalized wire protocol. Scripts
  compose them: produce a vector with `embed`, query nearest
  neighbors with `vector_search`.
- Neither depends on the other technically. A workflow can
  call one without the other. They ship separately at 0.1.0.
- State management: both capabilities assume **tenants manage
  state out-of-band** (doc 08 §"State management for stateful
  external services"). The embed capability doesn't ingest
  anything — it produces vectors. The vector-search
  capability doesn't populate the store — it queries.
  Upsert-style capabilities are v2+ if ever.

---

## `embed` — design notes

### What it does

- **Input**: one or more texts.
- **Output**: one embedding vector per input text.
- **Wire shape goal**: OpenAI `/v1/embeddings`-compatible.
  This covers OpenAI itself + any OpenAI-compatible server
  that implements the same endpoint (vLLM, Together, Groq,
  local inference servers). Same rationale as
  `llm_openai_compat` for `llm_generate`.

### Proposed config shape

```json
{
  "realm": "embed",
  "impl": "embed",
  "config": {
    "base_url": "https://api.openai.com/v1",
    "api_key": "sk-...",
    "model": "text-embedding-3-small",
    "dimensions": 1536,
    "timeout_ms": 30000
  }
}
```

- `base_url` — required. `/v1/embeddings` is appended.
- `api_key` — required. `Authorization: Bearer <key>`.
- `model` — required. Provider model identifier (OpenAI:
  `text-embedding-3-small`, `text-embedding-3-large`, etc.;
  vLLM: the locally-loaded model name).
- `dimensions` — required. Expected output dimension count;
  passed through to OpenAI as the `dimensions` parameter (a
  truncation knob for `text-embedding-3-*` models) and
  validated against actual response length. Deployments
  choose; we don't auto-detect.
- `timeout_ms` — optional, default 30_000 (30s — embeddings
  are much cheaper than generation, so the default is
  tighter than `llm_openai_compat`'s 60s).

### Proposed request shape (normalized)

```json
{
  "texts": ["text one", "text two", "..."]
}
```

- `texts` — required. Array of strings to embed. Minimum 1
  element; maximum is provider-dependent (OpenAI's limit is
  currently 2048 inputs per request, but that cap is
  enforced upstream — scripts that exceed it get an
  `UpstreamError`).

### Proposed response shape (normalized)

```json
{
  "embeddings": [
    [0.012, -0.034, ...],
    [0.041, 0.007, ...]
  ],
  "model": "text-embedding-3-small-2024-...",
  "dimensions": 1536,
  "usage": {
    "input_tokens": 42
  }
}
```

- `embeddings` — array of arrays of f32. `embeddings.length`
  matches `texts.length`.
- `model` — the provider-echoed resolved model name (e.g.
  OpenAI returns the pinned subversion).
- `dimensions` — sanity-check echo of config's
  `dimensions`; also validates against actual vector length.
- `usage.input_tokens` — total input tokens across all
  texts.

### Proposed error cases

- `UpstreamError` — non-2xx from provider (bad key, model
  unknown, etc.). `body` carries the provider's error
  payload verbatim.
- `UpstreamUnreachable` — network / TLS / connection refused.
- `UpstreamTimeout` — per-request timeout.
- `InvalidConfig` — config deserialization / validation.
- `InvalidRequest` — request deserialization, zero-length
  `texts`, or vector-length mismatch between `dimensions`
  and actual provider response (shouldn't happen, but
  protects against provider bugs).
- `Internal` — response envelope malformed.

### Proposed module layout

```
src/
├── lib.rs            # crate rustdoc + public Embed type + trait impl + impl-api re-exports
├── config.rs         # EmbedConfig + prepare() (reqwest::Client builder)
├── request.rs        # EmbedRequest (texts: Vec<String>) with deny_unknown_fields
├── response.rs       # EmbedResponse + Usage
├── client.rs         # reqwest::Client builder + single-attempt POST
├── retry.rs          # hardcoded minimal retry (same pattern as llm_openai_compat)
└── error.rs          # internal Error enum + From<Error> for ImplementationError
```

### Dependencies

- `philharmonic-connector-impl-api = "0.1"`
- `philharmonic-connector-common = "0.2"`
- `async-trait = "0.1"`
- `reqwest = { version = "0.13", default-features = false, features = ["rustls-tls", "json", "gzip", "deflate", "brotli"] }`
- `tokio = { version = "1", features = ["rt", "macros", "time"] }`
- `serde`, `serde_json`, `thiserror`
- dev: `wiremock = "0.6"`, `tokio` with `test-util`

### Testing

- Unit tests per module (deny-unknown-fields, retry math,
  error mapping).
- Integration tests with `wiremock`: happy-path embedding,
  batch embedding (multiple texts), error cases, timeout,
  retry on 429.
- **Optional smoke** against the real OpenAI embeddings API,
  `#[ignore]`-d + env-gated
  (`EMBED_SMOKE_ENABLED=1 OPENAI_API_KEY=...`). Single call
  with a cheap model (`text-embedding-3-small`, ~$0.00002
  per test).

### `embed` — open questions (Qe1–Qe4)

**Qe1. Crate name: `embed` or `embed_openai_compat`?**

- Option A: `Implementation::name()` returns `"embed"`.
  Dialect is a config field (`"openai_compat"` for v1, others
  later). Matches the flexibility of `llm_openai_compat` but
  reserves the bare `embed` name.
- Option B: `Implementation::name()` returns
  `"embed_openai_compat"` to mirror
  `llm_openai_compat`'s naming. Leaves room for future
  `embed_anthropic`, `embed_gemini` as separate crates.
- **My rec: Option A.** The OpenAI `/v1/embeddings` shape is
  the de-facto lingua franca; every major provider
  (including local vLLM) implements it. Reserving `embed`
  for this single well-defined shape is cleaner than
  introducing a dialect split we don't have evidence we
  need.

**Qe2. Input: single text vs. batch (array only)?**

- Option A: `texts: [String]` (batch always; single-input
  callers use a 1-element array). Matches OpenAI's request
  shape directly.
- Option B: `text: String | [String]` (accept either,
  normalize internally). Scripts that only ever embed one
  text don't have to wrap it in an array.
- **My rec: Option A.** Uniform shape is easier to reason
  about; the cost of `["hello"]` vs. `"hello"` on the script
  side is negligible.

**Qe3. Output precision: f32 or f64?**

- Option A: `Vec<f32>`. OpenAI's embeddings are f32-native;
  storing as f32 saves 50% memory and matches what Qdrant
  stores.
- Option B: `Vec<f64>`. JSON numbers are f64 by default;
  storing as f64 avoids a potential precision loss on the
  serde round-trip.
- **My rec: Option A (f32).** f32 is the ecosystem norm for
  embeddings (SentenceTransformers, Qdrant, all OpenAI
  outputs). f64 would be a surprising upgrade with real
  memory cost for no gain.

**Qe4. `dimensions` as config vs. request?**

- Option A (current draft): config-only. Deployment chooses
  the model + dimensions; scripts take them as given.
- Option B: request-override allowed. Scripts can
  down-truncate dimensions per call.
- **My rec: Option A.** Dimension truncation is a deployment
  concern (it affects vector-store schema compatibility);
  scripts shouldn't surprise the deployment. If a use case
  emerges, add a request-level override in 0.2.0.

---

## `vector_search` — design notes

### What it does

- **Input**: query vector + top_k (+ optional filter).
- **Output**: ranked nearest neighbors with IDs and scores
  (+ optional payload metadata).
- **First implementation**: Qdrant. Doc 08 says "Qdrant;
  possibly others" — for v1, Qdrant only.

### Proposed config shape

```json
{
  "realm": "vector_search",
  "impl": "vector_search",
  "config": {
    "backend": "qdrant",
    "url": "https://qdrant.internal:6334",
    "api_key": "...",
    "collection": "my-tenant-knowledge-base",
    "timeout_ms": 10000
  }
}
```

- `backend` — required discriminator. For v1 the only valid
  value is `"qdrant"`; reserved field for future alternative
  backends.
- `url` — required. Qdrant endpoint. Port 6333 (HTTP REST)
  or 6334 (gRPC) — see Qv1 below.
- `api_key` — required. Sent as Qdrant's `api-key` header.
- `collection` — required. One collection per
  `TenantEndpointConfig`; a tenant with multiple
  collections creates multiple endpoint configs. Rationale:
  keeps the connector's operational surface small (one pool
  per collection matches the endpoint-config-per-resource
  pattern we've used elsewhere).
- `timeout_ms` — optional, default 10_000.

### Proposed request shape (normalized)

```json
{
  "vector": [0.012, -0.034, ...],
  "top_k": 10,
  "score_threshold": 0.75,
  "filter": {"must": [{"key": "tenant_id", "match": {"value": "t_abc"}}]},
  "with_payload": true
}
```

- `vector` — required. Query vector; length must match the
  collection's configured dimension.
- `top_k` — required. Number of neighbors to return.
  Deployment may clamp (see Qv3 below).
- `score_threshold` — optional. Drop results below this
  score (provider-agnostic meaning: "similarity score," not
  "distance"; Qdrant returns higher = closer for cosine/dot,
  lower = closer for euclidean — we normalize the meaning;
  see Qv4 below).
- `filter` — optional. Qdrant filter DSL pass-through (see
  Qv5 below).
- `with_payload` — optional boolean, default true. When
  false, `payload` is omitted from results.

### Proposed response shape (normalized)

```json
{
  "results": [
    {
      "id": "pt_42",
      "score": 0.91,
      "payload": {"text": "...", "source": "..."}
    },
    {"id": "pt_99", "score": 0.88, "payload": {...}}
  ]
}
```

- `results` — array of neighbors, ordered by score
  (descending — higher = closer).
- `id` — Qdrant point ID. Qdrant supports both u64 and UUID
  IDs; we serialize both as JSON strings for uniformity (a
  `u64` ID becomes its decimal string).
- `score` — the similarity score, normalized so higher =
  closer regardless of underlying distance metric (see
  Qv4).
- `payload` — present iff `with_payload: true` in the
  request. Arbitrary JSON object; Qdrant passes it through
  verbatim.

### Proposed error cases

- `UpstreamError` — Qdrant returned a non-2xx (collection
  not found, auth failure, etc.).
- `UpstreamUnreachable` — network / TLS failure.
- `UpstreamTimeout` — request exceeded `timeout_ms`.
- `InvalidConfig` — bad config, bad `backend` discriminator,
  unparseable URL.
- `InvalidRequest` — bad vector (wrong length, NaN),
  top_k <= 0, malformed filter.
- `Internal` — response envelope malformed.

### Proposed module layout

```
src/
├── lib.rs            # crate rustdoc + public VectorSearch type + trait impl + impl-api re-exports
├── config.rs         # VectorSearchConfig + Backend enum + prepare()
├── request.rs        # VectorSearchRequest with deny_unknown_fields
├── response.rs       # VectorSearchResponse + Result struct
├── backend/
│   ├── mod.rs        # Backend dispatch (one variant in v1: Qdrant)
│   └── qdrant.rs     # Qdrant-specific translation + execution
├── retry.rs          # hardcoded minimal retry
└── error.rs          # internal Error enum + From<Error> for ImplementationError
```

### Dependencies

- `philharmonic-connector-impl-api = "0.1"`
- `philharmonic-connector-common = "0.2"`
- `async-trait = "0.1"`
- Qdrant client: either `qdrant-client` crate (the official
  Rust client, ~v1.12 at time of drafting) OR direct HTTP
  via `reqwest`. See Qv1.
- `tokio`, `serde`, `serde_json`, `thiserror`
- dev: `testcontainers = "0.27"` (for real Qdrant container
  tests) and/or `wiremock = "0.6"` (for HTTP-shape tests),
  depending on Qv1.

### Testing

- Unit tests per module (deny-unknown-fields, score
  normalization, error mapping).
- Integration tests: depending on Qv1, either
  `testcontainers`-spawned Qdrant or `wiremock`-mocked HTTP.
  Unit tests always run; integration tests `#[ignore]`-d
  when Docker absent.

### `vector_search` — open questions (Qv1–Qv5)

**Qv1. Qdrant client: `qdrant-client` crate or direct
HTTP-via-reqwest?**

- Option A: `qdrant-client = "1.12"`. Official crate; uses
  gRPC (port 6334). Pulls in `tonic`, `prost`, and a sizable
  generated-code tree. Updates with Qdrant server versions.
- Option B: Direct HTTP via `reqwest` against Qdrant's REST
  API (port 6333). Fewer deps; more stable surface; easier
  to mock with `wiremock` in tests. We control the wire
  shape we emit.
- **My rec: Option B (direct HTTP).** Qdrant's REST API is
  stable, well-documented, and tiny for our use case (one
  endpoint: `POST /collections/{collection}/points/search`).
  Avoiding `tonic`'s transitive dep surface keeps the crate
  light, and our testing becomes deterministic via
  `wiremock` (no testcontainers dependency in CI). The
  official crate makes sense for projects using many Qdrant
  features (cluster management, upserts, snapshots) — we
  use one feature.

**Qv2. Collection scope: config or request?**

- Option A (current draft): config-level. One
  `TenantEndpointConfig` per collection; a tenant with N
  collections creates N endpoint configs.
- Option B: request-level. One endpoint config covers all
  collections; scripts name the collection per call.
- **My rec: Option A.** Matches the
  endpoint-config-per-resource pattern we've used for other
  connectors. Cross-collection queries are rare; when they
  exist, scripts chain multiple `vector_search` calls.

**Qv3. `top_k` cap in config?**

- Option A: No cap; scripts can request any top_k, Qdrant
  enforces its own limits.
- Option B: Add `max_top_k` to config (default 100); clamp
  request values silently. Matches SQL's `default_max_rows`
  pattern.
- **My rec: Option B.** Symmetry with SQL; protects against
  accidentally-huge result sets; deployments choose.

**Qv4. Score normalization: pass-through or normalize
higher-is-closer?**

- Option A: Pass-through Qdrant's score as-is. Scripts need
  to know the collection's distance metric to interpret.
- Option B: Normalize so higher = closer regardless of
  metric. For Cosine/Dot (Qdrant's default), higher already
  means closer; for Euclidean/Manhattan (where lower =
  closer), flip the sign or apply `1 / (1 + d)`. Script
  always sees higher = better.
- **My rec: Option B.** The normalization is a connector
  concern; pushing metric-awareness onto the script would
  leak provider semantics.

**Qv5. Filter DSL: Qdrant-native pass-through or normalized
subset?**

- Option A (current draft): pass-through Qdrant's filter
  object verbatim. Scripts write Qdrant syntax;
  documentation points at Qdrant's docs.
- Option B: Define a portable filter subset (key/value
  match, range, must/should/must_not) and translate in the
  adapter. Future backends can share the subset.
- **My rec: Option A for v1.** We have one backend; adding
  a translation layer before we know what other backends
  need is premature abstraction. If/when a second backend
  lands, extract the common subset then. Keep filter
  pass-through behind a comment in `src/backend/qdrant.rs`
  so future maintainers see the coupling.

---

## Open-questions summary (for Yuka's answer)

Answer each; my recommendation is in **bold** where I have
one.

**`embed`**:
- Qe1. Crate name. **A: `embed` (single dialect for v1)**.
- Qe2. Input shape. **A: batch array only**.
- Qe3. Output precision. **A: f32**.
- Qe4. `dimensions` scope. **A: config-only for v1**.

**`vector_search`**:
- Qv1. Qdrant client. **B: direct HTTP (reqwest)**.
- Qv2. Collection scope. **A: config-level**.
- Qv3. `top_k` cap. **B: add `max_top_k` to config**.
- Qv4. Score normalization. **B: normalize higher-is-closer
  in the adapter**.
- Qv5. Filter DSL. **A: Qdrant pass-through for v1**.

---

Next step: once you answer, I'll convert the above into two
Codex dispatch prompts
(`docs/codex-prompts/2026-04-24-0005-phase-7-embed.md` +
`2026-04-24-0006-phase-7-vector-search.md`) and dispatch them
in parallel alongside the already-running SQL pair.
