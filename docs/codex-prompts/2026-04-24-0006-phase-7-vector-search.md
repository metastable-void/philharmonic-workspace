# Phase 7 Tier 1 — `vector_search` implementation (initial dispatch)

**Date:** 2026-04-24
**Slug:** `phase-7-vector-search`
**Round:** 01 (initial dispatch)
**Subagent:** `codex:codex-rescue`

## Motivation

Phase 7 Tier 1 connector implementation: `vector_search`,
the stateless in-memory cosine kNN connector. No external
vector store, no network calls, no persistent state. Each
request carries its own corpus of up to a few thousand
vectors; the connector computes cosine similarity between
the query vector and every corpus item in-memory, sorts,
applies top-k and optional score threshold, returns the
ranked neighbors. Pure math. Dispatched in parallel with
its sibling `2026-04-24-0005-phase-7-embed.md`; the two
crates are independent and will land in separate submodule
commits.

Non-crypto task: no Gate 1/2, no key material.

## References (read before coding)

- **Authoritative impl spec**:
  [`docs/notes-to-humans/2026-04-24-0005-phase-7-tier-1-embed-and-vector-search-spec.md`](../notes-to-humans/2026-04-24-0005-phase-7-tier-1-embed-and-vector-search-spec.md)
  §"`vector_search` — design" and §"Decisions" + §"Residual
  decisions". If this prompt contradicts the spec, the spec
  wins.
- [`docs/design/08-connector-architecture.md`](../design/08-connector-architecture.md)
  §"Embedding and vector search" — the split decision; no
  wire-protocol detail (the impl spec above fills that in).
- [`ROADMAP.md`](../../ROADMAP.md) §"Phase 7" — priority
  tiers; `embed` + `vector_search` are Tier 1 alongside SQL.
- [`CONTRIBUTING.md`](../../CONTRIBUTING.md):
  - §10.3 no panics in library `src/`.
  - §10.4 libraries take bytes, not file paths (everything
    is in-memory here).
  - §4 git workflow, §5 script wrappers, §11 pre-landing.
- `philharmonic-connector-impl-api` 0.1.0 — source of
  `Implementation`, `async_trait`, `ConnectorCallContext`,
  `ImplementationError`, `JsonValue`.
- `philharmonic-connector-impl-http-forward` 0.1.0,
  `philharmonic-connector-impl-llm-openai-compat` 0.1.0 —
  reference impls for trait shape. Do NOT copy
  retry/client/HTTP machinery — this crate has none of
  those.

If this prompt contradicts the docs above, the docs win.
Flag contradictions and stop.

## Crate state (starting point)

- `philharmonic-connector-impl-vector-search` — 0.0.0
  placeholder submodule at
  `philharmonic-connector-impl-vector-search/`. Placeholder
  `Cargo.toml`, empty-ish `src/lib.rs`, `README.md`,
  `CHANGELOG.md`, `LICENSE-*`. Never published
  substantively. Drop any aspirational `[0.0.0]` entry.
- Workspace-internal `[patch.crates-io]` already in place.

Target: `0.1.0` implementing the `vector_search` capability
as stateless in-memory cosine kNN. Pre-landing green,
working tree dirty.

## Scope

### In scope

1. `Cargo.toml`:
   - Version `0.0.0` → `0.1.0`.
   - Deps: `philharmonic-connector-impl-api = "0.1"`,
     `philharmonic-connector-common = "0.2"`, `async-trait =
     "0.1"`, `tokio = { version = "1", features = ["rt",
     "macros", "time"] }`, `serde = { version = "1",
     features = ["derive"] }`, `serde_json = "1"`,
     `thiserror = "2"`.
   - Dev-deps: `tokio` with `test-util`.
   - **No `reqwest`, no `qdrant-client`, no `ndarray`, no
     `nalgebra`.** Plain `[f32]` math.
