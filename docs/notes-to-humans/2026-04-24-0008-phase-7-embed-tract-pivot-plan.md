# Phase 7 Tier 1 — `embed` pivot: fastembed → tract

**Author**: Claude Code
**Date**: 2026-04-24 (金)
**Audience**: Yuka (decision already made: go tract) + future
Codex dispatch against this plan.
**Status**: **plan locked; embed 0.1.0 NOT published today**.
The current submodule contains Codex's fastembed-based round-01
work committed as a checkpoint; it's replaced — not extended —
by a tract-based rewrite in a subsequent Codex dispatch.
**Crate**:
[`philharmonic-connector-impl-embed`](https://github.com/metastable-void/philharmonic-connector-impl-embed)

## Why pivot

Round-01 of this crate shipped the fastembed + ort v2
(`ort-download-binaries` feature) stack — glibc-x86_64 only
(plus a handful of other tier-1 targets pykeio publishes
prebuilts for). musl / Alpine / `x86_64-unknown-linux-musl`
targets hit a link-time failure because the ort runtime
library shipped by the feature is glibc-linked.

Yuka is keeping musl support as a v1 baseline. That rules
out the "download pykeio's prebuilt libonnxruntime" path.
Two architecturally-cleaner options existed:

- `ort/load-dynamic` — dlopen a caller-supplied
  `libonnxruntime.so`; deployment ships three artifacts
  (binary + weights + runtime library).
- Pure-Rust inference stack — **`tract`** (Sonos,
  https://github.com/sonos/tract). No C runtime, musl-
  native, ONNX-model support.

Yuka picked `tract`. It's the most architecturally consistent
with the workspace's "no runtime surprises" disposition and
keeps the deployment artifact count at two (binary with the
weights embedded + nothing else).

## Scope of this pivot

**Not a Cargo feature flip; a small but real rewrite.** The
fastembed-based code in the submodule does not survive
unchanged — it's replaced module-by-module. Round-01's src/
layout stays recognizable; the pipeline inside each module
is different.

## Target stack (dep set)

```toml
[dependencies]
async-trait = "0.1"
philharmonic-connector-impl-api = "0.1"
philharmonic-connector-common = "0.2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
tokio = { version = "1", features = ["rt", "macros", "time"] }
# Inference stack (pure Rust, musl-native):
tract-onnx = "0.22"          # ONNX execution (via tract_hir → tract_core). Verify latest via xtask.
tokenizers = { version = "0.22", default-features = false, features = ["onig"] }
# `tokenizers` core. `default-features = false` drops `http` / `hf-hub` fetching (again, no runtime HF).
# `onig` for regex-backed pre-tokenizers (BERT-class tokenizers depend on it).
ndarray = "0.16"              # tensor construction (tract's input/output type is ndarray-flavored)
```

No `fastembed`, no `ort`, no `ort-sys`, no `rayon` transitive
blowup. `tract-onnx` + `tokenizers` + `ndarray` cover the
pipeline end-to-end.

### Version cooldown

Verify each of `tract-onnx`, `tokenizers`, `ndarray` passes
the 3-day cooldown via
`./scripts/xtask.sh crates-io-versions -- <crate>` before
committing Cargo.toml.

## Target module layout

Same set of files as round-01, different insides:

```
src/
├── lib.rs           # crate rustdoc + public Embed type + Embed::new_from_bytes + trait impl + re-exports
├── config.rs        # EmbedConfig (unchanged — model_id/max_batch_size/timeout_ms)
├── request.rs       # EmbedRequest (unchanged)
├── response.rs      # EmbedResponse (unchanged; still f32, still model-echo, still dimensions)
├── model.rs         # NEW SHAPE — wraps the tract SimplePlan + tokenizers Tokenizer + metadata
├── pool.rs          # NEW — mean-pool across sequence length with attention-mask weighting + L2-normalize
└── error.rs         # internal Error + From<Error> for ImplementationError (unchanged)
```

Wire protocol (`EmbedConfig`, `EmbedRequest`, `EmbedResponse`)
is **unchanged** — doc 08's shape + the spec's decisions
carry over verbatim. The implementation underneath the public
surface is what gets rewritten.

## Public surface (unchanged from round-01's spec)

```rust
pub struct Embed { /* tract SimplePlan + tokenizer + model_id + dimensions + max_seq_length */ }

impl Embed {
    /// Eagerly loads the ONNX model into tract + parses the
    /// tokenizer JSON bytes. Fails fast on malformed input.
    /// `dimensions` and `max_seq_length` come from the
    /// caller (deployment's binary, which knows the model
    /// it's bundling).
    pub fn new_from_bytes(
        model_id: impl Into<String>,
        onnx_bytes: Vec<u8>,
        tokenizer_json_bytes: &[u8],
        dimensions: usize,
        max_seq_length: usize,
    ) -> Result<Self, ImplementationError>;

    pub fn model_id(&self) -> &str;
    pub fn dimensions(&self) -> usize;
}

#[async_trait]
impl Implementation for Embed { ... }
```

Signature difference from round-01: fastembed's
`TokenizerFiles` struct had four byte-slices
(tokenizer/tokenizer_config/config/special_tokens_map). The
`tokenizers` crate loads everything from one `tokenizer.json`
(the HF "combined" tokenizer format). So `new_from_bytes`
now takes a single `tokenizer_json_bytes: &[u8]` instead of
four.

This is a real public-API change relative to round-01. It
**reduces** surface (4 fields → 1) and matches the HF
tokenizer-format convention. The `hf-fetch-embed-model` xtask
(already shipped at `8b4c88f`) already fetches
`tokenizer.json` as one of the five files it pulls, so the
build-time bundle story is unchanged — deployments just don't
need to plumb the other three tokenizer files into
`Embed::new_from_bytes` anymore.

The xtask itself still fetches all five files (the extra
three are cheap; `config.json` is also useful metadata; no
reason to narrow it).

## Inference pipeline

`execute(config, request, ctx)`:

1. Deserialize config → `EmbedConfig`. `InvalidConfig` on
   failure. Validate `config.model_id == self.model_id`.
2. Deserialize request → `EmbedRequest`. `InvalidRequest` on
   failure. Validate `request.texts.len() >= 1` and `<=
   config.max_batch_size`.
3. `tokio::task::spawn_blocking(...)` the rest (CPU-bound);
   wrap in `tokio::time::timeout(config.timeout_ms, ...)`.
4. **Tokenize batch**:
   `tokenizer.encode_batch(texts, true)` →
   `Vec<Encoding>`. Each encoding has `ids`,
   `attention_mask`, optionally `type_ids`. Pad/truncate to
   `max_seq_length` (tokenizers does this when configured
   via the tokenizer.json's padding/truncation rules, OR
   we call `tokenizer.with_padding(...)` /
   `tokenizer.with_truncation(...)` at construction time).
5. **Build input tensors**:
   ```rust
   let batch = encodings.len();
   let seq  = max_seq_length;
   let input_ids = Array2::<i64>::from_shape_fn((batch, seq), |(i, j)| encodings[i].get_ids()[j] as i64);
   let attention_mask = Array2::<i64>::from_shape_fn((batch, seq), |(i, j)| encodings[i].get_attention_mask()[j] as i64);
   // token_type_ids if the model needs it (BERT does; paraphrase-multilingual-MiniLM-L12-v2 does too).
   ```
6. **Run the tract model**:
   ```rust
   let inputs = tvec!(input_ids.into(), attention_mask.into()[, token_type_ids.into()]);
   let outputs = self.plan.run(inputs)?;
   let last_hidden_state: Array3<f32> = outputs[0].to_array_view::<f32>()?.into_dimensionality()?.to_owned();
   // shape: (batch, seq, hidden)
   ```
7. **Mean-pool with attention mask** (`pool.rs`):
   ```rust
   // For each (batch) row:
   //   sum[b][d] = Σ_j last_hidden_state[b][j][d] * attention_mask[b][j]
   //   count[b]  = Σ_j attention_mask[b][j]
   //   pooled[b][d] = sum[b][d] / count[b]  (safe-divide on zero)
   // Then L2-normalize pooled[b] so ||pooled[b]|| = 1.
   ```
   This is the standard sentence-transformers pooling;
   matches what fastembed does internally.
8. **Serialize response**:
   `EmbedResponse { embeddings: Vec<Vec<f32>>, model:
   self.model_id.clone(), dimensions: self.dimensions }`.
9. `serde_json::to_value` → return.

### Which model-op inventory?

`paraphrase-multilingual-MiniLM-L12-v2` (canonical Yuka-
indicated target) is a BERT-class Sentence-Transformer. The
ops used: Gather, MatMul, Add, Mul, Softmax, LayerNorm,
Erf, Reshape, Transpose, Unsqueeze — all standard ONNX ops
that tract supports in 0.22.

Verify during Codex dispatch: load the ONNX with
`tract_onnx::onnx().model_for_read(...)`; if any op rejects,
flag and stop — would reopen the library-choice question.

### Pooling choice

Mean pooling with attention-mask weighting is the
sentence-transformers default and matches fastembed's output
semantics. CLS pooling is available behind the same weights
but produces different vectors — don't use. Our goal is to
produce vectors compatible with whatever the deployment's
downstream (`vector_search` et al.) expects.

## Error cases (unchanged from round-01)

- `InvalidConfig` — config/model_id mismatch.
- `InvalidRequest` — empty texts, overrun `max_batch_size`.
- `UpstreamTimeout` — inference exceeded `timeout_ms`.
- `Internal` — tract op rejection, tokenizer failure,
  tensor-shape mismatch.

No `UpstreamError`, no `UpstreamUnreachable`.

## Testing (unchanged from round-01)

Env-var-gated live-inference tests:

- `EMBED_TEST_ONNX_PATH`
- `EMBED_TEST_TOKENIZER_JSON_PATH` (new — path to a
  `tokenizer.json` file; replaces `EMBED_TEST_TOKENIZER_DIR`
  since we only need the one file)
- `EMBED_TEST_MODEL_ID`
- `EMBED_TEST_DIMENSIONS`
- `EMBED_TEST_MAX_SEQ_LENGTH`

Unit tests always run; live-inference tests `#[ignore]`-gated.
Heavy coverage on `pool.rs` (mean-pool + L2-norm is pure
math with known expected values).

## Build size / compile time

- fastembed + ort total build: ~2m 13s cold, brings in ~100
  transitive crates, drags in `rayon`, `rav1e`-style
  transitives through ort's image-path (all dead for us but
  present).
- tract + tokenizers + ndarray estimated: faster compile,
  fewer transitives, no image/audio crates in the tree.

A real number is empirical — Codex verifies during
dispatch.

## Binary size

- fastembed + ort runtime: ~30MB `libonnxruntime.so` +
  ~10MB ort wrapper = 40MB of library in the binary (or
  loaded dynamically).
- tract pure-Rust: all inference code statically linked,
  expected footprint ~5–15MB for the operator implementations
  (tract-core + tract-onnx).

Net savings: probably ~25MB per deployment binary, AND
musl compatibility. Both directions win.

## What the Codex round-02 prompt should say

When Claude drafts the round-02 dispatch prompt (next
session), it should:

1. Link to this plan doc as authoritative.
2. Instruct Codex to **replace** all of `src/*.rs` in the
   submodule — keep the file layout, rewrite the innards.
   The fastembed-based round-01 code in the working tree is
   a reference for the wire-protocol shape that stays, not
   the impl pipeline that goes.
3. Drop `fastembed`, `ort`, `ort-sys` from Cargo.toml; add
   `tract-onnx`, `tokenizers`, `ndarray`.
4. Change `Embed::new_from_bytes` signature from the 4-file
   tokenizer bundle to single `tokenizer_json_bytes: &[u8]`.
5. All live-inference tests update their env-var set
   (`EMBED_TEST_TOKENIZER_JSON_PATH` replaces
   `EMBED_TEST_TOKENIZER_DIR`).
6. Verify on a real model (optionally), report inference
   latency + tensor shapes.
7. Pre-landing green before return.

## Workspace-level follow-ups

- **`hf-fetch-embed-model` xtask is already correct**. It
  fetches `tokenizer.json` among the five files; the other
  four stay as metadata that deployment tooling may want
  but the embed crate no longer requires.
- **ROADMAP Phase 7 Tier 1 marker**: three of four done
  today (sql-postgres, sql-mysql, vector-search); embed
  pending tract rewrite. Not a blocker for the Phase 7 Tier
  2 (SMTP) or Phase 8 roll-up.
- **CONTRIBUTING.md §10.x** — document the "no C runtime
  dependency in impl crates" discipline implicitly set by
  this pivot. Follow-up.

## Risks / residuals

1. **Unsupported ONNX op**. If the target model uses an op
   `tract 0.22` doesn't implement, we'd need a workaround
   (manual op substitution, op backfill PR upstream, or
   pick a different target model). Probability: low for
   BERT-class models; flag explicitly during Codex dispatch.

2. **Tokenizer padding/truncation mismatch**. The
   `tokenizers` crate requires explicit padding config; the
   `tokenizer.json` usually encodes this, but some model
   repos are missing the padding section. Handle explicitly
   (set padding from the model's known `max_seq_length`
   at `Embed::new_from_bytes`).

3. **`type_ids` / `token_type_ids` input** not always
   present. Some ONNX exports drop it; some require it.
   Detect from the loaded model's input-signature and build
   the tvec accordingly.

4. **Inference determinism** vs fastembed's. Not guaranteed
   byte-identical (different runtime, potentially different
   operator implementations). Semantic similarity still
   holds — the plan's `semantic_similarity` test exercises
   that, not byte-identity.

---

Next step: in the session that picks this up, draft
`docs/codex-prompts/YYYY-MM-DD-NNNN-phase-7-embed-tract.md`
referencing this plan as authoritative + the committed
round-01 checkpoint as "wire-shape reference, not impl
reference". Dispatch against the same crate submodule; land
0.1.0 when green.
