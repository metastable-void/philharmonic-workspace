# Phase 6 Task 2 — `llm_openai_compat` implementation (resume after fixture-location correction)

**Date:** 2026-04-24
**Slug:** `phase-6-llm-openai-compat`
**Round:** 02 (resume after round 01 was halted for a
structural correction; item 5 of the round-01 prompt was
wrong, see below)
**Subagent:** `codex:codex-rescue`

## Motivation

Round 01 of this task ran against
[`2026-04-24-0002-phase-6-llm-openai-compat.md`](./2026-04-24-0002-phase-6-llm-openai-compat.md)
and produced a substantially-complete working tree in the
submodule (config / request / response / client / dialect /
error / retry / schema modules, plus `src/types.rs`, plus
`tests/`) before being halted. See round 01's `## Outcome`
for details of what landed.

Reason for halt: item 5 of round 01's "In scope" told you to
load fixtures via `include_str!("../../../docs/upstream-fixtures/...")`
from `tests/*.rs`. That escapes the submodule boundary, which
(a) breaks standalone clone of the impl repo, (b) fails
`cargo publish --dry-run`'s "file outside package directory"
check, and (c) ships a `.crate` tarball whose tests can't be
built by downstream consumers. Structurally wrong.

This round corrects item 5: **fixtures live inside the
submodule at `tests/fixtures/`**, as real files committed to
the submodule's git tree.

## Delta from round 01

Everything in round 01's prompt still applies **except item 5
of "In scope"**. Re-read round 01 in full for the motivation,
references, crate state, scope (items 1–4, 6–8), out-of-scope
list, decisions-fixed-upstream, workspace conventions, HTTP
client rule, schema validation plan, retry pseudocode,
pre-landing, git discipline, deliverables, structured output
contract, follow-through policy, completeness contract,
verification loop, missing-context gating, and action safety.
If this round-02 prompt contradicts round 01, round 02 wins
because the only thing round 02 changes is the fixture-
location error.

### Replacement for item 5 (fixture location + loading)

Fixtures live **inside the submodule** at
`philharmonic-connector-impl-llm-openai-compat/tests/fixtures/`.
The workspace's `docs/upstream-fixtures/` tree remains the
authoritative provenance-documented source of truth; the
submodule copy is a sync'd duplicate so the crate is
self-contained for standalone build + `cargo publish`.

Required submodule layout:

```
philharmonic-connector-impl-llm-openai-compat/tests/fixtures/
├── README.md                           # write this fresh — contents below
├── vllm/
│   ├── sample_json_schema.json         # byte-exact copy of docs/upstream-fixtures/vllm/sample_json_schema.json
│   └── structured_outputs_json_chat_request.json  # byte-exact copy of docs/upstream-fixtures/vllm/structured_outputs_json_chat_request.json
├── openai-chat/
│   ├── sample_json_schema.json         # byte-exact copy of docs/upstream-fixtures/openai-chat/sample_json_schema.json
│   ├── openai_native/
│   │   ├── request.json                # byte-exact copy of docs/upstream-fixtures/openai-chat/openai_native/request.json
│   │   └── response.json               # byte-exact copy of docs/upstream-fixtures/openai-chat/openai_native/response.json
│   └── tool_call_fallback/
│       ├── request.json                # byte-exact copy of docs/upstream-fixtures/openai-chat/tool_call_fallback/request.json
│       └── response.json               # byte-exact copy of docs/upstream-fixtures/openai-chat/openai_native/response.json
└── vllm_native_response.json           # synthesized — no upstream source for vLLM responses
```

Rules:

- **Byte-exact copies.** `cp` the files verbatim from
  `docs/upstream-fixtures/...` at the workspace root. Do not
  edit the contents. `diff -r` between the two trees must
  show only the new `vllm_native_response.json` you
  synthesize + the new `tests/fixtures/README.md` you write.
- **Do NOT edit or delete the workspace-root
  `docs/upstream-fixtures/` tree.** That is the authoritative
  provenance tree with pinned SHAs / capture-command records;
  the submodule copy is downstream of it.
- **`tests/fixtures/README.md`** (~20–40 lines, write fresh):
  - State this directory is a sync'd copy of
    `<workspace-root>/docs/upstream-fixtures/`.
  - Per-subtree provenance:
    - `vllm/` → byte-exact from upstream vLLM commit
      `cf8a613a87264183058801309868722f9013e101` (see
      workspace `docs/upstream-fixtures/vllm/README.md` for
      pinned blob SHAs).
    - `openai-chat/` → captured 2026-04-24 against the real
      OpenAI API, model `gpt-4o-mini-2024-07-18` (see
      workspace `docs/upstream-fixtures/openai-chat/README.md`
      for the full capture command + re-capture recipe).
    - `vllm_native_response.json` → synthesized to match the
      OpenAI chat-completion response envelope; no upstream
      source because vLLM's committed tests only cover
      requests. Describe the synthesis rationale in one
      sentence.
  - Note that standalone impl-repo CI + `cargo publish` both
    rely on these copies being inside the submodule.

