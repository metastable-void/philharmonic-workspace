# D16 — `llm_openai_compat` `tool_call_fallback_auto` dialect (initial dispatch)

**Date:** 2026-05-11
**Slug:** `d16-llm-openai-compat-tool-call-fallback-auto`
**Round:** 01 (initial dispatch — D16, ROADMAP §3.C, single
crate `philharmonic-connector-impl-llm-openai-compat`)
**Subagent:** `codex:codex-rescue`

## Motivation

Some OpenAI-compatible inference providers — notably some
local LLM server implementations and some Hugging Face
Inference Providers — reject the forced
`tool_choice: {type: "function", function: {name: ...}}`
that the existing `tool_call_fallback` dialect emits, and
need `tool_choice: "auto"` instead. With only one tool
offered (the `emit_output` function), the model still
effectively has to pick it, so the structured-output
contract is preserved.

D16 ships a new dialect variant
`Dialect::ToolCallFallbackAuto` (serialised as
`"tool_call_fallback_auto"`) alongside the existing
`Dialect::ToolCallFallback`. Shape decision locked
2026-05-11 to **option (a)** per HUMANS.md follow-up
directive and Yuka's 2026-05-11 conversation:
a separate variant rather than a sub-option on the
existing dialect. Reasoning: keeps `Dialect` a clean
discriminator (no mixing of per-request flags with
discriminator-style variants), and gives operators a
single setting to flip at endpoint-config write time
without needing to remember a back-compat default
direction.

This unblocks production deployments hitting upstream
providers that reject forced tool_choice; until D16
ships, those tenants have no working dialect option
(`openai_native` requires native structured-output
support which they also lack; `vllm_native` is vLLM-
specific; `tool_call_fallback` is the failing one).

## References