2. Module layout per the impl spec's `vector_search`
   §"Module layout":
   - `src/lib.rs` — crate rustdoc + public `VectorSearch`
     type (ZST; `pub fn new() -> Self`) + trait impl +
     impl-api re-exports.
   - `src/config.rs` — `VectorSearchConfig` (`max_corpus_size`,
     `timeout_ms`); `deny_unknown_fields`; `max_corpus_size`
     is REQUIRED (no default, matches
     `http_forward`'s `response_max_bytes` discipline);
     `timeout_ms` optional with `default_timeout_ms() ->
     2_000`.
   - `src/request.rs` — `VectorSearchRequest`
     (`query_vector`, `corpus`, `top_k`, optional
     `score_threshold`) + `CorpusItem` (`id: String`,
     `vector: Vec<f32>`, optional `payload: Option<serde_json::Value>`).
     `deny_unknown_fields` on both.
   - `src/response.rs` — `VectorSearchResponse` (`results:
     Vec<ResultItem>`) + `ResultItem` (`id: String`,
     `score: f32`, optional `payload:
     Option<serde_json::Value>`).
   - `src/search.rs` — the actual algorithm: cosine
     similarity + top-k select. Pure math, heavily
     unit-tested. Exposes `cosine_score(&[f32], &[f32]) ->
     f32` and `rank_top_k(query: &[f32], corpus: &[CorpusItem],
     top_k: usize, score_threshold: Option<f32>) ->
     Vec<ResultItem>`.
   - `src/error.rs` — internal `Error` + `From<Error> for
     ImplementationError`.
3. Public surface:

   ```rust
   #[derive(Debug, Default, Clone, Copy)]
   pub struct VectorSearch;

   impl VectorSearch {
       pub fn new() -> Self { Self }
   }

   #[async_trait]
   impl Implementation for VectorSearch { ... }

   pub use philharmonic_connector_impl_api::{
       Implementation, ImplementationError, ConnectorCallContext,
       JsonValue, async_trait,
   };
   pub use crate::config::VectorSearchConfig;
   pub use crate::request::{VectorSearchRequest, CorpusItem};
   pub use crate::response::{VectorSearchResponse, ResultItem};
   ```

4. `execute(config, request, ctx)` flow:
   - Deserialize `config` → `VectorSearchConfig`.
     `InvalidConfig` on failure, including missing
     `max_corpus_size`.
   - Deserialize `request` → `VectorSearchRequest`.
     `InvalidRequest` on failure.
   - Validate pre-scoring (all `InvalidRequest` on
     violation):
     - `request.corpus.len() >= 1`.
     - `request.corpus.len() <= config.max_corpus_size`.
     - `request.top_k >= 1`.
     - `request.score_threshold` ∈ [-1.0, 1.0] if present.
     - **Whole-corpus vector-length pre-scan** (Qv6→A):
       every item's `vector.len()` must equal the query's
       `vector.len()`. On mismatch, error message names the
       offending offset: "corpus item 42 has vector length
       385, expected 384".
     - Reject non-finite values (NaN, ±Inf) in any vector
       (query or corpus item). Same error format as length
       mismatch.
   - Spawn scoring on `tokio::task::spawn_blocking(...)`
     (the pre-scan + scoring + sort is CPU-bound; keep it
     off the async runtime's thread even though it usually
     finishes in microseconds — the pattern is correct and
     the overhead is negligible). Wrap the spawn in
     `tokio::time::timeout(config.timeout_ms, ...)` — on
     timeout → `UpstreamTimeout`.
   - Compute `cosine_score(&query, &item.vector)` for each
     corpus item. Select top-k via a min-heap of size k
     (standard top-k-from-stream pattern) — don't sort the
     whole list then truncate.
   - Apply `score_threshold` if present: drop items with
     `score < threshold` from the result (so `results.len()
     ≤ top_k` and possibly `< top_k` when threshold bites).
   - Build `ResultItem { id, score, payload }` cloning
     from the matched corpus item.
   - Serialize `VectorSearchResponse { results }` to JSON.
     Return.
5. Cosine math (per impl spec's §"Cosine similarity
   specifics"):

   ```text
   cos(q, v) = dot(q, v) / (||q|| * ||v||)
   ```

   Safe-divide: if either norm is 0.0, score is 0.0, not
   NaN. Compute the query norm once and reuse across all
   corpus items. Result is in `[-1.0, 1.0]` naturally.

6. Unit tests colocated with each module:
   - `config::tests` — deny-unknown-fields, require
     `max_corpus_size`, default `timeout_ms`.
   - `request::tests` — deserialize happy + every
     rejection case (missing field, wrong type, non-string
     id per Qv7→A).
   - `response::tests` — round-trip serialize/deserialize,
     payload present/absent.
   - `search::tests` — heavy coverage:
     - Known cosine values: orthogonal vectors → 0.0;
       identical → 1.0; opposite → -1.0; 60° → 0.5.
     - Top-k selection with ties (reasonable
       tiebreaker — insertion order is fine; document
       it).
     - `score_threshold` cutoff behavior (below-threshold
       items dropped).
     - Empty corpus handled by caller; `rank_top_k` isn't
       called with an empty corpus.
     - Zero-norm vectors → 0.0 score (safe-divide).
     - Payload echoed only when present in corpus item
       (no synthesized `null`).
   - `error::tests` — internal variants map correctly to
     wire `ImplementationError`.
7. Integration tests under `tests/`:
   - `happy_path.rs` — small fixture corpus (say 5-10
     items), query vector, expected ranked result list.
   - `error_cases.rs` — every `InvalidRequest` variant
     triggered:
     - `corpus.len() > max_corpus_size`.
     - Empty corpus.
     - `top_k = 0`.
     - `score_threshold = 2.0` (out of range).
     - Vector-length mismatch (offending offset named).
     - NaN / Inf in vectors.
     - Non-string id (Qv7→A).
   - `score_threshold.rs` — threshold application;
     threshold that keeps zero items returns `results: []`
     (not an error).
   - `top_k_behavior.rs` — `top_k > corpus.len()` returns
     all items (not an error); exact top_k = corpus.len()
     case; large top_k.
   - `large_corpus.rs` — corpus of 2000 items with
     random-ish vectors; verifies the min-heap top-k
     approach handles the scale + `timeout_ms` doesn't
     fire at reasonable defaults.
   - Fixtures under `tests/fixtures/` as small JSON files
     (`corpus_5items.json`, `query_simple.json`, etc.).
     These are synthetic, stable, hand-authored.
8. `CHANGELOG.md` — `[0.1.0] - 2026-04-24`; drop `[0.0.0]`.
9. Crate-root rustdoc on `src/lib.rs` — density matching
   http_forward's. Explain: stateless design, target scale
   (hundreds to low thousands), cosine-only metric, why
   the corpus rides with the request instead of being
   stored.
10. `README.md` — one-paragraph expansion.

### Out of scope (flag; do NOT implement)

- Any change to impl-api, connector-common, connector-service,
  or doc 08.
- Persistent vector stores. Qdrant, pgvector, anything that
  isn't "the corpus arrived in the request." A persistent-
  store backend is a SEPARATE crate (`-vector-qdrant` or
  similar) for Phase 7 follow-up or v2; not here.
- Other distance metrics (dot product, euclidean). Cosine
  only for v1 per decision.
- Approximate nearest-neighbor algorithms (HNSW, IVF).
  Linear scan is correct at the target scale; ANN adds
  complexity without payoff for ≤ few thousand items.
- Tenant-state caching across calls.
- Retries (no network, nothing to retry).
- `cargo publish`, `git tag`, commit, push — Claude.
- Workspace-root `Cargo.toml` edits.

### Decisions fixed upstream (do NOT deviate)

From the impl spec:

1. **Stateless corpus-per-request**. No store.
2. **Cosine only**. No metric config field.
3. **`max_corpus_size` config-required**. No framework
   default; deployments choose.
4. **Whole-corpus vector-length pre-scan** (Qv6→A); error
   names the offending offset.
5. **Corpus item IDs are strings only** (Qv7→A); non-string
   → `InvalidRequest`.
6. **Min-heap top-k selection** (`O(n log k)`). Don't sort
   the whole list then truncate.
7. **No `UpstreamError`, no `UpstreamUnreachable`, no retry**
   — there's no external service.
8. **Implementation name**: `Implementation::name()` returns
   **`"vector_search"`**.

## Workspace conventions

- Edition 2024, MSRV 1.88.
- License `Apache-2.0 OR MPL-2.0`.
- `thiserror`, no `anyhow`.
- No panics in library `src/`.
- No `unsafe`.
- Rustdoc on every `pub`.
- Re-export impl-api public surface.
- Use `./scripts/*.sh` wrappers.

## Pre-landing

```sh
./scripts/pre-landing.sh philharmonic-connector-impl-vector-search
```

All tests run in CI (no env-var gating, no external
services). Must pass green.

## Git

You do NOT commit, push, branch, tag, or publish. Leave
dirty. Claude commits and publishes post-review.

## Deliverables

1. Populated `Cargo.toml` at `0.1.0`.
2. `src/` with full impl + colocated unit tests (heavy
   coverage in `search.rs`).
3. `tests/` with integration tests + JSON fixtures.
4. `CHANGELOG.md` `[0.1.0] - 2026-04-24`.
5. Crate-root rustdoc + expanded `README.md`.

Working tree: dirty.

## Structured output contract

1. **Summary** (3–6 sentences).
2. **Files touched**.
3. **Verification results** — pre-landing output, test
   counts (all passing, no `#[ignore]`-d tests expected).
4. **Residual risks / TODOs**.
5. **Git state**.
6. **Dep versions used**.

## Follow-through, completeness, verification, missing-context, action safety

Same rules as earlier dispatches:

- Pre-landing failures: fix, re-run.
- Spec ambiguity: pick minimal-public-surface
  interpretation; flag.
- Floating-point numeric exactness in tests: use
  appropriate tolerance (e.g., `assert!((a - b).abs() <
  1e-6)`) for cosine values that involve `sqrt`.
  Integer values (top_k indexing) stay exact.
- Stop and flag if: required docs missing, impl-api
  surface differs.
- No `cargo publish`, no `git push`, no branch creation,
  no tags.
- No edits outside `philharmonic-connector-impl-vector-search/`.
- No destructive ops.

Verification loop:

```sh
./scripts/pre-landing.sh philharmonic-connector-impl-vector-search
cargo test -p philharmonic-connector-impl-vector-search --all-targets
git -C philharmonic-connector-impl-vector-search status --short
git -C . status --short
```

---

## Outcome

Pending — will be updated after Codex run.