- **Loading in tests**: `include_str!` is relative to the
  file containing the macro, so from `tests/<any>.rs`:

  ```rust
  const OPENAI_NATIVE_REQUEST: &str =
      include_str!("fixtures/openai-chat/openai_native/request.json");

  const VLLM_NATIVE_REQUEST: &str =
      include_str!("fixtures/vllm/structured_outputs_json_chat_request.json");

  const VLLM_NATIVE_RESPONSE: &str =
      include_str!("fixtures/vllm_native_response.json");

  // etc.
  ```

  No `..` hops. No workspace-escape paths.

- **Test byte-assertions** (round 01's item 4 sub-bullets on
  `dialect_openai_native.rs` / `dialect_vllm_native.rs` /
  `dialect_tool_call_fallback.rs`) unchanged except for the
  path base: load from the submodule's `tests/fixtures/`,
  not from `docs/upstream-fixtures/`.

- **Publishability check**: as part of the verification loop
  (below), confirm the crate packages cleanly. Run
  `cargo package --list -p philharmonic-connector-impl-llm-openai-compat`
  and sanity-check that all `tests/fixtures/*.json` files
  appear in the listing. If any fixture is missing from the
  package listing, investigate
  `philharmonic-connector-impl-llm-openai-compat/Cargo.toml`'s
  `package.include` / `package.exclude` (most likely the
  existing `.gitignore` or the placeholder `Cargo.toml`
  excludes something it shouldn't). Do NOT actually run
  `cargo publish`; `--list` is read-only.

## Handling the round-01 partial work in the submodule tree

Round 01 left `src/client.rs`, `src/config.rs`, `src/dialect/`,
`src/error.rs`, `src/request.rs`, `src/response.rs`,
`src/retry.rs`, `src/schema.rs`, `src/types.rs`, `tests/` (all
untracked) + modified `Cargo.toml`, `CHANGELOG.md`,
`README.md`, `src/lib.rs` in the submodule. These were
produced under the wrong item-5 instruction but the rest of
round 01's plan was correct. **Treat the partial work as your
starting state**; inspect each file against the round-01 plan
+ this round-02 correction, then keep / adjust / discard per
your judgement:

- If the `tests/*.rs` files already have the broken
  `include_str!` paths, rewrite them to load from
  `tests/fixtures/...` inside the submodule.
- If `src/types.rs` makes sense as a shared-types module,
  keep it. If it's a redundant split the spec didn't call
  for and the module layout reads more naturally without it,
  collapse it. Your call — the spec said 10 modules but
  didn't forbid an 11th.
- If any module was only partially written (`todo!()`,
  `unimplemented!()`, `// TODO` stubs), complete it.
- If anything in the partial work already implements the
  correction (e.g., you started with `tests/fixtures/`
  because you independently spotted the problem and
  pre-corrected), so much the better.

If starting from the partial state is messier than starting
from scratch — e.g., `src/types.rs` has contradictions with
`src/response.rs`, or the dialect/ subtree uses a different
dispatch pattern than the spec assumes — then blow it away
and start over. Either path is fine. Return which you
chose + why under Residual risks.

Parent tree: round 01 also modified `Cargo.lock`. That's
fine; it'll be legitimately regenerated by anything that
touches dependencies and Claude will review + commit it.

## Verification loop (updated)

```sh
./scripts/pre-landing.sh philharmonic-connector-impl-llm-openai-compat
cargo test -p philharmonic-connector-impl-llm-openai-compat --all-targets
cargo package --list -p philharmonic-connector-impl-llm-openai-compat
git -C philharmonic-connector-impl-llm-openai-compat status --short
git -C . status --short
```

Expected:

- pre-landing: "all checks passed".
- cargo test: all non-ignored tests passing; `#[ignore]`-d
  smokes skipped.
- cargo package --list: includes every file under
  `tests/fixtures/` (plus `src/`, `Cargo.toml`, etc.); does
  NOT include the workspace's `docs/upstream-fixtures/` tree
  (because it shouldn't — it's outside the crate).
- submodule status: dirty files for Cargo.toml, CHANGELOG,
  README, src/*, tests/*, tests/fixtures/*.
- workspace status: `modified:
  philharmonic-connector-impl-llm-openai-compat` (pointer
  bump) and `modified: Cargo.lock`; no other changes.

## Structured output contract (updated)

In addition to round 01's structured output contract, add:

- **Partial-work disposition**: for each file round 01 left
  in the submodule tree, one-line note on what you did
  (kept / adjusted / rewrote / discarded) and why.
- **`cargo package --list` sanity**: paste the output
  lines naming files under `tests/fixtures/`. Confirm
  count matches the 10 fixture files (4 vLLM + 5 openai-chat
  + 1 synthesized).

Rest of round 01's structured output contract (summary,
files touched, pre-landing results, test counts, residual
risks, git state, dep versions) applies unchanged.

---

## Outcome

Pending — will be updated after Codex round-02 run.