- [`docs/ROADMAP.md` §3.C](../ROADMAP.md#c-connector-enhancements-2-dispatches)
  — D16 entry with the locked-in option (a) shape.
- [`HUMANS.md` §"Follow-up tasks from 2026-05-10 work"](../../HUMANS.md)
  — Yuka's original directive listing the missing
  `tool_choice: "auto"` support.
- [`docs/design/08-connector-architecture.md`](../design/08-connector-architecture.md)
  — connector framework, the `llm_generate` capability's
  normalized `{output, stop_reason, usage}` shape with
  mandatory `output_schema`.
- D12 prompt + outcome:
  [`docs/codex-prompts/2026-05-10-0005-d12-llm-openai-compat-custom-headers-01.md`](2026-05-10-0005-d12-llm-openai-compat-custom-headers-01.md)
  — most recent dispatch in the same crate; mirror its
  version-bump + CHANGELOG + test conventions.
- Current crate state:
  - Version: `0.1.1` locally (D12 bump,
    unpublished — published latest is `0.1.0`).
  - Dialect enum at
    `philharmonic-connector-impl-llm-openai-compat/src/config.rs:158-165`:
    `OpenaiNative`, `VllmNative`, `ToolCallFallback`,
    `#[serde(rename_all = "snake_case")]`. Add a fourth
    variant `ToolCallFallbackAuto` serialising as
    `"tool_call_fallback_auto"`.
  - Dispatch points at
    `philharmonic-connector-impl-llm-openai-compat/src/dialect/mod.rs`:
    `build_request_body` (line 16) and
    `extract_response` (line 24). Both have one match arm
    per variant; add the new arm.
  - Existing implementation at
    `philharmonic-connector-impl-llm-openai-compat/src/dialect/tool_call_fallback.rs`
    — the new variant reuses
    `tool_call_fallback::extract_response` verbatim (the
    response shape is identical) and shares ~90% of
    `tool_call_fallback::translate_request` (the only
    diff is the `tool_choice` value).

## Context files pointed at

- `philharmonic-connector-impl-llm-openai-compat/src/config.rs`
  — add `ToolCallFallbackAuto` variant to `Dialect`
  enum. Update the `dialect_enum_roundtrips_all_three`
  test (rename to `dialect_enum_roundtrips_all_four` or
  similar; cover the new variant).
- `philharmonic-connector-impl-llm-openai-compat/src/dialect/mod.rs`
  — add `pub(crate) mod tool_call_fallback_auto;` and
  the dispatch arms in `build_request_body` and
  `extract_response`.
- `philharmonic-connector-impl-llm-openai-compat/src/dialect/tool_call_fallback.rs`
  — likely refactor to share the body-construction code
  (see "Shape" section below).
- `philharmonic-connector-impl-llm-openai-compat/src/dialect/tool_call_fallback_auto.rs`
  — new module.
- `philharmonic-connector-impl-llm-openai-compat/Cargo.toml`
  — version bump 0.1.1 → 0.1.2.
- `philharmonic-connector-impl-llm-openai-compat/CHANGELOG.md`
  — add `[Unreleased]` entry or new `[0.1.2]` section.

## Outcome

Pending — will be updated after Codex run.

---

## STRUCTURED-OUTPUT-CONTRACT — READ THIS FIRST

Rounds 02 / 03 / D12 / D6 / D13 / D11 / D11-follow-up #3
(the last seven) all honored the contract: six-section
report emitted before `task_complete`, including the
verbatim `RUN STATUS: COMPLETE` token. **Streak is 8/8
since the contract was added — maintain it.**

The contract is repeated at the end of the prompt.

---

## Shape (locked decisions)

The new variant `Dialect::ToolCallFallbackAuto` lives
alongside `Dialect::ToolCallFallback`. Serde rename
matches the enum's existing `rename_all = "snake_case"`,
producing the wire string `"tool_call_fallback_auto"`.

The translate-request body for the new variant is
**byte-identical** to `tool_call_fallback`'s body except
for the `tool_choice` field:

| Field | `tool_call_fallback` | `tool_call_fallback_auto` |
|---|---|---|
| `model` | identical | identical |
| `messages` | identical | identical |
| `tools` | identical (`emit_output` function + same parameters) | identical |
| `tool_choice` | `{type: "function", function: {name: "emit_output"}}` | `"auto"` |
| `max_completion_tokens` / `temperature` / `top_p` / `stop` | identical | identical |

The extract-response path is **identical**:

- `choices[0].finish_reason` → `StopReason` via the same
  `map_finish_reason` table.
- `choices[0].message.tool_calls[0].function.arguments` →
  JSON-decoded as the structured output.
- `usage` → normalized via the shared `normalized_usage`
  helper.

This means the new dialect module can delegate
`extract_response` to the existing
`tool_call_fallback::extract_response` directly, no
duplication needed.

For `translate_request`, you have two reasonable
sub-shapes (your call — both are fine, prefer whichever
is cleaner in the resulting code):

**Sub-shape 1**: refactor `tool_call_fallback.rs` to
expose a shared `pub(crate)` helper
`translate_request_with_tool_choice(request, tool_choice:
JsonValue) -> JsonValue` that the existing
`translate_request` calls with the forced tool_choice
literal, and the new module calls with `json!("auto")`.

**Sub-shape 2**: duplicate the body-construction
function in `tool_call_fallback_auto.rs`. Acceptable if
sub-shape 1 produces awkward module organization.

Don't change the existing `tool_call_fallback` dialect's
behavior — every test in `tool_call_fallback.rs::tests`
must still pass byte-for-byte after the refactor (the
forced tool_choice path is a back-compat guarantee).

## Tests

Required:

1. **Round-trip the enum**: `dialect_enum_roundtrips_all_four`
   (or similar) covering `OpenaiNative`, `VllmNative`,
   `ToolCallFallback`, `ToolCallFallbackAuto` via the
   existing `[Dialect::OpenaiNative, ...]` array test
   pattern in `config.rs:216-228`.

2. **Translate request, basic**: mirror the existing
   `tool_call_fallback::tests::translates_basic_request_to_expected_body`
   test in the new module, with the only diff being
   `body["tool_choice"] == json!("auto")` instead of the
   forced function object.

3. **Translate request, optional fields**: confirm
   `max_completion_tokens`, `temperature`, `top_p`,
   `stop` pass through correctly in the new variant
   (mirror whatever coverage the existing variant has).

4. **Extract response delegates correctly**: a single
   test that constructs a `ProviderChatResponse` with
   one tool_call carrying JSON arguments, runs it
   through the new variant's `extract_response`, and
   asserts the same `(output, stop_reason, usage)` shape
   as the forced variant. If sub-shape 1 is chosen
   (shared helper), this test exercises that the new
   variant's wrapper delegates without mutating; if
   sub-shape 2 (duplication), it exercises the
   duplicate.

5. **finish_reason mapping**: the same five-case table
   already tested for the forced variant. If you reuse
   `map_finish_reason` directly via re-export, you can
   skip duplicating this test in the new module — but
   add at least one smoke test confirming the
   delegation works.

Existing `tool_call_fallback` tests must remain green
byte-for-byte; do not modify them.

## Verification flow

```sh
./scripts/pre-landing.sh
```

Runs fmt + check + clippy (-D warnings) + rustdoc + test
across the workspace, including the crate-specific
`--ignored` phase for the crates this dispatch touches.

Also run:

```sh
./scripts/check-api-breakage.sh philharmonic-connector-impl-llm-openai-compat
```

`cargo-semver-checks` against the crates.io baseline
(`0.1.0`). The new variant addition to a
non-`#[non_exhaustive]` enum is a minor breaking change
for downstream `match` users; bumping the patch version
(0.1.1 → 0.1.2) is acceptable because the crate is at
`0.x.y` (SemVer allows breaking changes at minor or
patch level pre-1.0; bumping patch matches D12's
convention). semver-checks may flag the variant
addition; **flag the output in residual risks** rather
than papering it over — Yuka's call on whether to bump
patch or minor.

Skip:

- No version bump on consumers' Cargo.toml. The
  workspace-path pin in the root Cargo.toml resolves to
  the local 0.1.2 automatically. The meta-crate
  `philharmonic`'s `philharmonic-connector-impl-llm-openai-compat
  = "0.1.0"` requirement is a SemVer range that accepts
  0.1.2 (no change needed there).
- No CHANGELOG churn beyond the new `[0.1.2]` entry.
- No publish — Claude reviews and decides post-Codex.

## Prompt (verbatim)

<task>
Ship D16: add a new dialect variant `tool_call_fallback_auto`
to `philharmonic-connector-impl-llm-openai-compat`. The
new variant carries the same tools-array shape as the
existing `tool_call_fallback` but sends `tool_choice:
"auto"` instead of the forced
`{type: "function", function: {name: "emit_output"}}`.

Single crate. No public-trait change. No other crate
edits. No crypto path touched.

Deliverables (in order):

1. **`src/config.rs`**: add `ToolCallFallbackAuto` variant
   to the `Dialect` enum. The existing `rename_all =
   "snake_case"` produces the wire string
   `"tool_call_fallback_auto"` automatically — no extra
   `#[serde(rename = ...)]` needed.

   Update the existing `dialect_enum_roundtrips_all_three`
   test (in `mod tests`) to cover all four variants.
   Rename to `dialect_enum_roundtrips_all_four` or
   similar. The test's array literal needs the new
   `Dialect::ToolCallFallbackAuto` element.

2. **`src/dialect/mod.rs`**:
   - Add `pub(crate) mod tool_call_fallback_auto;`
     alongside the existing module declarations.
   - Add the new dispatch arm in `build_request_body`:
     `Dialect::ToolCallFallbackAuto =>
     tool_call_fallback_auto::translate_request(request)`.
   - Add the new dispatch arm in `extract_response`:
     `Dialect::ToolCallFallbackAuto =>
     tool_call_fallback_auto::extract_response(&provider)`.
   - Ensure clippy doesn't complain about
     non-exhaustive match patterns (the existing arms
     are explicit; the new one slots in without
     reformatting).

3. **`src/dialect/tool_call_fallback.rs`** (refactor for
   reuse — sub-shape 1 preferred, but sub-shape 2 is
   acceptable if the refactor produces awkward module
   organization):

   Sub-shape 1: extract the body-construction logic to
   a `pub(crate)` helper
   `translate_request_with_tool_choice(request:
   &LlmGenerateRequest, tool_choice: JsonValue) ->
   JsonValue`. The existing `translate_request` calls
   it with the forced tool_choice literal. The existing
   tests continue to use the public `translate_request`
   entry point (no test changes here; byte-for-byte
   compatibility required).

   Sub-shape 2: leave `tool_call_fallback.rs`
   unchanged; the new module duplicates the body
   construction with the `tool_choice` value swapped.

4. **`src/dialect/tool_call_fallback_auto.rs`** (new
   module):

   - `pub(crate) fn translate_request(request:
     &LlmGenerateRequest) -> JsonValue` — either calls
     the shared helper (sub-shape 1) or duplicates the
     body construction with `tool_choice:
     json!("auto")` (sub-shape 2).
   - `pub(crate) fn extract_response(provider:
     &ProviderChatResponse) -> Result<LlmGenerateResponse>`
     — delegates directly to
     `super::tool_call_fallback::extract_response(provider)`.
     Identical extraction path; no logic of its own.

   Inline `#[cfg(test)] mod tests` mirroring the four
   required tests above (basic translate, optional
   fields, extract delegation smoke test, plus the
   finish_reason smoke test if not reusing the existing
   one directly).

5. **`Cargo.toml`**: bump `version = "0.1.1"` →
   `version = "0.1.2"`. No dep changes.

6. **`CHANGELOG.md`**: add a `[0.1.2] - 2026-05-11`
   entry above the existing `[0.1.1] - 2026-05-10`
   block (the `[Unreleased]` placeholder line is the
   anchor — slot the new release between
   `[Unreleased]` and `[0.1.1]`). Single bullet
   covering the new variant + the operator-facing
   reason. Mirror the prose style of the `[0.1.1]`
   entry.

7. **Verification**: run
   `./scripts/pre-landing.sh` and
   `./scripts/check-api-breakage.sh
   philharmonic-connector-impl-llm-openai-compat`. The
   semver-checks may report the new enum variant as a
   minor breaking change for downstream `match` users;
   that's expected for a non-`#[non_exhaustive]` enum.
   Flag the output in residual risks.

8. **No publish.** Claude reviews and decides post-Codex.

## Hard constraints

- The existing `tool_call_fallback` dialect's behavior
  is a back-compat guarantee. Existing tenants relying
  on the forced tool_choice path continue to use
  `dialect: "tool_call_fallback"` and get identical
  upstream request bodies. Every test in
  `tool_call_fallback.rs::tests` must remain green
  byte-for-byte after any refactor.
- No public-trait change. The `Implementation` impl on
  the crate stays as-is; the new variant is just one
  more match arm internally.
- No crypto path touched. AAD / AEAD / COSE_Sign1 /
  COSE_Encrypt0 / SCK paths are untouched and don't
  appear in this dispatch.
- `#[serde(deny_unknown_fields)]` already applies to
  `LlmOpenaiCompatConfig` (via `LlmOpenaiCompatConfigRaw`
  per D12); no relaxation.
- No `unsafe` blocks. No panicking in lib `src/`
  (no `.unwrap()` / `.expect()` on `Result`/`Option`,
  no `panic!` / `unreachable!` / `todo!` /
  `unimplemented!` on reachable paths). Tests are
  exempt.

<structured_output_contract>
**Critical: emit this report before `task_complete`.**

Six sections, in this order:

1. **Summary** — one paragraph: what landed, sub-shape
   chosen (1 or 2), version bump applied, semver-checks
   outcome. Include the verbatim string "RUN STATUS:
   COMPLETE" or "RUN STATUS: PARTIAL — <reason>" for
   grep.

2. **Touched files** — exhaustive list with
   `(new|edited|deleted) <path> — <one-line note>`.

3. **Verification results** — exact commands + outcomes:
   - `./scripts/pre-landing.sh` — pass/fail/exit code.
   - `./scripts/check-api-breakage.sh
     philharmonic-connector-impl-llm-openai-compat` —
     pass/fail/output excerpt (especially relevant for
     the variant-addition semver question).
   - `./scripts/test-scripts.sh` — pass/fail (run only
     if you touched any `scripts/*.sh`; this dispatch
     should not).

4. **Residual risks / known issues** — including:
   - Sub-shape choice (1 or 2) and why.
   - semver-checks output for the enum-variant
     addition: whether it flagged a breaking change and
     how it framed it. If it did, note that Yuka may
     prefer a minor bump (0.1.1 → 0.2.0) instead of the
     patch bump (0.1.1 → 0.1.2) — that's her call, not
     yours.
   - Any prose-only divergence between what this prompt
     promised and what landed (e.g. if the refactor
     reshape was different from sub-shape 1 / 2 as
     described).
   - Test coverage gaps if any (e.g. if you couldn't
     write a test for some edge case because of how
     the existing module structure constrained it).

5. **Git state** — current `HEAD` SHA in the parent
   workspace repo and in the
   `philharmonic-connector-impl-llm-openai-compat`
   submodule. Confirm no commits made.

6. **Open questions** — questions for Yuka or Claude:
   - Patch bump (0.1.1 → 0.1.2, per this prompt) vs.
     minor bump (0.1.1 → 0.2.0, if semver-checks flags
     the variant addition as breaking and Yuka prefers
     to call it out via version).
   - Whether the new variant should also be wired into
     the workflow authoring guide
     `docs/guide/workflow-authoring.md` §
     `llm_openai_compat` dialect table as a follow-up
     (not in this dispatch's scope; Claude's call).
</structured_output_contract>

<default_follow_through_policy>
- Implement the deliverables in the order listed (config →
  dispatch → fallback refactor → new module → version +
  changelog → verification).
- After step 4, run `cargo test -p
  philharmonic-connector-impl-llm-openai-compat` directly
  before invoking the heavier pre-landing pipeline; faster
  iteration.
- Prefer sub-shape 1 unless the refactor produces an
  awkward module shape — extract a shared helper rather
  than duplicating. If you go with sub-shape 2, explain in
  residuals.
- The new variant gets identical extraction behavior to
  the existing one. Don't introduce a separate extract
  path that just happens to look the same — delegate to
  the existing one and document the delegation in a
  doc-comment on the new module's `extract_response`.
- No edits outside the crate. If you find yourself
  wanting to touch any other crate, **stop** and surface
  in residual risks.
</default_follow_through_policy>

<completeness_contract>
"Done" means:

- `Dialect::ToolCallFallbackAuto` variant added and
  dispatched in both `build_request_body` and
  `extract_response`.
- New module `tool_call_fallback_auto.rs` ships with
  `translate_request` + `extract_response` + tests.
- `tool_call_fallback.rs::tests` still green
  byte-for-byte.
- `Cargo.toml` version bumped 0.1.1 → 0.1.2.
- `CHANGELOG.md` has a `[0.1.2]` entry.
- `./scripts/pre-landing.sh` clean.
- `./scripts/check-api-breakage.sh` run and its output
  surfaced in residuals.
- Structured output report emitted before
  `task_complete`.

Partial completion is acceptable only if you hit a token
limit or genuine blocker — say so explicitly with "RUN
STATUS: PARTIAL — <reason>". Half-shipped enum variants
with missing dispatch arms compile but produce wrong
behavior at runtime; if you can't finish, revert the
config.rs change so the dispatch stays exhaustive.

A run without the structured-output report is
**incomplete**, even if the code landed.
</completeness_contract>

<verification_loop>
1. Implement config + dispatch + module.
2. `cargo test -p
   philharmonic-connector-impl-llm-openai-compat` — green.
3. Run `cargo check` on the workspace to catch any
   downstream match-pattern issues:
   `CARGO_TARGET_DIR=target-main cargo check --workspace`.
4. Run `./scripts/pre-landing.sh` once.
5. Run `./scripts/check-api-breakage.sh
   philharmonic-connector-impl-llm-openai-compat`.
6. Emit structured output report.
7. `task_complete`.
</verification_loop>

<missing_context_gating>
If you find yourself needing information not in this prompt
or the cited authoritative sources, **stop** and report
what's missing in the structured output's "Open questions"
section.

Specifically: do **not**:

- Touch any other Rust crate. D16 is single-crate.
- Add a new Cargo dependency.
- Mint new permission atoms or change crypto behavior.
- Edit `philharmonic`, `philharmonic-api`,
  `philharmonic-policy`, `philharmonic-workflow`, or any
  other workspace member.
- Edit the workspace `Cargo.toml`
  `[patch.crates-io]` block.
- Edit docs/guide/workflow-authoring.md (the new
  dialect will get a one-line addition there as a
  Claude follow-up after D16 lands; not in scope here).
- Edit `HUMANS.md`, `CLAUDE.md`, `AGENTS.md`,
  `CONTRIBUTING.md`, any `.claude/`, `docs-jp/`, or
  `scripts/` content.
- Publish to crates.io. No `cargo publish` even
  `--dry-run`. Claude reviews and decides post-Codex.
</missing_context_gating>

<action_safety>
Files allowed to be created or modified:

- `philharmonic-connector-impl-llm-openai-compat/src/config.rs`
  (edited — new enum variant + updated test).
- `philharmonic-connector-impl-llm-openai-compat/src/dialect/mod.rs`
  (edited — new module declaration + dispatch arms).
- `philharmonic-connector-impl-llm-openai-compat/src/dialect/tool_call_fallback.rs`
  (edited if sub-shape 1; untouched if sub-shape 2).
- `philharmonic-connector-impl-llm-openai-compat/src/dialect/tool_call_fallback_auto.rs`
  (new).
- `philharmonic-connector-impl-llm-openai-compat/Cargo.toml`
  (edited — version bump only).
- `philharmonic-connector-impl-llm-openai-compat/CHANGELOG.md`
  (edited — new `[0.1.2]` entry).
- `Cargo.lock` (will regenerate when cargo runs;
  expected to update).

Files NOT to touch (flag if you find a reason to):

- Any file under `bins/`, `philharmonic/`,
  `philharmonic-api/`, `philharmonic-policy/`,
  `philharmonic-workflow/`, `philharmonic-store*/`,
  `mechanics-*/`, or any other connector crate.
- The workspace `Cargo.toml` `[patch.crates-io]` block.
- `philharmonic-connector-impl-llm-openai-compat/src/client.rs`,
  `request.rs`, `response.rs`, `types.rs`, `lib.rs`,
  `error.rs` (the variant addition slots into existing
  dispatch points without changes here).
- `HUMANS.md`, `CLAUDE.md`, `AGENTS.md`, `CONTRIBUTING.md`,
  any `.claude/`, `docs/`, `docs-jp/`, or `scripts/`
  content.

Git rules: signed-off + signed commits via
`scripts/commit-all.sh` only. **Codex never runs
`commit-all.sh`** — Claude commits after reviewing your
work. No `cargo publish`. No raw `git commit` /
`git push` / `git add` / `git reset` / `git rebase` /
`git revert`. Read-only `git log` / `git diff` /
`git show` is fine.
</action_safety>
</task>
