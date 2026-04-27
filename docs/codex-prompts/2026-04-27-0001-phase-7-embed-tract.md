# Phase 7 Tier 1 — `embed` rewrite to tract / tokenizers (round 02 dispatch)

**Date:** 2026-04-27
**Slug:** `phase-7-embed-tract`
**Round:** 02 (clean rewrite on top of round-01 fastembed checkpoint)
**Subagent:** `codex:codex-rescue`

## Motivation

Tier 1 wave 1 published 2026-04-27
(`philharmonic-connector-impl-sql-postgres` 0.1.0,
`philharmonic-connector-impl-sql-mysql` 0.1.0,
`philharmonic-connector-impl-vector-search` 0.1.0). Wave 2 is the
embed crate, blocked since 2026-04-24 on a library-choice pivot:
the round-01 `fastembed` + `ort` implementation linked against the
glibc-only ORT prebuilt runtime, and the deployment targets
include musl. Yuka picked pure-Rust `tract` + `tokenizers` for
musl-native inference. This round produces the rewrite.

This is a **clean rewrite**, not a migration — round-01's
fastembed code is replaced module-by-module. The wire protocol
(`EmbedConfig` / `EmbedRequest` / `EmbedResponse`) survives
unchanged; the public surface changes minimally
(`tokenizer_files: TokenizerFiles` → `tokenizer_json_bytes:
&[u8]`); the inference pipeline is all new.

Non-crypto task: no Gate 1/2, no key material.

## References (read before coding)

- **Authoritative impl spec / pivot plan**:
  [`docs/notes-to-humans/2026-04-24-0008-phase-7-embed-tract-pivot-plan.md`](../notes-to-humans/2026-04-24-0008-phase-7-embed-tract-pivot-plan.md).
  Read end-to-end before writing any code. If anything in this
  prompt contradicts that plan, the plan wins — flag and stop.
- [`docs/notes-to-humans/2026-04-26-0001-phase-7-tier-1-next-steps-and-ambiguities.md`](../notes-to-humans/2026-04-26-0001-phase-7-tier-1-next-steps-and-ambiguities.md)
  §"Ambiguities that need a Yuka call" — the five ambiguities
  (B/C/D/E/F) flagged before this dispatch. Yuka's calls are
  baked into the **Decisions fixed upstream** section below.
- [`docs/notes-to-humans/2026-04-27-0002-phase-7-embed-default-bundled-model-architecture.md`](../notes-to-humans/2026-04-27-0002-phase-7-embed-default-bundled-model-architecture.md)
  — durable record of Yuka's 2026-04-27 architectural
  decision to bundle a default model in the lib crate via
  `build.rs`. Supersedes the pivot plan's §"Public surface"
  and §"Testing"; companion to this prompt.
- [`docs/design/08-connector-architecture.md`](../design/08-connector-architecture.md)
  §"Embedding and vector search" — wire-shape narrative; the
  pivot plan fills in the concrete fields.
- [`docs/codex-prompts/2026-04-24-0005-phase-7-embed.md`](2026-04-24-0005-phase-7-embed.md)
  — round-01 prompt. Use it for the wire-shape / module-layout
  reference; ignore the fastembed-specific sections (deps,
  inference pipeline, test env-var set).
- [`ROADMAP.md`](../../ROADMAP.md) §"Phase 7" — Tier 1 wave 1
  (published) + wave 2 (this work).
