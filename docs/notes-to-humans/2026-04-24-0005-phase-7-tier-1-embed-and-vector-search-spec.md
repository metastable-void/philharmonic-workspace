# Phase 7 Tier 1 — `embed` + `vector_search` impl spec

**Author**: Claude Code
**Date**: 2026-04-24 (金); redesigned same day after Yuka's
steers on scope (embed = local inference with binary-bundled
model; vector_search = stateless in-memory kNN).
**Audience**: Yuka — the design below reflects those steers
end-to-end. Any residual questions are listed at the bottom;
otherwise this is ready to split into two Codex dispatch
prompts.
**Status**: **approved for Codex prompt archival** pending
Yuka's sign-off on the four residual questions at the bottom
(all narrow; reasonable defaults named for each).
**Crates**:
- [`philharmonic-connector-impl-embed`](https://github.com/metastable-void/philharmonic-connector-impl-embed)
- [`philharmonic-connector-impl-vector-search`](https://github.com/metastable-void/philharmonic-connector-impl-vector-search)

## Purpose

Doc 08 §"Embedding and vector search" only settles the split
decision. Doc 14 explicitly flags both as under-specified.
This doc provides concrete wire protocols + module layouts +
decisions for both, calibrated to the scope Yuka set 2026-04-24:

1. **`embed` runs a small CPU-runnable model loaded in-process
   from binary-bundled bytes.** No HTTP, no provider API key,
   no HF-hub runtime download. Deployments embed the model
   weights into their connector-service binary at build time
   (Philharmonic provides the loader; the binary provides the
   bytes). Constrained-network and air-gapped deployments are
   a first-class target.
2. **`vector_search` is stateless.** Up to ~hundreds-to-
   thousands of items per collection. The *corpus* is passed
   with each request; the connector computes kNN in-memory
   against that corpus and returns ranked neighbors. No
   persistent store, no Qdrant, no network calls out.

Both crates are Phase 7 Tier 1. Once Yuka signs off on the
four residuals below, they dispatch to Codex in parallel
alongside the already-running SQL pair.

## Shared context

- Separate capabilities, separate crates. Neither depends on
  the other at build time.
- Scripts compose them end-to-end (produce a vector with
  `embed`, query against a per-call corpus with
  `vector_search`).
- State stays out-of-band per doc 08 §"State management for
  stateful external services". In particular, `vector_search`
  v1 explicitly does *not* populate a store — the store is
  whatever the script has in hand and passes in.
- No crypto paths. No Gate 1/2.

---

## `embed` — design

### What it does

Load a small embedding model from binary-provided ONNX +
tokenizer bytes at construction time; at call time, embed
one or more texts and return one f32 vector per text. Model
choice is the deployment's — the connector crate is model-
agnostic; the deployment's connector-service binary bundles
whatever ONNX it wants via `include_bytes!` (or equivalent)
and hands the bytes to `Embed::new_from_bytes(...)`.

Target model characteristics: small, multilingual,
CPU-runnable, fits comfortably in RAM (tens to low-hundreds
of MB). fastembed's `paraphrase-multilingual-MiniLM-L12-v2`
(≈100MB, 384-dim) is the canonical example; the E5
multilingual variants and `bge-m3` are alternatives.

### Config shape

```json
{
  "realm": "embed",
  "impl": "embed",
  "config": {
    "model_id": "paraphrase-multilingual-MiniLM-L12-v2",
    "max_batch_size": 32,
    "timeout_ms": 10000
  }
}
```

- `model_id` — required string. Must match the `model_id`
  the deployment's connector-service binary registered when
  it constructed the `Embed` instance. If it doesn't match,
  `execute` returns `InvalidConfig`. Enables operator
  sanity-check ("the endpoint config says X model; is that
  what this binary is running?") and leaves room for
  multi-model connector binaries later (v1 ships single-
  model per binary).
- `max_batch_size` — optional. Cap on `texts.length` per
  call; requests above the cap fail with `InvalidRequest`.
  Default: `32`. Prevents runaway batch sizes from
  exhausting CPU or memory.
- `timeout_ms` — optional. Wraps the full embed pipeline
  (tokenize + infer + serialize) via
  `tokio::time::timeout`. Default: `10_000` (10s). CPU
  inference on 32 short texts typically completes in
  milliseconds; the timeout is a safety cap for pathological
  inputs, not an operational parameter.

### Request shape

```json
{
  "texts": ["text one", "text two", "..."]
}
```

- `texts` — required array of strings. Length 1 ≤ N ≤
  `max_batch_size` (from config).

### Response shape

```json
{
  "embeddings": [
    [0.012, -0.034, ...],
    [0.041, 0.007, ...]
  ],
  "model": "paraphrase-multilingual-MiniLM-L12-v2",
  "dimensions": 384
}
```

- `embeddings` — array of arrays of f32. Length matches
  `texts`; inner length matches `dimensions`.
- `model` — echo of the `model_id` config value; lets
  scripts audit-log which model produced which vector.
- `dimensions` — integer matching the inner vector length.
  Taken from the model's known dimension at construction
  time.

No `usage` field. Local inference has no token-billing
metric the connector cares about.

### Error cases

- `InvalidConfig` — malformed config, `model_id` doesn't
  match the loaded model, unparseable field.
- `InvalidRequest` — malformed request, `texts.length` zero
  or above `max_batch_size`, non-string elements.
- `UpstreamTimeout` — inference exceeded `timeout_ms`.
- `Internal` — inference pipeline failed (ONNX runtime
  error, tokenizer error). Not a script bug; genuinely
  internal.

No `UpstreamError` (no HTTP upstream). No `UpstreamUnreachable`
(no network). No retry logic — CPU inference failures are
deterministic; retrying doesn't help.

### Module layout

```
src/
├── lib.rs           # crate rustdoc + public Embed type + Embed::new_from_bytes constructor + trait impl
├── config.rs        # EmbedConfig + deny_unknown_fields
├── request.rs       # EmbedRequest with deny_unknown_fields
├── response.rs      # EmbedResponse (embeddings, model, dimensions)
├── model.rs         # thin fastembed wrapper: holds the TextEmbedding instance + model metadata
└── error.rs         # internal Error + From<Error> for ImplementationError
```

No `retry.rs`, no `client.rs` — local inference has no
retry/client concern. No `dialect/` — single pipeline.

### Public surface

```rust
pub struct Embed { /* private */ }

impl Embed {
    /// Constructs an Embed from the caller-provided ONNX model
    /// bytes + tokenizer files. Eager-loads the model into memory
    /// and initializes the tokenizer; fails fast on malformed
    /// inputs so the connector-service binary errors at startup
    /// rather than mid-workflow.
    ///
    /// `model_id` is the identifier the config's `model_id`
    /// field must match; it is not interpreted against any
    /// registry — the deployment binary names whatever it wants.
    pub fn new_from_bytes(
        model_id: impl Into<String>,
        onnx_bytes: Vec<u8>,
        tokenizer_files: TokenizerFiles,
        dimensions: usize,
        max_seq_length: usize,
    ) -> Result<Self, ImplementationError>;

    pub fn model_id(&self) -> &str;
    pub fn dimensions(&self) -> usize;
}

#[async_trait]
impl Implementation for Embed { ... }
```

`TokenizerFiles` is fastembed's existing struct
(`tokenizer.json`, `config.json`, `special_tokens_map.json`,
`tokenizer_config.json` — four bytes fields). Re-exported
from `src/lib.rs` so binaries can construct it with
`include_bytes!` of each file.

### Dependencies

- `philharmonic-connector-impl-api = "0.1"`
- `philharmonic-connector-common = "0.2"`
- `async-trait = "0.1"`
- `fastembed = "4"` (verify latest via `crates-io-versions`
  at implementation time). fastembed wraps `ort` (ONNX
  Runtime) internally; we get CPU inference without touching
  `ort` directly.
- `tokio = { version = "1", features = ["rt", "macros",
  "time"] }` — for `spawn_blocking` around inference and the
  timeout.
- `serde`, `serde_json`, `thiserror`.
- **No `reqwest`.** **No HF-hub fetch at runtime.**

fastembed's `TextEmbedding::try_new_from_user_defined(...)`
path takes pre-loaded bytes (vs. the default
`TextEmbedding::try_new(...)` which downloads from HF). We
use the user-defined path exclusively.

### Multilingual verification (resolve at implementation time)

Yuka's steer requires multilingual support. The multilingual
models in fastembed's public catalog (check
`fastembed::EmbeddingModel` at the exact dep version pinned):

- `paraphrase-multilingual-MiniLM-L12-v2` — 100MB, 384-dim,
  ~50 languages.
- `paraphrase-multilingual-mpnet-base-v2` — 400MB, 768-dim,
  higher quality, slower.
- `multilingual-e5-*` variants — if present in the catalog.
- `bge-m3` — if present; supports multilingual.

The connector crate doesn't pin a specific model — the
binary does. But Codex should verify during implementation
that fastembed's `TextEmbedding::try_new_from_user_defined`
works against at least one of the above multilingual
models; if fastembed has regressed on multilingual, flag
and stop (we'd reconsider the library choice).

### Testing

- Unit tests per module: deny-unknown-fields, config
  validation, request validation, response serialization,
  error mapping. Pure-logic, no model needed.
- Integration tests under `tests/`: run against an actual
  loaded model. **`#[ignore]`-gated by default**, opt-in
  via env vars:
  - `EMBED_TEST_ONNX_PATH=/path/to/model.onnx`
  - `EMBED_TEST_TOKENIZER_DIR=/path/to/tokenizer-files/`
  - `EMBED_TEST_MODEL_ID=<identifier>`
  - `EMBED_TEST_DIMENSIONS=<int>`
  - `EMBED_TEST_MAX_SEQ_LENGTH=<int>`
  When all set, tests run against the provided model; test
  assertions check shape (batch count, dimension count,
  model-echo) + basic properties (embedding of "hello" is
  deterministic, embedding-cosine-similarity of near-
  synonyms is higher than unrelated texts).
- **No ONNX fixtures committed to the repo.** The tree
  stays binary-free; developers who want to exercise the
  live-inference tests locally drop a model into a scratch
  directory and set the env vars.
- CI only runs the unit tests (no `EMBED_TEST_*` env vars
  in CI). That's sufficient for pre-landing green; the
  live-inference tests are manual.

---

## `vector_search` — design

### What it does

Given a query vector and a **per-request corpus** of up to a
few thousand labeled vectors, compute cosine similarity of
the query against each corpus item and return the top-k
highest-scoring items with their IDs, scores, and optional
payloads. The corpus is stateless: every request carries the
full set to search against.

Target use case: session-scoped or instance-scoped retrieval
over small knowledge bases. Classic RAG with a few thousand
chunks stays within scope; anything millions-scale needs a
different connector (persistent vector store, not v1).

### Config shape

```json
{
  "realm": "vector_search",
  "impl": "vector_search",
  "config": {
    "max_corpus_size": 5000,
    "timeout_ms": 2000
  }
}
```

- `max_corpus_size` — required. Upper bound on
  `corpus.length` per request. Corpora above this →
  `InvalidRequest`. Deployment sets — no framework default,
  per the same discipline as http_forward's
  `response_max_bytes` (forcing the deployment to choose
  rather than hiding it behind a default).
- `timeout_ms` — optional. Wraps the scoring + sort +
  serialize pipeline. Default: `2_000` (2s). At the target
  scale (hundreds to low-thousands of vectors × small
  dims), scoring completes in milliseconds — the timeout
  is a safety cap.

Metric is hardcoded to cosine per Yuka's call. No
`metric` config field in v1.

### Request shape

```json
{
  "query_vector": [0.012, -0.034, ...],
  "corpus": [
    {"id": "a", "vector": [0.1, 0.2, ...], "payload": {"text": "..."}},
    {"id": "b", "vector": [0.3, 0.4, ...]},
    {"id": "c", "vector": [-0.1, 0.5, ...], "payload": {"text": "...", "source": "..."}}
  ],
  "top_k": 5,
  "score_threshold": 0.75
}
```

- `query_vector` — required. Array of f32 (or JSON numbers
  coerced to f32).
- `corpus` — required. Array of items, each with:
  - `id` — required string. Uniqueness not enforced by the
    connector; duplicates produce duplicate result entries
    if both score above threshold (script's responsibility).
  - `vector` — required. Array of f32, same length as
    `query_vector`. Length mismatch → `InvalidRequest` with
    offset.
  - `payload` — optional arbitrary JSON object; echoed
    verbatim in results when the item scores in.
- `top_k` — required. Integer ≥ 1.
- `score_threshold` — optional. f32 in [-1.0, 1.0]. Drop
  items whose cosine similarity is below this. Applied
  after top-k: if fewer than `top_k` items pass the
  threshold, return fewer (not an error).

### Response shape

```json
{
  "results": [
    {"id": "a", "score": 0.91, "payload": {"text": "..."}},
    {"id": "c", "score": 0.82, "payload": {"text": "...", "source": "..."}}
  ]
}
```

- `results` — array sorted by `score` descending.
  `results.length ≤ top_k`.
- `id`, `score`, `payload` — echoed from the matching
  corpus item. `payload` only present when it was present
  in the input item (not synthesized as `null`).

### Error cases

- `InvalidConfig` — malformed config; missing
  `max_corpus_size`.
- `InvalidRequest`:
  - `corpus.length > max_corpus_size`.
  - `corpus.length == 0`.
  - Vector-length mismatch (query vs. any corpus item, or
    corpus items among themselves — check first item, all
    others must match).
  - `top_k <= 0`.
  - `score_threshold` out of `[-1.0, 1.0]`.
  - Non-finite values (NaN, ±Inf) in any vector.
- `UpstreamTimeout` — scoring exceeded `timeout_ms`.
- `Internal` — serde round-trip failure.

No `UpstreamError`, no `UpstreamUnreachable`, no retry —
no external service.

### Cosine similarity specifics

Standard definition:

```
cos(q, v) = dot(q, v) / (||q|| * ||v||)
```

For each corpus item, compute the dot product and the
corpus item's norm. The query's norm is computed once and
reused. Safe-divide: if either norm is zero, score is 0.0
(not NaN).

Result is in `[-1.0, 1.0]`. Higher = closer. No
normalization or sign-flipping needed (matches Yuka's
earlier "higher = closer" preference naturally for cosine).

### Module layout

```
src/
├── lib.rs           # crate rustdoc + public VectorSearch type + trait impl
├── config.rs        # VectorSearchConfig + deny_unknown_fields
├── request.rs       # VectorSearchRequest + CorpusItem + deny_unknown_fields
├── response.rs      # VectorSearchResponse + ResultItem
├── search.rs        # cosine-score + top-k-select (pure math, heavily unit-tested)
└── error.rs         # internal Error + From<Error> for ImplementationError
```

No `retry.rs`, no `client.rs`, no `backend/` — single
code path, single algorithm, no external service.

### Public surface

```rust
pub struct VectorSearch { /* private, may be ZST */ }

impl VectorSearch {
    pub fn new() -> Self;
}

#[async_trait]
impl Implementation for VectorSearch { ... }
```

Trivially constructible. No state.

### Dependencies

- `philharmonic-connector-impl-api = "0.1"`
- `philharmonic-connector-common = "0.2"`
- `async-trait = "0.1"`
- `tokio = { version = "1", features = ["rt", "macros",
  "time"] }` — for `spawn_blocking` around scoring and the
  timeout.
- `serde`, `serde_json`, `thiserror`.

No `reqwest`, no `qdrant-client`, no `ndarray`, no
`nalgebra`. Pure `[f32]` math. At target scale (≤ few
thousand × ≤ thousand dims), plain scalar loops finish in
microseconds to low milliseconds; SIMD micro-optimization
doesn't earn its complexity cost.

### Testing

- Unit tests per module — heavy on `search.rs` since it
  holds the actual algorithm. Vector pairs with known
  cosine values, top-k selection with ties, score-threshold
  cutoffs, empty corpus rejection, vector-length-mismatch
  rejection.
- Integration tests under `tests/`: end-to-end
  `execute(...)` calls with JSON fixtures and expected
  output shapes. No external services; no `testcontainers`,
  no `wiremock`. Deterministic.
- Fixtures: small JSON corpora committed under
  `tests/fixtures/` (~5-20 items each). Trivial to reason
  about.

### Non-goals (v1)

- Persistent vector stores. When scale or persistence is
  needed, we add a new crate (e.g.,
  `philharmonic-connector-impl-vector-qdrant`).
- Other distance metrics (dot, euclidean). 0.2.0 if asked.
- Corpus deduplication, index construction, ANN approximate
  algorithms. Linear scan is good enough at the target
  scale.
- Tenant-managed state caching across calls.

---

## Decisions (resolved 2026-04-24)

Locked in by Yuka during the spec round:

1. **`embed` library**: `fastembed` v4, pending
   implementation-time verification that multilingual models
   work via `TextEmbedding::try_new_from_user_defined(...)`.
   If fastembed has regressed on multilingual, flag and stop.
2. **`embed` model packaging**: deployment bundles ONNX +
   tokenizer bytes in its connector-service binary via
   `include_bytes!` and passes them to
   `Embed::new_from_bytes(...)`. **No HF-hub runtime
   download.** Constrained-network deployments are a
   first-class target.
3. **`embed` model loading**: eager at construction. Binary
   fails fast at startup on misconfigured model rather than
   erroring mid-workflow.
4. **`embed` dimensions**: config field carries `model_id`
   only; `dimensions` is derived from the loaded model's
   metadata (passed in at `new_from_bytes` construction) and
   echoed in the response. No per-call dimension knob.
5. **`vector_search` distance metric**: cosine only for v1.
   No `metric` config field. Add in 0.2.0 if needed.
6. **`vector_search` corpus cap**: `max_corpus_size` is
   config-required (no framework default), following the
   `response_max_bytes` precedent — forces deployments to
   choose an explicit cap.
7. **`vector_search` backend**: **no backend.** Stateless
   in-memory search over per-request corpus. Target scale
   hundreds-to-low-thousands. Persistent-store backends are
   a separate crate if/when someone needs them (out of v1
   scope).
8. **`vector_search` score semantics**: cosine returns
   `[-1, 1]` natively with higher = closer. No
   normalization layer needed.

## Residual open questions (narrow; my default in bold)

**Qe5. Multi-model support in a single connector binary?**

- Option A: `Embed` holds one model; binaries with multiple
  models construct multiple `Embed` instances (one per
  model_id) and register them separately in the
  `Implementation` registry.
- Option B: `Embed` holds a map of `model_id -> (model,
  tokenizer)`; dispatch internally by config's `model_id`.
  One `Implementation` registration per binary.
- **My default: A (one model per `Embed`).** Simpler surface;
  multi-model binaries construct multiple instances. Upgrade
  to B in 0.2.0 if the registration-multiplicity becomes
  awkward in practice.

**Qv6. Corpus vector length consistency check: whole-corpus
or first-item-only?**

- Option A: Check all corpus items have the same length as
  the query (O(N) pre-scan before scoring).
- Option B: Check the first corpus item matches the query;
  subsequent mismatches error mid-scoring with the offset.
- **My default: A (whole-corpus pre-scan).** At target scale
  (thousands × few hundred dims), the pre-scan cost is
  negligible and the error is clearer: "corpus item 42 has
  length 385, expected 384" rather than "scoring failed at
  item 42."

**Qe6. Crate-level test model fixture: env-var gated (as
above), or commit a tiny ONNX fixture (say, a trimmed
3-layer MiniLM)?**

- Option A: Env-var gated. `#[ignore]`-d by default.
- Option B: Commit a tiny ONNX + tokenizer under
  `tests/fixtures/`. Tests always run locally; adds ~10-50MB
  to the submodule repo.
- **My default: A (env-var gated).** Matches the "no
  redistributed ONNX in the repo" principle; committed model
  files are a license-compliance risk (multilingual models
  have varied licenses) that we should only cross if the
  testing payoff justifies it. CI runs unit tests; live-
  inference verification is a developer's local task.

**Qv7. Corpus IDs: strings only, or accept JSON numbers and
coerce?**

- Option A: Strings only. Scripts pass `"42"` for numeric
  IDs. `InvalidRequest` on non-string ID.
- Option B: Accept `string | number` and coerce numbers
  to strings at ingest.
- **My default: A (strings only).** Uniform representation;
  scripts passing numeric IDs trivially wrap them. B adds
  coercion surface without real value.

---

Next step: once Yuka signs off on the four residuals above
(or tweaks), I'll split this into two Codex dispatch prompts
(`docs/codex-prompts/2026-04-24-0005-phase-7-embed.md` and
`2026-04-24-0006-phase-7-vector-search.md`) and dispatch both
in parallel, alongside the still-running SQL pair if they
haven't returned by then.
