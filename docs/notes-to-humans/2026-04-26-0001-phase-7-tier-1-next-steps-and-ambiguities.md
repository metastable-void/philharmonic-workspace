# Phase 7 Tier 1 — next steps and ambiguities

**Author:** Claude Code · **Audience:** Yuka · **Date:** 2026-04-26 (Sun) JST

This is a checkpoint as of the parent push at `f9089da`. It
captures what the plan looks like after Tier 1's three-of-four
landed locally and surfaces the decisions still hanging that
need a human call before the next substantial Codex dispatch.

---

## State (one-screen recap)

- **Phase 6** — done, all three crates published 2026-04-24
  (`impl-api`, `http-forward`, `llm-openai-compat`).
- **Phase 7 Tier 1** (data-layer connectors) — three of four
  locally ready at 0.1.0, none published yet:
  - `philharmonic-connector-impl-sql-postgres` — green.
  - `philharmonic-connector-impl-sql-mysql` — green.
  - `philharmonic-connector-impl-vector-search` — green
    (34 tests, no external deps).
  - `philharmonic-connector-impl-embed` — round-01
    (`fastembed` + `ort`) committed as a checkpoint
    only. Yuka rejected the ort-download-binaries
    library choice because the deployment targets
    include musl and the prebuilt ORT shared library is
    glibc-only. Pivot plan to pure-Rust `tract` +
    `tokenizers` is at
    [`2026-04-24-0008-phase-7-embed-tract-pivot-plan.md`](2026-04-24-0008-phase-7-embed-tract-pivot-plan.md).
- **Doc reconciliation since the push**:
  README "Status", `01-project-overview.md`,
  `03-crates-and-ownership.md`, `08-connector-architecture.md`,
  `14-open-questions.md` — refreshed on 2026-04-26 to reflect
  the Tier 1 WIP state and to mark `sql_query`,
  `embed` wire shape, and `vector_search` wire shape as
  settled. Only `email_send` (Tier 2) wire shape remains
  open.

---

## Next steps (recommended order)

The macro sequence Yuka committed to earlier in this branch was:

> Phase 7 Tier 1 → Phase 8 → Phase 9 prototype → anything other.

That means Tier 2 (SMTP) and Tier 3 (Anthropic, Gemini, deferred
to on/after 2026-05-07 post-Golden-Week) come **after** Phase 8
(`philharmonic-api`) and the Phase 9 prototype, not in between.

Concrete next units of work, in order:

1. **Draft the embed tract Codex prompt.** Path
   `docs/codex-prompts/2026-04-26-NNNN-phase-7-embed-tract.md`
   (or `2026-04-2{7,8}-...` if drafted after midnight JST). The
   prompt must point at
   [`2026-04-24-0008-phase-7-embed-tract-pivot-plan.md`](2026-04-24-0008-phase-7-embed-tract-pivot-plan.md)
   as the authoritative spec — do not restate the construction
   in the prompt body. Archive + commit the prompt **before**
   spawning Codex (per the `codex-prompt-archive` skill).
2. **Spawn Codex** for the embed rewrite. Single round if it
   lands clean; otherwise round-02 / round-03 iterations as
   usual.
3. **Review the rewrite.** Verify musl build (cross-compile
   target check at minimum), tract-supported op set holds
   for the chosen model architecture, output-shape +
   invariant assertions in tests rather than bit-exact value
   equality vs. round-01.
4. **Publish all four Tier 1 crates** as 0.1.0 to crates.io —
   `sql-postgres`, `sql-mysql`, `vector-search`, `embed` —
   in one publish batch (or sequenced if the dep-graph
   forces sequencing; none of these depend on each other so
   one batch should be fine).
5. **Mark Tier 1 done** in `ROADMAP.md` Phase 7 section.
6. **Phase 8** — `philharmonic-api`. Reference:
   `docs/design/10-api-layer.md`.