- [`CONTRIBUTING.md`](../../CONTRIBUTING.md):
  - §4 git workflow (signed-off, signed, scripts only).
  - §5 script wrappers over raw cargo.
  - §10.3 no panics in library `src/`.
  - §10.4 libraries take bytes, not file paths. **Strongly
    relevant** — `Embed::new_from_bytes(...)` keeps the
    bytes-only constraint; the crate must NOT read files,
    env vars, or the network at runtime. (build.rs is
    bin-territory and CAN read env vars; see §"Test fixture
    mechanism" below.)
  - §11 pre-landing checks.
- `philharmonic-connector-impl-api` 0.1.x — source of
  `Implementation`, `async_trait`, `ConnectorCallContext`,
  `ImplementationError`, `JsonValue`. Verify exact published
  version with `./scripts/xtask.sh crates-io-versions -- philharmonic-connector-impl-api`
  before pinning.
- `philharmonic-connector-common` 0.2.x — verify exact published
  version the same way.
- `philharmonic-connector-impl-vector-search` 0.1.0 (just
  published) — reference for trait-impl shape on a
  recently-landed Tier 1 crate.

If this prompt contradicts the docs above, the docs win. Flag
contradictions and stop.

## Crate state (starting point)

- `philharmonic-connector-impl-embed` — submodule at
  `philharmonic-connector-impl-embed/`. Currently contains the
  round-01 fastembed checkpoint at version `0.1.0` in
  `Cargo.toml`. **Not published** to crates.io
  (`crates-io-versions -- philharmonic-connector-impl-embed`
  returned 404 prior to wave 1; same is true now). Layout
  per round-01:
  - `src/{lib,config,request,response,model,error}.rs`
  - `tests/{batch_inference,batch_size_enforcement,common,deterministic_output,inference_produces_correct_shape,semantic_similarity}.rs`
    + `tests/README.md`
  - `Cargo.toml` (fastembed deps), `CHANGELOG.md` (0.1.0
    entry mentioning fastembed), `README.md`, `LICENSE-*`.
- Workspace-internal `[patch.crates-io]` already in place.

Target: `0.1.0` implementing the `embed` capability via tract +
tokenizers + ndarray, pre-landing green, working tree dirty.

## Decisions fixed upstream (do NOT deviate)

These are the calls Yuka made on 2026-04-27 against the
ambiguities surfaced in
[`2026-04-26-0001-phase-7-tier-1-next-steps-and-ambiguities.md`](../notes-to-humans/2026-04-26-0001-phase-7-tier-1-next-steps-and-ambiguities.md):

1. **(B) Default-bundled model in the lib crate.** The lib's
   own `build.rs` — gated by a default-on Cargo feature
   `bundled-default-model` — fetches an ONNX +
   `tokenizer.json` bundle from HuggingFace at lib build
   time, caches it outside the repo, and `include_bytes!`s
   the bytes into the lib so consumers get
   `Embed::new_default()` without supplying bytes.
   **`Embed::new_from_bytes(...)` remains** for explicit-bytes
   consumers and works regardless of the feature.
   **Default model: `BAAI/bge-m3`** (multilingual, 1024-dim,
   ~2.3GB ONNX), pinned to a specific HF revision SHA at
   dispatch time so builds are reproducible.

   **Knobs**:
   - **Skip the bundle**: Cargo feature
     `bundled-default-model` (default-on); opt out via
     `default-features = false` in a downstream `Cargo.toml`
     or `cargo build --no-default-features`. Idiomatic
     Cargo opt-out. When the feature is off,
     `Embed::new_default()` does not exist (cfg-out) and
     build.rs makes no network calls.
   - **Choose the model**: env vars
     `PHILHARMONIC_EMBED_DEFAULT_MODEL=<HF repo>` +
     `PHILHARMONIC_EMBED_DEFAULT_REVISION=<sha>`,
     compile-time-read by build.rs. Required together; no
     implicit `main`/`HEAD`.
   - **Auto-skip on docs.rs**: build.rs checks `DOCS_RS=1`
     even when the feature is on, and treats it as a
     belt-and-suspenders skip — covers downstream consumers
     whose docs.rs build would otherwise hit the network.
     Plus `Cargo.toml` carries
     `[package.metadata.docs.rs] no-default-features = true`
     so this crate's own docs.rs build skips cleanly.
   - **Cache location**:
     `$XDG_CACHE_HOME/philharmonic/embed-bundles/` with
     `$HOME/.cache/...` fallback; overridable via
     `PHILHARMONIC_EMBED_CACHE_DIR`. Outside the repo so it
     needs no `.gitignore` entry.

   **For test cycles**: bge-m3 is overkill — verification
   commands export the override env to a smaller
   multilingual model so test cycles aren't dominated by
   2.3GB include_bytes. See §"Default-bundled model
   architecture" below for the complete mechanism + the
   trade-offs Yuka explicitly accepted on 2026-04-27.

   **This decision supersedes** the pivot plan's
   §"Public surface" (which did not contemplate
   `Embed::new_default()`) and §"Testing" (which assumed
   env-var-gated `#[ignore]` tests). Where this prompt and
   the pivot plan disagree on the default-bundled-model
   architecture specifically, this prompt wins; on tract
   ops, tokenizer handling, error mapping, and the
   `tokenizer.json` single-file change, the pivot plan
   still wins.

2. **(C) Op-coverage strategy: verify early, fail fast.** Before
   writing any of the wrapper / pool / inference code, perform
   the op-coverage probe (see §"Phased dispatch" below). If
   the probe fails — any required ONNX op rejected by tract —
   STOP and flag. Do not write scaffolding around an
   unloadable model.

3. **(D) Round shape: clean rewrite.** Delete and rewrite all
   six `src/*.rs` files. Delete and rewrite all six
   `tests/*.rs` files (and `tests/common.rs` and
   `tests/README.md`). Refer to round-01 only as a wire-shape
   reference for the structs that survive (`EmbedConfig`,
   `EmbedRequest`, `EmbedResponse` — exact field-by-field
   match expected per the pivot plan). Do not import
   round-01's fastembed-specific code or comments. The
   only structural element preserved is the file layout
   (six modules, with `pool.rs` added; six tests).

4. **(E) Test vectors: shape-only.** Tests assert tensor shape,
   batch size, dimension count, L2-normalization within ε,
   determinism (same input → same output bit-for-bit on
   tract), and a coarse semantic sanity check
   (`cosine(embed("hello"), embed("hi")) >
   cosine(embed("hello"), embed("goodbye"))`). **Do NOT**
   commit a Python-generated reference vector or a
   tract-self-generated reference vector. No regression test
   that pins exact float values across runtime versions —
   too brittle, and embed is not a crypto path so the
   crypto-grade test-vector discipline does not apply.

5. **(F) Publishing: scripts only.** Codex does NOT run
   `cargo publish`. The Tier 1 wave 2 publish (this crate,
   alone) is a Claude post-review step using
   `./scripts/publish-crate.sh philharmonic-connector-impl-embed`
   per `CONTRIBUTING.md` §4. No new wrapper script needed.
   Codex also does NOT commit, push, or tag (see §"Git"
   below).

## Default-bundled model architecture

The lib carries a default ONNX + tokenizer bundle, fetched at
lib build time by the crate's own `build.rs`, cached outside
the repo, and `include_bytes!`-d so callers get
`Embed::new_default()` without supplying bytes. This is a
deliberate departure from the bytes-only library design that
governed round-01 and the pivot plan — Yuka's call on
2026-04-27, accepting the trade-offs below.

### Trade-offs Yuka has explicitly accepted

- **Build-script network IO at lib build time** — widely
  considered hostile by downstream packagers (Debian, NixOS,
  Bazel-style sandboxes). They opt out via the Cargo feature.
  crates.io itself does not block build-script network access;
  consumers of the published crate trigger an HF fetch on
  first build unless they disable the default feature.
- **~2.3GB ONNX baked into the lib's static bytes** when the
  default holds. Compile times balloon (cargo includes 2.3GB
  per build), linker passes are slower, output binaries are
  ~2.3GB+. Acceptable for the deployment binary; painful for
  routine dev iteration — hence the test-cycle small-model
  override.
- **docs.rs / offline builds** can't reach the network;
  handled by `[package.metadata.docs.rs] no-default-features
  = true` on this crate, plus build.rs's `DOCS_RS=1`
  belt-and-suspenders auto-skip for downstream consumer
  doc builds.
- **Reproducibility** depends on pinning an HF revision SHA,
  not `main`. build.rs requires
  `PHILHARMONIC_EMBED_DEFAULT_REVISION` paired with any
  non-default `PHILHARMONIC_EMBED_DEFAULT_MODEL`; the bge-m3
  default revision is baked into build.rs at dispatch time.
  No implicit `main` / `HEAD`.

### Cargo features

```toml
[features]
default = ["bundled-default-model"]
bundled-default-model = []

[package.metadata.docs.rs]
no-default-features = true
```

`bundled-default-model = []` (empty feature, no extra deps)
is the gate for both the build.rs fetch AND the
`Embed::new_default()` constructor. Off → no network in
build.rs, no `new_default()`, `new_from_bytes` still works.

### Env-var knobs (read by build.rs at compile time)

- `PHILHARMONIC_EMBED_DEFAULT_MODEL` — HF repo id
  (e.g. `BAAI/bge-m3`,
  `sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2`).
  Default: `BAAI/bge-m3`.
- `PHILHARMONIC_EMBED_DEFAULT_REVISION` — pinned commit SHA
  on the model repo. **Required** when
  `PHILHARMONIC_EMBED_DEFAULT_MODEL` is set; defaults to the
  bge-m3 SHA Codex bakes into build.rs at dispatch time when
  the model env var is unset. No `main`, no `HEAD` — both
  rejected with a clear error.
- `PHILHARMONIC_EMBED_CACHE_DIR` — override the cache root.
  Default: `$XDG_CACHE_HOME/philharmonic/embed-bundles/` if
  set, else `$HOME/.cache/philharmonic/embed-bundles/`.
- `DOCS_RS` — auto-detected; non-empty value skips the fetch
  even with the feature on. build.rs emits no
  `embed_default_bundle` cfg → `Embed::new_default()` is
  cfg-out → docs build cleanly without a bundle.

build.rs also emits the appropriate
`cargo:rerun-if-env-changed=...` lines for each knob so
cargo recompiles when the knobs flip.

### Cache layout

```
$cache_root/
└── <sanitized-model>__<revision-sha-prefix-12>/
    ├── manifest.json     (model id, full revision SHA, fetched-at, sha256 per file)
    ├── model.onnx
    ├── tokenizer.json
    └── config.json       (used to extract dimensions + max_seq_length)
```

`<sanitized-model>` = HF repo id with `/` → `__`.
`<revision-sha-prefix-12>` = first 12 chars of the pinned
SHA. Cache hit = manifest is present, every listed file
exists, and every file's SHA256 matches manifest. Cache
miss = re-fetch from HF. SHA mismatch on a cached file
(corrupted cache, revision SHA collision attempt) = explicit
error, no silent overwrite — operator must `rm -rf` the
cache entry to force re-fetch.

### build.rs HTTP stack

`[build-dependencies]`: `ureq` (rustls TLS, no native-tls)
+ `serde_json` + `sha2` + `dirs` (cache root resolution).
**Not `reqwest`** — build.rs is workspace tooling per
[`CONTRIBUTING.md §10.9`](../../CONTRIBUTING.md#109-http-client-runtime-stack-vs-tooling-stack)
(the tooling-stack rule applies even though build.rs
runs on consumer machines), and `reqwest` would drag in a
tokio runtime at build time.

Each build-dep version verified via `crates-io-versions`
during Phase 0 (3-day cooldown rule).

### Public surface

```rust
pub struct Embed { /* private */ }

impl Embed {
    /// Construct from caller-supplied bytes. Always available.
    pub fn new_from_bytes(
        model_id: impl Into<String>,
        onnx_bytes: Vec<u8>,
        tokenizer_json_bytes: &[u8],
        dimensions: usize,
        max_seq_length: usize,
    ) -> Result<Self, ImplementationError>;

    /// Construct from the build-time-bundled default model.
    /// Available only when the `bundled-default-model` feature
    /// is on AND build.rs successfully bundled (i.e., not on
    /// docs.rs). Bundled `model_id`, `dimensions`,
    /// `max_seq_length` come from the cached `config.json`
    /// + manifest.
    #[cfg(all(feature = "bundled-default-model", embed_default_bundle))]
    pub fn new_default() -> Result<Self, ImplementationError>;

    pub fn model_id(&self) -> &str;
    pub fn dimensions(&self) -> usize;
}
```

### Test workflow

- **Unit tests run regardless of bundle state.** Pure math,
  wire-shape, error-mapping. The `include_bytes!` for the
  default bundle still happens at compile time when the
  feature is on (the bytes ride in the test binary), but
  unit tests don't load them into tract.
- **Integration tests under `tests/`** call
  `Embed::new_default()`, gated `#[cfg(all(feature =
  "bundled-default-model", embed_default_bundle))]`. With
  the feature off OR build.rs skipped, they compile out
  cleanly.
- **For workspace pre-landing**, override the default model
  to a small multilingual one so iteration isn't dominated
  by 2.3GB include_bytes:

  ```sh
  export PHILHARMONIC_EMBED_DEFAULT_MODEL=sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2
  export PHILHARMONIC_EMBED_DEFAULT_REVISION=<pinned-sha-codex-picks>
  ./scripts/pre-landing.sh philharmonic-connector-impl-embed
  ```

  Codex picks the small-model SHA in Phase 0 and uses it
  in the verification commands. **A `scripts/embed-test.sh`
  wrapper** that automates the env-var setup is a Claude
  post-review polish — out of scope for this round.

## Scope

### In scope

1. **`Cargo.toml`**:
   - Version stays `0.1.0` (round-01 was a never-published
     checkpoint; the version line is correct as-is).
   - **Runtime deps** (replace fastembed dep set):
     - `async-trait = "0.1"`
     - `philharmonic-connector-common = "<verify-via-xtask>"` (currently 0.2.x)
     - `philharmonic-connector-impl-api = "<verify-via-xtask>"` (currently 0.1.x)
     - `serde = { version = "1", features = ["derive"] }`
     - `serde_json = "1"`
     - `thiserror = "2"`
     - `tokio = { version = "1", features = ["rt", "macros", "time"] }`
     - `tract-onnx = "<verify-latest-via-xtask>"` (pivot plan
       names 0.22 as the floor; verify the latest non-yanked)
     - `tokenizers = { version = "<verify>", default-features = false, features = ["onig"] }`
       (`default-features = false` drops `http` / `hf-hub`
       fetching — no runtime HF)
     - `ndarray = "<verify-latest-via-xtask>"`
   - **Build deps** (only used by `build.rs` when the
     `bundled-default-model` feature is on, per
     §"Default-bundled model architecture"):
     - `ureq = { version = "<verify>", default-features = false, features = ["rustls"] }`
       (rustls TLS, no native-tls)
     - `serde = { version = "1", features = ["derive"] }`
     - `serde_json = "1"`
     - `sha2 = "<verify>"`
     - `dirs = "<verify>"` (cache root resolution —
       `data_dir` / `cache_dir` portable)
     **Not `reqwest`** in build-deps — build.rs is workspace
     tooling per CONTRIBUTING.md §10.9 even when running on
     consumer machines.
   - **Features**:
     ```toml
     [features]
     default = ["bundled-default-model"]
     bundled-default-model = []

     [package.metadata.docs.rs]
     no-default-features = true
     ```
   - **Drop entirely**: `fastembed`, `ort`, `ort-sys`,
     anything ort-derived.
   - **Dev-deps**: `tokio = { version = "1", features = ["rt",
     "rt-multi-thread", "macros", "time", "test-util"] }`.
     Add `approx = "<verify>"` (or latest-verified) for
     L2-normalization assertions within ε.
   - **Cooldown rule**: every dep version added or bumped
     (runtime, build, dev) must pass the 3-day cooldown via
     `./scripts/xtask.sh crates-io-versions -- <crate>`
     before committing `Cargo.toml`. If any has a release
     within the last 3 days, pin to the prior version and
     note in residual risks.
   - **No `reqwest`**, no TLS stack, no `hyper` in runtime
     deps — this crate has no network at runtime. (Build deps
     have `ureq` + rustls for the bundle fetch, which is
     compile-time only.)

2. **Module layout** (per pivot plan §"Target module layout",
   plus a new `build.rs` and a new `Embed::new_default()` in
   `lib.rs`):
   - `build.rs` — implements the default-bundled-model fetch
     per §"Default-bundled model architecture". Reads the
     `CARGO_FEATURE_BUNDLED_DEFAULT_MODEL` /
     `PHILHARMONIC_EMBED_DEFAULT_MODEL` /
     `PHILHARMONIC_EMBED_DEFAULT_REVISION` /
     `PHILHARMONIC_EMBED_CACHE_DIR` / `DOCS_RS` env vars;
     short-circuits cleanly when the feature is off OR
     `DOCS_RS=1`; resolves the cache dir; cache-hits via
     SHA256 manifest verification or fetches via
     `ureq`+rustls and writes the manifest; emits
     `cargo:rustc-cfg=embed_default_bundle`,
     `cargo:rustc-env=EMBED_DEFAULT_BUNDLE_DIR=<path>`,
     `cargo:rustc-env=EMBED_DEFAULT_MODEL_ID=<HF id>`,
     `cargo:rustc-env=EMBED_DEFAULT_REVISION=<sha>`,
     `cargo:rustc-env=EMBED_DEFAULT_DIMENSIONS=<n>` (parsed
     from the bundle's `config.json` `hidden_size` field),
     `cargo:rustc-env=EMBED_DEFAULT_MAX_SEQ_LENGTH=<n>`
     (parsed from `max_position_embeddings` or sensible
     fallback). Plus the appropriate
     `cargo:rerun-if-env-changed=...` and
     `cargo:rerun-if-changed=$bundle/manifest.json` lines.
     **Build-script panics are acceptable on fetch failure**
     (build-script context is exempt from §10.3 panic rules);
     the panic message must clearly name the model + revision
     + URL that failed and suggest setting
     `PHILHARMONIC_EMBED_NO_BUNDLE`-style opt-out (i.e.,
     `--no-default-features`).
   - `src/lib.rs` — crate rustdoc + public `Embed` type +
     `Embed::new_from_bytes(...)` constructor +
     `#[cfg(all(feature = "bundled-default-model",
     embed_default_bundle))] Embed::new_default()`
     constructor + trait impl + impl-api re-exports.
     Rustdoc explains both constructors, the
     bytes-bundled-into-binary architecture for `new_default`,
     the no-HF-at-runtime constraint (network is build-time
     only, and only when the feature is on), the
     deployment integration pattern, the `tokenizer.json`
     single-file change relative to round-01, the tract
     op-coverage constraint (BERT-class /
     sentence-transformers known to work; other architectures
     need probing first via §"Phase 1"), and the way
     `--no-default-features` opts out of the bundle.
   - `src/config.rs` — `EmbedConfig` (exact field set
     unchanged from round-01: `model_id: String`,
     `max_batch_size: usize` default 32, `timeout_ms: u64`
     default 10_000); `deny_unknown_fields`; default helpers.
   - `src/request.rs` — `EmbedRequest { texts: Vec<String> }`;
     `deny_unknown_fields`.
   - `src/response.rs` — `EmbedResponse { embeddings:
     Vec<Vec<f32>>, model: String, dimensions: usize }`.
   - `src/model.rs` — wraps the tract `SimplePlan` + the
     `tokenizers::Tokenizer` + `model_id` / `dimensions` /
     `max_seq_length`. Owns the `forward(&self, texts:
     &[String]) -> Result<Vec<Vec<f32>>, Error>` method that
     does tokenize → input-tensor build → tract run →
     mean-pool with attention mask → L2-normalize → return
     `Vec<Vec<f32>>`. The pool/normalize step lives in
     `pool.rs`; `model.rs` calls into it.
   - `src/pool.rs` — pure math: `mean_pool_with_mask(last_hidden_state:
     &Array3<f32>, attention_mask: &Array2<i64>) ->
     Array2<f32>` followed by `l2_normalize_rows(&mut
     Array2<f32>)`. Heavy unit-test coverage with
     hand-computed expected values.
   - `src/error.rs` — internal `Error` enum with variants
     for tract load/run failures, tokenizer failures,
     shape mismatches, etc. `From<Error> for
     ImplementationError` mapping per the pivot plan
     §"Error cases":
     - tokenizer / tensor / tract Internal → `Internal`.
     - timeout → `UpstreamTimeout`.
     - config-shape / model_id mismatch → `InvalidConfig`.
     - batch overflow / empty texts → `InvalidRequest`.
     - **No `UpstreamError`, no `UpstreamUnreachable`** —
       there is no upstream.

3. **Public surface** (the canonical signature lives in
   §"Default-bundled model architecture" → §"Public surface"
   above; reproduce here):

   ```rust
   pub struct Embed { /* private */ }

   impl Embed {
       pub fn new_from_bytes(
           model_id: impl Into<String>,
           onnx_bytes: Vec<u8>,
           tokenizer_json_bytes: &[u8],
           dimensions: usize,
           max_seq_length: usize,
       ) -> Result<Self, ImplementationError>;

       #[cfg(all(feature = "bundled-default-model", embed_default_bundle))]
       pub fn new_default() -> Result<Self, ImplementationError>;

       pub fn model_id(&self) -> &str;
       pub fn dimensions(&self) -> usize;
   }

   #[async_trait]
   impl Implementation for Embed { ... }

   pub use philharmonic_connector_impl_api::{
       Implementation, ImplementationError, ConnectorCallContext,
       JsonValue, async_trait,
   };
   ```

   `new_default()` body uses
   `include_bytes!(concat!(env!("EMBED_DEFAULT_BUNDLE_DIR"), "/model.onnx"))`
   and `include_bytes!(concat!(env!("EMBED_DEFAULT_BUNDLE_DIR"), "/tokenizer.json"))`,
   plus `env!("EMBED_DEFAULT_MODEL_ID")`, `env!("EMBED_DEFAULT_DIMENSIONS")`
   (parsed at compile time via `const _: usize = ...` pattern),
   `env!("EMBED_DEFAULT_MAX_SEQ_LENGTH")`. Delegates to
   `new_from_bytes`. Note: `include_bytes!` of a 2.3GB ONNX
   does noticeably increase compile + link time; the
   workspace pre-landing flow uses a smaller multilingual
   model via the override env vars to keep iteration fast
   (see §"Default-bundled model architecture" → §"Test
   workflow").

   NO re-export of `fastembed::TokenizerFiles` (gone). NO
   re-export of any tract type; `tract-onnx` is an impl
   detail. Eager load: `new_from_bytes` parses the ONNX with
   `tract_onnx::onnx().model_for_read(...)` and the
   `tokenizer.json` with `tokenizers::Tokenizer::from_bytes`,
   converts the model to a typed runnable plan, validates
   `dimensions` and `max_seq_length` against the plan's
   declared input/output shapes if discoverable, and returns
   `Embed`. Failure → `ImplementationError::InvalidConfig {
   detail }`.

4. **`execute(config, request, ctx)` flow** (per pivot plan
   §"Inference pipeline"):
   - Deserialize config → `EmbedConfig`. `InvalidConfig` on
     failure.
   - Validate `config.model_id == self.model_id`. Mismatch →
     `InvalidConfig { detail: "config model_id 'X' does not
     match loaded model 'Y'" }`.
   - Deserialize request → `EmbedRequest`. `InvalidRequest`
     on failure.
   - Validate `request.texts.len() >= 1` and `<=
     config.max_batch_size`. Violations → `InvalidRequest`.
   - `tokio::task::spawn_blocking(...)` the tokenize +
     forward + pool + normalize block (CPU-bound). Wrap in
     `tokio::time::timeout(config.timeout_ms, ...)`. On
     timeout → `UpstreamTimeout`. On `JoinError` →
     `Internal`. On internal `Error` → mapped per `error.rs`.
   - Build `EmbedResponse { embeddings, model:
     self.model_id.clone(), dimensions: self.dimensions }`.
     `serde_json::to_value` → return.

5. **Tokenizer configuration**: the `tokenizers` crate requires
   explicit padding/truncation if the `tokenizer.json` doesn't
   carry it. At `new_from_bytes`, call
   `tokenizer.with_padding(...)` and `tokenizer.with_truncation(...)`
   with `max_seq_length` (the caller-supplied parameter) so
   batch tokenization always emits same-length sequences.
   Padding strategy: `BatchLongest` is fine for variable-length
   batches; the pivot plan §"Inference pipeline" assumes fixed
   `seq = max_seq_length`. Use `Fixed(max_seq_length)` for
   determinism.

6. **`token_type_ids` handling** (pivot plan §"Risks" item 3):
   detect from the loaded model's input signature whether
   `token_type_ids` is required. If yes, build the tensor as a
   zero `Array2::<i64>` (BERT's standard "single-segment"
   convention); if no, omit. Wire this into `model.rs` once,
   not per-call.

7. **Pool / normalize implementation** — `pool.rs`:
   - `mean_pool_with_mask`: for each batch row b, for each
     dimension d, `sum[b][d] = Σ_j last[b][j][d] *
     mask[b][j]`; `count[b] = Σ_j mask[b][j]`;
     `pooled[b][d] = sum[b][d] / max(count[b], 1)` (the
     `max(_, 1)` is the safe-divide guard).
   - `l2_normalize_rows`: in-place; for each row b,
     `norm[b] = sqrt(Σ_d pooled[b][d]^2)`; if `norm[b] >
     EPS` (e.g. `1e-12`), divide; else leave as-is.

8. **Unit tests colocated with each module**:
   - `config::tests` — deny-unknown-fields, defaults for
     `max_batch_size` / `timeout_ms`, model_id required,
     batch-size > 0.
   - `request::tests` — deserialize valid; reject non-string
     elements, empty array, missing field, unknown field.
   - `response::tests` — round-trip serialize/deserialize.
   - `error::tests` — every internal variant maps to the
     intended `ImplementationError` variant.
   - `pool::tests` — comprehensive math coverage:
     - mean-pool with all-1 mask gives mean of inputs;
     - mean-pool with mixed-mask correctly excludes masked
       positions;
     - mean-pool with all-0 mask returns zero vector
       (safe-divide path);
     - L2-normalize of unit vector is itself;
     - L2-normalize of 3·unit-vector is unit-vector;
     - L2-normalize of zero vector is zero vector
       (epsilon-guard path);
     - Hand-computed expected values for at least one
       small (batch=2, seq=3, hidden=4) case so the test
       reads as a worked example.
   - No `model::tests` that load real ONNX bytes —
     real-model tests live under `tests/` (see below).

9. **Integration tests under `tests/`** (per Yuka's call E,
   shape-only — NO reference-vector regression test):
   - **Gated `#[cfg(all(feature = "bundled-default-model",
     embed_default_bundle))]`**, no `#[ignore]`. When the
     feature is on AND build.rs successfully bundled (i.e.
     not `--no-default-features`, not on docs.rs), `cargo
     test` exercises them via `Embed::new_default()`. When
     either gate is off, they compile out cleanly with unit
     tests still passing.
   - Tests:
     - `inference_produces_correct_shape.rs` — single
       text → one vector; vector length matches
       `Embed::dimensions()`; model-echo matches the
       bundled model's id (`env!("EMBED_DEFAULT_MODEL_ID")`).
     - `batch_inference.rs` — batch of 3 texts → 3
       vectors, each correct length, distinct (unless the
       texts happen to be identical, which they aren't).
     - `deterministic_output.rs` — same input twice →
       byte-identical output (tract is deterministic under
       fixed input + fixed model bytes; assert exact float
       equality, not ε-tolerant).
     - `semantic_similarity.rs` — `cosine(embed("hello"),
       embed("hi")) > cosine(embed("hello"),
       embed("goodbye"))`. Coarse sanity check.
     - `batch_size_enforcement.rs` — batch overrun gets
       `InvalidRequest`; max-batch-size boundary passes.
     - `l2_normalized.rs` — every output vector has
       norm 1 within ε (e.g. `1e-5`).
   - `tests/common.rs` (or `tests/common/mod.rs`) — shared
     helper that constructs an `Embed` via
     `Embed::new_default()` and surfaces the bundled
     `model_id`, `dimensions`, `max_seq_length` as
     compile-time constants drawn from the build.rs-emitted
     env vars (`env!("EMBED_DEFAULT_MODEL_ID")`,
     `env!("EMBED_DEFAULT_DIMENSIONS")`,
     `env!("EMBED_DEFAULT_MAX_SEQ_LENGTH")`). No runtime env
     reads.
   - `tests/README.md` — short doc explaining the
     `bundled-default-model` feature flag, the small-model
     override env vars for fast test cycles, the cache
     location, and `--no-default-features` for offline
     iteration.

10. **`CHANGELOG.md`** — replace the existing `[0.1.0] -
    2026-04-24` entry with a `[0.1.0] - 2026-04-27` entry
    describing the tract / tokenizers / ndarray stack, the
    no-network-at-runtime constraint, the
    `Embed::new_from_bytes` and `Embed::new_default`
    constructors, and the `bundled-default-model` feature
    flag with its env-var knobs. The fastembed mention in the
    old entry goes away — that code never published.
    (Append-only-history rule does not apply to file edits;
    the round-01 commit stays in the git log but the
    CHANGELOG state-of-the-world reflects what actually
    published.)

11. **`README.md`** — refresh for the tract stack; one
    paragraph each on `new_from_bytes` (caller-supplies-bytes
    pattern, the deployment xtask → include_bytes! →
    `new_from_bytes` flow) and `new_default`
    (default-bundled-model feature, env-var knobs,
    `--no-default-features` opt-out).

12. **Crate-root rustdoc on `src/lib.rs`** — density matching
    `philharmonic-connector-impl-vector-search/src/lib.rs`
    (the most recent Tier 1 reference): explain the
    architecture, the no-network-at-runtime constraint, both
    constructor paths, the `bundled-default-model` feature,
    deployment pattern, and tract's role.

### Out of scope (flag; do NOT implement)

- Any change to `philharmonic-connector-impl-api`,
  `philharmonic-connector-common`, `connector-service`, or
  the design docs.
- HF Hub fetching at **runtime** in the library. The crate
  must NOT make an HTTP call, must NOT read a filesystem
  path, must NOT read env vars at runtime. All bytes flow
  through `new_from_bytes` or the build.rs-baked
  `include_bytes!`. (build.rs network IO at compile time
  IS in scope — that's the bundled-default-model
  architecture.)
- Changes to the `xtask hf-fetch-embed-model` bin. It stays
  as-is; build.rs has its own minimal HF fetcher (parallel
  implementation rather than reused dep) since the embed
  crate can't depend on the workspace `xtask/` from a
  published-tarball context.
- A new `scripts/embed-test.sh` wrapper. Claude adds that
  post-review if it earns its keep — the verification
  commands in this prompt set the override env vars
  inline.
- Multi-model support within a single `Embed` instance.
- Streaming / incremental embeds, quantization controls, GPU
  inference.
- A reference-vector regression test (Yuka's call E:
  shape-only).
- Per-model Cargo features (`bundled-bge-m3`,
  `bundled-mini-lm`, etc.). Model selection at lib build
  time is via env vars, not features. Only one
  bundle-vs-no-bundle feature exists.
- `cargo publish`, `git tag`, commit, push — Claude's job
  after review.
- Workspace-root `Cargo.toml` edits.

## Phased dispatch

Execute in this order. Stop at any phase that fails the
listed gate.

### Phase 0 — Setup + cooldown + revision pinning

1. Read the pivot plan, the
   round-01 prompt, and the §"Default-bundled model
   architecture" section above end-to-end.
2. Run `./scripts/xtask.sh crates-io-versions -- <crate>` for
   each of the runtime deps (`tract-onnx`, `tokenizers`,
   `ndarray`, `philharmonic-connector-impl-api`,
   `philharmonic-connector-common`), build deps (`ureq`,
   `sha2`, `dirs`, `serde_json`), and dev deps (`approx`).
   Pick versions that pass the 3-day cooldown.
3. **Pin the bge-m3 default revision**: visit
   <https://huggingface.co/BAAI/bge-m3/commits/main> (you
   can `curl` it via `./scripts/web-fetch.sh` or just resolve
   from the HF API). Pick a recent stable commit SHA; this
   becomes the build.rs-baked default for
   `PHILHARMONIC_EMBED_DEFAULT_REVISION`.
   - Also verify bge-m3 has accessible ONNX + tokenizer.json
     at the standard HF paths (`onnx/model.onnx` or
     similar, and `tokenizer.json` at the repo root). If
     bge-m3's ONNX export lives at a non-standard path
     (e.g. `onnx_int8/model.onnx`), document the path in
     build.rs as a per-model fetch-config map. If bge-m3
     has no usable ONNX export at all, **STOP and flag** —
     this would force picking a different default model
     and reopen Yuka's call B.
4. **Pin a small multilingual test-cycle model + revision**:
   recommend `sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2`
   (already named in the pivot plan as op-set-clean) or
   `intfloat/multilingual-e5-small`. Pick a recent
   revision SHA. This pair is what the workspace
   pre-landing flow uses via the override env vars; record
   in the structured-output **Verification results**.

### Phase 1 — Op-coverage probe (Yuka's call C: verify early)

The probe runs against **bge-m3** first (it's the binding
production default; if it fails, the whole architecture
falls), then optionally against the small test-cycle model
(the pivot plan already vetted MiniLM-class as op-clean, but
re-confirm if cheap).

1. Pre-fetch the bge-m3 bundle to your cache dir (use the
   xtask if convenient, or any one-off mechanism — this is
   probe-time only, not part of the deliverable):
   ```sh
   ./scripts/xtask.sh hf-fetch-embed-model -- \
       --model BAAI/bge-m3 \
       --revision <pinned-bge-m3-sha-from-Phase-0> \
       --out target/probe-bge-m3
   ```
2. Write the **smallest possible** scaffolding inside the
   submodule that does **only** this:
   ```rust
   // src/probe.rs (delete after Phase 1) or a one-shot test
   let onnx_bytes = std::fs::read("target/probe-bge-m3/.../model.onnx")?;
   let _model = tract_onnx::onnx().model_for_read(&mut &onnx_bytes[..])?;
   ```
   plus a `Cargo.toml` listing `tract-onnx` only (no other
   deps yet). Run it.
3. **Gate**: if bge-m3 loads cleanly under tract — proceed
   to Phase 2. If tract rejects any op:
   - STOP. Do NOT write the wrapper / pool / inference code.
   - Report the rejected op name in the structured-output
     **Residual risks** section.
   - This reopens Yuka's call B (default model choice). Codex
     does not unilaterally pick a different default — the
     production-default architecture hangs on bge-m3 working,
     so a different default needs Yuka's say-so. Codex
     reports and stops.
4. (Optional) Repeat the probe against the small test model
   to confirm — should pass per the pivot plan.
5. Once Phase 1 passes, delete the probe scaffolding (it
   was a dispatch-time guard, not a deliverable).

### Phase 2 — Wire-shape rewrite

Rewrite `config.rs`, `request.rs`, `response.rs`, `error.rs`,
plus the `lib.rs` rustdoc + public-surface skeleton (no
inference yet). Land the unit tests for those four modules.
`./scripts/pre-landing.sh philharmonic-connector-impl-embed`
green here before moving on.

### Phase 3 — Pool / normalize

Write `pool.rs` with full unit-test coverage. This is pure
math — heavy on hand-computed expected values, no model
needed. Pre-landing green again before moving on.

### Phase 4 — Model + execute

Write `model.rs` and the `Implementation::execute` body in
`lib.rs`. Wire the tokenizer + tract run + pool + normalize
end-to-end. At this point the integration tests under
`tests/` should compile (cfg-gated on the bundle).

### Phase 5 — Integration tests

Write the six integration tests + `tests/common.rs` +
`tests/README.md`. The test cycle uses the small multilingual
model overridden via env vars so `include_bytes!` doesn't
balloon to 2.3GB:

```sh
export PHILHARMONIC_EMBED_DEFAULT_MODEL=sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2
export PHILHARMONIC_EMBED_DEFAULT_REVISION=<pinned-small-model-sha-from-Phase-0>
./scripts/pre-landing.sh philharmonic-connector-impl-embed
```

Pre-landing runs `cargo test` with default features on; with
the override env vars, build.rs fetches+caches the small
model on first run, subsequent runs cache-hit.

Also verify the no-bundle path compiles cleanly (no network,
no `new_default()`):

```sh
unset PHILHARMONIC_EMBED_DEFAULT_MODEL PHILHARMONIC_EMBED_DEFAULT_REVISION
cargo check -p philharmonic-connector-impl-embed --no-default-features
cargo test -p philharmonic-connector-impl-embed --no-default-features --tests
```

### Phase 6 — Final verification

```sh
# Default path (small-model override for sanity)
export PHILHARMONIC_EMBED_DEFAULT_MODEL=sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2
export PHILHARMONIC_EMBED_DEFAULT_REVISION=<as above>
./scripts/pre-landing.sh philharmonic-connector-impl-embed
cargo test -p philharmonic-connector-impl-embed --all-targets

# No-bundle path
unset PHILHARMONIC_EMBED_DEFAULT_MODEL PHILHARMONIC_EMBED_DEFAULT_REVISION
cargo test -p philharmonic-connector-impl-embed --no-default-features --all-targets

# (Optional, expensive) Production-default path — only if you
# have ~3GB+ free disk + bandwidth + patience for the
# include_bytes! compile pass. Skip if it's painful; report
# in residual risks.
unset PHILHARMONIC_EMBED_DEFAULT_MODEL PHILHARMONIC_EMBED_DEFAULT_REVISION
cargo check -p philharmonic-connector-impl-embed   # picks up bge-m3 default

# docs.rs simulation
DOCS_RS=1 cargo check -p philharmonic-connector-impl-embed --no-default-features

git -C philharmonic-connector-impl-embed status --short
git -C . status --short
```

All paths compile/test green. Working tree dirty. Do not
commit.

## Workspace conventions (recap)

- Edition 2024, MSRV 1.88.
- License `Apache-2.0 OR MPL-2.0`.
- `thiserror` for library errors; no `anyhow`.
- **No panics in library `src/`** (CONTRIBUTING.md §10.3).
  No `.unwrap()` / `.expect()` / `panic!` / `unreachable!`
  / `todo!` / `unimplemented!` on reachable paths, no
  unbounded indexing, no unchecked integer arithmetic, no
  lossy `as` casts on untrusted widths. Tests / dev-deps
  are exempt; `build.rs` is exempt (build-script context).
- **Library takes bytes, not file paths** (§10.4). Runtime
  code in `src/` does not read files / env vars / network.
  build.rs IS allowed to read env vars (it's bin-territory).
- **No `unsafe`** in `src/`.
- **Rustdoc on every `pub` item.**
- Re-export the impl-api public surface so consumers depend
  on just this crate (`Implementation`, `ImplementationError`,
  `ConnectorCallContext`, `JsonValue`, `async_trait`).
- Use `./scripts/*.sh` wrappers (not raw cargo) — but this
  prompt explicitly authorizes `cargo test` invocations
  with the bundle env var, since pre-landing doesn't carry
  that env-var plumbing.

## Pre-landing

Two paths must pass:

```sh
# Bundled path (small-model override for fast iteration)
PHILHARMONIC_EMBED_DEFAULT_MODEL=sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2 \
PHILHARMONIC_EMBED_DEFAULT_REVISION=<pinned-sha> \
    ./scripts/pre-landing.sh philharmonic-connector-impl-embed

# No-bundle path
cargo test -p philharmonic-connector-impl-embed --no-default-features --all-targets
```

The integration tests are `#[cfg(...)]`-gated, so the
no-bundle path runs unit tests only and the bundled path
runs unit + integration.

## Git

You do NOT commit, push, branch, tag, or publish. Leave
the working tree dirty in the submodule (and in the parent
if any submodule pointer would change). Claude commits via
`./scripts/commit-all.sh` and publishes via
`./scripts/publish-crate.sh` post-review.

Read-only git is fine (`log`, `diff`, `show`, `status`).

## Deliverables

1. Updated `Cargo.toml`: tract / tokenizers / ndarray runtime
   deps; fastembed / ort removed; `[features]` block with
   default-on `bundled-default-model`; `[build-dependencies]`
   for `ureq`+rustls + `serde_json` + `sha2` + `dirs`;
   `[package.metadata.docs.rs] no-default-features = true`;
   versions verified via `crates-io-versions`.
2. `build.rs` implementing the bundled-default-model fetch
   per §"Default-bundled model architecture" — feature-gated
   short-circuit, `DOCS_RS` auto-skip, env-var knobs, cache
   hit/miss with SHA256 manifest verification, cargo
   directives (`rustc-cfg=embed_default_bundle`,
   `rustc-env=EMBED_DEFAULT_*`, `rerun-if-env-changed=...`,
   `rerun-if-changed=$bundle/manifest.json`), and a baked-in
   bge-m3 default revision SHA.
3. `src/{lib,config,request,response,model,pool,error}.rs`
   — clean rewrite, all seven modules (six per pivot plan
   plus `pool.rs` which the plan adds vs. round-01). `lib.rs`
   has both `new_from_bytes` and the cfg-gated `new_default`.
4. `tests/{batch_inference,batch_size_enforcement,common,
   deterministic_output,inference_produces_correct_shape,
   l2_normalized,semantic_similarity}.rs` + `tests/README.md`
   — clean rewrite, six tests + helper module, all
   `#[cfg(all(feature = "bundled-default-model",
   embed_default_bundle))]`-gated.
5. `CHANGELOG.md` — `[0.1.0] - 2026-04-27` entry replacing
   the round-01 fastembed entry.
6. `README.md` — refreshed for tract stack + dual-constructor
   architecture + feature flag.
7. Crate-root rustdoc on `src/lib.rs` matching the density
   of the recently-published Tier 1 sibling crates.

Working tree: dirty. Do not commit.

## Structured output contract

1. **Summary** (3–6 sentences). Include: did the bge-m3
   op-coverage probe pass? Which small-model + HF revision
   SHA was used for test cycles? Any tract op that was
   almost-but-not-quite a problem on bge-m3?
2. **Files touched** — list every file added / modified /
   deleted.
3. **Verification results** — output of:
   - bundled-path pre-landing (with small-model override),
   - `--no-default-features` test pass,
   - (optional) bge-m3 production-default `cargo check`,
   - `DOCS_RS=1 --no-default-features` simulation.
   For each, test counts and whether the cfg-gated
   integration tests ran. Plus: **bge-m3 model id + pinned
   revision SHA + ONNX file size + tokenizer.json size**;
   **small test model id + revision SHA + ONNX size +
   dimensions + max_seq_length**; estimated binary-size
   delta with each default model bundled (or note "skipped
   bge-m3 build for time/disk; estimated XGB").
4. **Residual risks / TODOs** — anything that didn't fit
   the round, anything that needs a Yuka call, any tract
   op known-fine-on-bge-m3-but-watch-it-on-others, any
   bge-m3 ONNX-path quirk, build-time pain points worth
   flagging.
5. **Git state** — `git -C philharmonic-connector-impl-embed
   status --short` and `git -C . status --short`.
6. **Dep versions used** — exact runtime deps (`tract-onnx`,
   `tokenizers`, `ndarray`, `tokio`,
   `philharmonic-connector-impl-api`,
   `philharmonic-connector-common`), build deps (`ureq`,
   `sha2`, `dirs`, `serde_json`), dev deps (`approx`),
   plus any transitive surprising pins. Noted whether each
   passed the 3-day cooldown when checked.

## Default follow-through policy

- Carry through to pre-landing-green and bundled-tests-green
  before returning. Do not return red.
- If pre-landing fails: fix and re-run.
- If a phase-gate (Phase 1 op-coverage especially) fails:
  STOP at that phase and report — do not press past a
  failing gate.

## Completeness contract

- Every module in the §"In scope" §"Module layout" list
  exists with its specified content (including `build.rs`
  and `pool.rs`).
- Every test in the §"Integration tests" list exists, is
  `#[cfg(all(feature = "bundled-default-model",
  embed_default_bundle))]`-gated, and runs green under the
  bundled small-model test pass.
- The `--no-default-features` test pass also runs green
  (unit tests only).
- `Cargo.toml` has no fastembed / ort residue, has the
  feature block, has the build-deps block, and has the
  docs.rs metadata.
- `CHANGELOG.md` has a single `[0.1.0] - 2026-04-27` entry
  reflecting the tract stack + bundled-default-model
  feature.
- Crate-root rustdoc on `src/lib.rs` is non-empty and
  describes both constructor paths + the feature.

## Verification loop

```sh
# Phase 0 — cooldown + revision pinning
./scripts/xtask.sh crates-io-versions -- tract-onnx
./scripts/xtask.sh crates-io-versions -- tokenizers
./scripts/xtask.sh crates-io-versions -- ndarray
./scripts/xtask.sh crates-io-versions -- philharmonic-connector-impl-api
./scripts/xtask.sh crates-io-versions -- philharmonic-connector-common
./scripts/xtask.sh crates-io-versions -- ureq
./scripts/xtask.sh crates-io-versions -- sha2
./scripts/xtask.sh crates-io-versions -- dirs
./scripts/xtask.sh crates-io-versions -- serde_json
./scripts/xtask.sh crates-io-versions -- approx
# Pin bge-m3 default revision SHA (bake into build.rs)
# Pin small test model + revision SHA (use as override env vars)

# Phase 1 — bge-m3 op-coverage probe
./scripts/xtask.sh hf-fetch-embed-model -- \
    --model BAAI/bge-m3 \
    --revision <pinned-bge-m3-sha> \
    --out target/probe-bge-m3
# Tract probe: model_for_read against bge-m3 ONNX. STOP if any op rejected.

# Phase 6 — final, three paths
# (1) Bundled small-model pass (the workhorse for iteration)
export PHILHARMONIC_EMBED_DEFAULT_MODEL=sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2
export PHILHARMONIC_EMBED_DEFAULT_REVISION=<pinned-small-sha>
./scripts/pre-landing.sh philharmonic-connector-impl-embed
cargo test -p philharmonic-connector-impl-embed --all-targets

# (2) No-bundle pass
unset PHILHARMONIC_EMBED_DEFAULT_MODEL PHILHARMONIC_EMBED_DEFAULT_REVISION
cargo test -p philharmonic-connector-impl-embed --no-default-features --all-targets

# (3) docs.rs simulation
DOCS_RS=1 cargo check -p philharmonic-connector-impl-embed --no-default-features

# Optional, time/disk permitting: bge-m3 default check
unset DOCS_RS
cargo check -p philharmonic-connector-impl-embed   # picks up baked-in bge-m3 default

git -C philharmonic-connector-impl-embed status --short
git -C . status --short
```

## Missing-context gating

- If the impl-api or impl-common surface differs from the
  expectations encoded in the pivot plan + round-01 prompt:
  STOP and flag.
- If `tract-onnx` 0.22+ has materially different API surface
  than the pivot plan assumes
  (`tract_onnx::onnx().model_for_read(...)` is the canonical
  load entrypoint per the plan): adapt within the
  pure-Rust-no-network constraint; if adaptation would
  require a different library (tract-tflite, candle, etc.),
  STOP and flag.
- If the `tokenizers` crate's `Tokenizer::from_bytes` /
  `with_padding` / `with_truncation` API has shifted:
  adapt; if it's gone, STOP and flag.
- **bge-m3 specifically**:
  - If bge-m3's HF repo has no usable ONNX export at any
    standard or near-standard path (e.g. `onnx/model.onnx`,
    `onnx_int8/model.onnx`, root `model.onnx`): STOP and
    flag — Yuka's call B presupposes a usable ONNX. Codex
    does not unilaterally pick a different default.
  - If bge-m3's tokenizer is not in the `tokenizer.json`
    single-file format the architecture assumes: STOP and
    flag.
  - If tract rejects any op in the bge-m3 ONNX (Phase 1
    probe): STOP per Phase 1's stop rule; do not press
    past.
- If `paraphrase-multilingual-MiniLM-L12-v2` (or whichever
  small model you pick) has been yanked / restructured: pick
  another small multilingual with a usable ONNX +
  `tokenizer.json` and document the choice. This one is a
  test-cycle convenience, not architecturally binding —
  Codex CAN pick a different one without a Yuka call.
- If a dep is yanked or fails the cooldown: pin to the prior
  version and note.
- If `ureq`'s rustls feature gates have shifted in a way
  that requires a different feature combination: adapt and
  document. The constraint is "ureq + rustls, no native-tls,
  no OpenSSL".
- If any other architecturally-significant surprise: STOP
  and flag.

## Action safety

- No `cargo publish`, no `git push`, no branch creation,
  no tags.
- No edits outside `philharmonic-connector-impl-embed/`
  except `Cargo.lock` regeneration, which is fine and
  expected.
- No destructive ops (`rm -rf` outside `target/`, force
  pushes, history rewrites — none of which you'd touch
  anyway given "no commits / pushes" above, but stated for
  completeness).
- `target/probe-bge-m3/` and the build.rs cache root
  (default `$XDG_CACHE_HOME/philharmonic/embed-bundles/`
  with `$HOME/.cache/...` fallback, override
  `PHILHARMONIC_EMBED_CACHE_DIR`) are fine to write. Both
  hold multi-hundred-MB-to-multi-GB model bytes; the cache
  root is outside the workspace tree → automatically
  excluded from git. `target/probe-bge-m3/` is gitignored
  via the workspace's blanket `target/` rule. Do not
  commit either (and remove the probe scaffolding under
  `src/probe.rs` after Phase 1 — it's a dispatch-time
  guard, not a deliverable).

---

## Outcome

Round 02 — **stopped at the Phase 1 gate** as instructed.
Codex session `019dcd66-2f22-7182-9490-0e5303ead606`, ran
2026-04-27 14:24→14:28 JST (3m 46s).

### What happened

bge-m3's ONNX export at HF revision
`5617a9f61b028005a4858fdac845db406aefb181` is **not a single
self-contained byte blob**: the on-disk format is
`onnx/model.onnx` (~724KB metadata) plus
`onnx/model.onnx_data` (~2.27GB external-weights file). The
prompt's "single `include_bytes!(model.onnx)`" architecture
cannot resolve the external-data reference because
`tract_onnx::onnx().model_for_read(&mut &bytes[..])` has no
path context for the sibling file.

Codex's probe failed *before* reaching tract op-coverage
verification — the model didn't even load. Error: `no model
path was specified in the parsing context, yet external data
was detected. aborting`.

Codex stopped per the Phase 1 stop rule and the prompt's
instruction "Codex does not unilaterally pick a different
default — the production-default architecture hangs on bge-m3
working, so a different default needs Yuka's say-so." This
reopens Yuka's call B.

### Bundle metadata measured

| File | bge-m3 | paraphrase-multilingual-MiniLM-L12-v2 |
|---|---|---|
| `onnx/model.onnx` | 724,923 B | 470,301,610 B |
| `onnx/model.onnx_data` | 2,266,820,608 B | (n/a, single file) |
| `tokenizer.json` (root) | 17,098,108 B | 9,081,518 B |
| `hidden_size` | 1024 | 384 |
| `max_position_embeddings` | 8194 | 512 |

bge-m3 also has an `onnx/tokenizer.json` variant (17,082,821 B,
slightly different from the root copy).

### Dep cooldown findings

- `philharmonic-connector-impl-api 0.1.0` flagged
  `2d23h old (< 3d threshold)`. This is the same version
  the wave-1 crates published against today; the 3-day
  threshold clears within hours of the Codex run.
- `tract-onnx 0.22.1` pulls `ndarray 0.16.1` transitively;
  workspace `crates-io-versions` direct lookup reported
  `0.17.2` as latest. The pivot plan already pinned `0.16`,
  so this matches expectation — the direct dep must pin
  `0.16` to share the type universe with tract.
- All other deps pass cooldown: `tract-onnx 0.22.1`,
  `tokenizers 0.22.2`, `tokio 1.52.1`,
  `philharmonic-connector-common 0.2.0`, `ureq 3.3.0`,
  `sha2 0.11.0`, `dirs 6.0.0`, `serde_json 1.0.149`,
  `approx 0.5.1`.

### Small test model pinned

If round-03 keeps the small-model override env vars,
Codex pinned
`sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2`
at revision `e8f8c211226b894fcb81acc59f3b34ba3efd5f42`.

### Git state at stop

Workspace + submodule both clean. No tracked files were
modified — Codex stopped before any rewrite work.

### Files written outside the working tree (now stale)

- `target/probe-bge-m3-model.onnx`
- `/tmp/probe-tract/*`
- `/tmp/bge-*.json`, `/tmp/minilm*.json`

These are scratch artifacts; the workspace `.gitignore`
covers `target/` and `/tmp` is ephemeral. Safe to leave
or `rm -rf`.

### Decision needed (Yuka call — reopens B)

The default-bundled-model architecture as specified does
not work for bge-m3 directly. Three resolution paths, all
requiring a Yuka call before round 03:

1. **Pick a different default** with a self-contained ONNX
   export. `paraphrase-multilingual-MiniLM-L12-v2` (~470MB
   single-file, 384-dim, the small test model Codex
   already pinned) is the obvious fallback;
   `intfloat/multilingual-e5-large` (~2.2GB, 1024-dim) and
   other multilingual options may also be self-contained
   — needs verification. Smallest architectural change;
   loses bge-m3's quality.
2. **Extend the public API to accept external-data bytes**:
   `new_from_bytes(model_id, onnx_bytes, onnx_external_data:
   Option<&[u8]>, tokenizer_json_bytes, dimensions,
   max_seq_length)`. The default-bundled path `include_bytes!`s
   both files. `xtask hf-fetch-embed-model` gets a small
   extension to fetch `model.onnx_data` when present.
   Bytes-only contract preserved; bge-m3 stays as default.
3. **Asymmetric load**: `new_from_bytes` stays bytes-only
   (single-file ONNX only, errors loudly on external-data);
   `new_default` internally uses tract's path-based load
   from the build.rs cache dir. Mixed contract; ugly but
   avoids public-API churn.

**Claude's recommendation**: Option 2. Cleanest
representation of what ONNX-with-external-data actually is,
keeps the bytes-only architecture intact, no change of
default model. The xtask extension is ~10 lines.

A round-03 prompt would adjust §"Default-bundled model
architecture" → §"Public surface" + §"build.rs HTTP stack"
+ §"Cache layout" to add the external-data file, and a
small follow-up commit extends the xtask.