7. **Phase 9 prototype.** Reference: `ROADMAP.md`.
8. **Tier 2** (`email-smtp`) and **Tier 3** (`llm-anthropic`,
   `llm-gemini`) afterwards, per the macro sequence.

---

## Ambiguities that need a Yuka call

These are the questions where I do **not** want to pick
unilaterally because the wrong choice burns either time or
publish-quality. Listed in the order I'd want answers if
asked, but each is independent.

### A. Publish cadence — co-land Tier 1, or split?

The current `ROADMAP.md` line says Tier 1 publishes as a
coherent set, which is what I've been operating under. But
if the tract pivot drags (op-coverage surprise, tokenizer
shimming, etc.), the question is whether to split publish
into two waves:

- **Wave 1 now**: `sql-postgres` 0.1.0, `sql-mysql` 0.1.0,
  `vector-search` 0.1.0 — three crates, none depend on each
  other, none depend on `embed`.
- **Wave 2 when ready**: `embed` 0.1.0 alone.

Tradeoff: cleanest "Tier 1 = one announcement" framing is the
co-land. But the co-land path means three crates that were
ready 2026-04-24 don't publish until embed is too, and the
crates.io presence stays at 0.0.0 placeholders.

**Recommendation if asked**: hold for the co-land if tract
lands within ~1 week; split if it slips past Golden Week.

### B. Embed reference model for the in-tree test fixture

The deployment-build-time `xtask hf-fetch-embed-model` tool
is the supported way for a deployment to bundle a model. But
the embed crate's own test suite needs *some* model bytes
to run inference against. Options:

1. **Bundle a tiny model** (e.g. `sentence-transformers/all-MiniLM-L6-v2`,
   ~22MB ONNX) directly into the crate's `tests/fixtures/`
   via Git LFS or as part of the source tarball. Adds bulk
   to the crate.
2. **Run `hf-fetch-embed-model`** as a `build.rs`-time step
   in tests, gated on a dev-only feature flag. Requires
   network at test time.
3. **CI-only fetch + cache**, no fixture in the published
   tarball. The crate's own published test suite would skip
   the inference tests in the absence of a fetched model.
   `cargo publish --dry-run` would still pass.

Round-01's fastembed code took option 1 with the
`include_bytes!` macro pointing at a small model. The plan-
doc keeps the same approach but doesn't pin a specific model.
**Recommendation if asked**: option 1 with
`all-MiniLM-L6-v2` for the test fixture (small, multilingual
weak but fine for shape tests, well-supported by tract).
Production deployments still bundle whatever larger model
they want via the xtask.

### C. Tract op-coverage risk

Per the pivot-plan §Risks, tract may not implement every ONNX
op the chosen model emits. The Sentence Transformers MiniLM
export uses `MatMul`, `Add`, `Mul`, `Softmax`, `LayerNormalization`,
`Gelu`, `Erf`, `Tanh`, `Reshape`, `Transpose`, `Slice`, and
`ReduceMean` — all common, all should be fine. But that's a
hope, not a guarantee.

**Mitigation strategy choice — needs Yuka call**:
1. Have the Codex prompt instruct Codex to **verify op
   coverage early** by attempting load before writing the
   wrapper, and fail fast with a clear message if any op
   is missing (so we discover gaps before Codex burns
   round-02 hours building scaffolding around an unloadable
   model).
2. Or: trust tract's published op list, write the wrapper,
   discover gaps in test runs, iterate.

**Recommendation if asked**: option 1 (verify early). Cost
is one extra step in the Codex prompt; benefit is
short-circuiting if the chosen model isn't compatible.

### D. Codex round shape — incremental vs. clean rewrite

The round-01 fastembed code is committed as a checkpoint and
shares structure with the tract target (5 modules, same
public surface modulo `TokenizerFiles` → single
`tokenizer_json_bytes`). Two prompt shapes:

1. **Clean rewrite** prompt: tell Codex to delete and rewrite,
   referring to the plan doc only.
2. **Incremental migration** prompt: tell Codex to keep the
   existing config/request/response modules, replace
   model.rs / lib.rs, add pool.rs.

**Recommendation if asked**: option 2 (incremental). The
config/request/response layer was already reviewed-OK in
round-01 and is library-agnostic. Less surface for Codex to
re-derive, less risk of accidentally regressing the wire
shape.

### E. Tract test vectors — value-equality vs. invariants

Round-01's tests were shape + invariant tests (length,
L2-normalized within ε, deterministic). Those should
transfer. The question is whether to also commit a
**reference-vector regression test** — given a known input
string, expect a known output vector — to detect silent
drift later.

Generating a reference vector requires *a* deterministic
inference pipeline. Two sources:
1. The first successful tract run itself (lock the output
   we get).
2. A separate Python `sentence-transformers` reference run.

Option 1 is circular (tests verify "tract returns what tract
returned"). Option 2 is the more rigorous test-vector
discipline.

**Recommendation if asked**: option 2 — generate one
reference vector with Python sentence-transformers (or with
the round-01 fastembed code, since both go through ORT and
are reasonably comparable), commit the hex bytes as a
fixture, and assert tract's output is within ε of it. ε must
be loose enough to absorb runtime-implementation float
drift; ~1e-3 cosine distance from the reference is a
reasonable starting point. NB this is **not** crypto-grade
test-vector discipline since `embed` is not a crypto path —
the protocol's test-vector rules apply to crypto only.

### F. Crates.io publish trigger — manual or scripted?

When Tier 1 is ready to ship, the publishes are four
`cargo publish` invocations (in any order, no inter-crate
deps). The mechanical question is whether to add a
`scripts/publish-tier-1.sh` wrapper (wraps `cargo
publish` per crate, post-checks `crates.io-versions`,
emits a summary), or just do it by hand in four steps
through the existing `./scripts/cargo.sh` wrapper.

This is a small thing. **Recommendation if asked**: do it by
hand the first time (gives Yuka a chance to eyeball each
publish), and only abstract a wrapper if Tier 2/3 publishes
make it feel repetitive.

---

## Things I'm explicitly **not** doing

- **Spawning Codex right now.** The tract Codex prompt isn't
  drafted yet, and the ambiguities above (model fixture,
  op-coverage strategy, round shape) inform the prompt's
  content. Asking Codex to fix all of them simultaneously
  would burn iterations.
- **Publishing anything.** All four Tier 1 crates remain
  unpublished. Per Yuka's last explicit direction this
  session: "NOT publishing today."
- **Touching the round-01 fastembed code.** It stays as a
  checkpoint until the tract rewrite lands and the embed
  crate's `Cargo.toml` flips to the tract dep set.
- **Reaching into Tier 2 / Tier 3 work.** Those wait for
  Phase 8 / Phase 9 prototype per the macro sequence.

---

## Where to read more

- Tier 1 narrative + per-crate state:
  [`ROADMAP.md` §Phase 7](../../ROADMAP.md#phase-7--additional-implementations-parallel-safe).
- Embed + vector-search original spec:
  [`2026-04-24-0005-phase-7-tier-1-embed-and-vector-search-spec.md`](2026-04-24-0005-phase-7-tier-1-embed-and-vector-search-spec.md).
- Tract pivot plan:
  [`2026-04-24-0008-phase-7-embed-tract-pivot-plan.md`](2026-04-24-0008-phase-7-embed-tract-pivot-plan.md).
- SQL-postgres / SQL-mysql review notes (other Claude
  session):
  [`2026-04-24-0006-phase-7-sql-postgres-review.md`](2026-04-24-0006-phase-7-sql-postgres-review.md),
  [`2026-04-24-0007-phase-7-sql-mysql-review.md`](2026-04-24-0007-phase-7-sql-mysql-review.md).
