# D12 — custom_headers knob for llm-openai-compat (initial dispatch)

**Date:** 2026-05-10
**Slug:** `d12-llm-openai-compat-custom-headers`
**Round:** 01 (initial dispatch — D12, ROADMAP §3.C, single-
crate scope, lands before D7/D8/D9 per Yuka's directive)
**Subagent:** `codex:codex-rescue`

## Motivation

Production deployments hitting Hugging Face Inference need
the `X-HF-Bill-To` header to bill an organisation rather than
the personal account. The OpenAI-compatible API ecosystem has
similar per-provider knobs:

- **Hugging Face Inference**: `X-HF-Bill-To: <org-id>` — org
  billing.
- **OpenAI**: `OpenAI-Organization`, `OpenAI-Project` — usage
  attribution.
- **OpenRouter**: `HTTP-Referer`, `X-Title` — caller
  identification.
- Other gateways (Together, Fireworks, Groq) have their own.

Current `LlmOpenaiCompatConfig` has no escape hatch for these.
D12 adds a generic `custom_headers` map.

## References

- [`docs/ROADMAP.md` §3.C](../ROADMAP.md#c-connector-enhancements-1-dispatch)
  — D12 spec (this file is the implementation prompt for it).
- `philharmonic-connector-impl-llm-openai-compat/src/config.rs`
  — current `LlmOpenaiCompatConfig` struct (4 fields:
  `base_url`, `api_key`, `dialect`, `timeout_ms`). Uses
  `#[serde(deny_unknown_fields)]`.
- `philharmonic-connector-impl-llm-openai-compat/src/client.rs`
  — `execute_one_attempt`; the request-builder chain at lines
  26–37 is where the new headers attach (after the existing
  `Authorization` and `Content-Type` calls).
- [RFC 7230 §3.2](https://www.rfc-editor.org/rfc/rfc7230#section-3.2)
  — header field syntax, the source for what's a valid token
  character in a header name and what makes a header value
  malformed (CRLF injection, control chars).

## Context files pointed at

- `philharmonic-connector-impl-llm-openai-compat/src/config.rs`
- `philharmonic-connector-impl-llm-openai-compat/src/client.rs`
- `philharmonic-connector-impl-llm-openai-compat/src/error.rs`
  — for the typed-error variant naming pattern.
- `philharmonic-connector-impl-llm-openai-compat/src/lib.rs`
  — for re-export pattern + the `Implementation` trait wiring
  (only consult; don't edit unless the new error variant
  needs re-exporting).
- `philharmonic-connector-impl-llm-openai-compat/Cargo.toml`
  — version bump (0.1.0 → 0.1.1).
- `philharmonic-connector-impl-llm-openai-compat/CHANGELOG.md`
  — new entry.

## Outcome

**Completed 2026-05-10** — single deliverable landed; commits
`2fff3bb` (parent) + `3fef1f4`
(`philharmonic-connector-impl-llm-openai-compat` 0.1.0 →
0.1.1).

`./scripts/pre-landing.sh` GREEN end-to-end on Claude's
independent re-run. Codex's own pre-landing pass was also
green on first attempt — the "pre-landing-sh hygiene"
preamble (cargo fmt + rustdoc upfront) has reliably worked
for four consecutive rounds now.

**Spot-checks passed**:

- `client.rs:31-34` — existing `Authorization: Bearer
  <api_key>` and `Content-Type: application/json` lines
  preserved bit-for-bit. Custom-header iteration attaches
  AFTER those two.
- `config.rs:11` — `#[serde(try_from = "LlmOpenaiCompatConfigRaw")]`
  shim wires validate-at-deserialize-time semantics
  exactly as the prompt prescribed.
- `config.rs:48-49` — `TryFrom<Raw>::try_from` calls
  `validate_custom_headers` first thing.
- 17 config-level test scenarios + 1 wiremock-based
  client-level test (`execute_one_attempt_applies_custom_headers`).
- `retry.rs` touched only because the exhaustive `Error`
  match needed the new `InvalidCustomHeader` variant marked
  non-retryable — legitimate same-crate consequence, not
  scope creep.
- No edits to any other crate. No edits to
  `philharmonic-connector-common`,
  `philharmonic-connector-impl-api`, etc.

**Codex's deliverable choices** (per the prompt's residual-
risks request):

- Validation runs at deserialize-time via `#[serde(try_from
  = "LlmOpenaiCompatConfigRaw")]`. No execute-time-only
  fallback.
- Wiremock client-level test present (the crate already
  had wiremock as a dev-dep, no new dep added).
- Defensive runtime `HeaderName::try_from` /
  `HeaderValue::try_from` fallback maps to
  `Error::Internal` — should be unreachable since validate-
  time rejected anything reqwest would refuse, but guards
  against manually-constructed configs that bypass serde.

**ROADMAP §3.C discrepancy fixed in same parent-only
commit**: the §3.C entry initially said `HashMap`, but the
prompt and implementation correctly used `BTreeMap` for
deterministic ordering. ROADMAP entry updated to match what
landed; rationale documented inline.

**Open questions Codex surfaced**:

1. Per-impl validation hook at the API layer
   (`philharmonic-api/src/routes/endpoints.rs::validate_abstract_config`)
   so bad configs are rejected at endpoint-config write
   time rather than at first decrypt — out of D12 scope.
   Reasonable D13/D14 follow-up.
2. Dedicated `custom_headers` WebUI editor vs. JSON-edited
   through CodeMirror 6. Probably stays JSON-edited for
   v1; flagged for D6 / WebUI-pass consideration.

**Structured-output-contract honored** for the fourth
consecutive round (rounds 02 / 03 / 04 / D12 all emitted
the six-section report with `RUN STATUS: COMPLETE` token
before `task_complete`). The contract has settled into
reliable convention.

D12 was the smallest dispatch of the day (~6 minutes Codex
time vs round 02's 18 min and round 03's 12 min) and lands
the production HF Inference org-billing unblocker before
D7/D8/D9 per Yuka's directive.

---

## STRUCTURED-OUTPUT-CONTRACT — READ THIS FIRST

Both round 02 and round 03 of the embedding-datasets work
honored the contract — `RUN STATUS: COMPLETE` token + the six
required sections emitted before `task_complete`. Maintain
the bar. A run without the report is incomplete by definition,
even if the code compiles.

The contract is repeated at the end of the prompt; it's on
you to actually emit it before `task_complete`.

---

## Pre-landing-sh hygiene (apply early)

```bash
cargo fmt -p philharmonic-connector-impl-llm-openai-compat
```

(Use `CARGO_TARGET_DIR=target-main` per `CONTRIBUTING.md §5`
when running raw cargo. The wrapper scripts handle this.)

Add field-level rustdoc on every new `pub` item.

---

## Prompt (verbatim)

<task>
Add a `custom_headers` knob to `philharmonic-connector-impl-llm-openai-compat`'s
runtime endpoint config. Single deliverable, single crate,
no public-trait change, no crypto path touched.

If anything below contradicts the existing code's patterns or
`docs/ROADMAP.md §3.C` (the D12 entry), the docs / existing
code patterns win — flag the contradiction in your structured
output instead of guessing.

## Scope

1. **`config.rs`**: add a new field to `LlmOpenaiCompatConfig`:

   ```rust
   /// Caller-supplied HTTP headers to attach to every upstream
   /// request. Useful for per-provider knobs (e.g. Hugging
   /// Face Inference's `X-HF-Bill-To` for org billing,
   /// OpenAI's `OpenAI-Organization` / `OpenAI-Project`,
   /// OpenRouter's `HTTP-Referer` / `X-Title`). Reserved
   /// headers and malformed values are rejected at deserialize
   /// time — see `validate_custom_headers`.
   #[serde(default)]
   pub custom_headers: BTreeMap<String, String>,
   ```

   Use **`BTreeMap<String, String>`** (not `HashMap`) so:
   (a) test fixtures can compare bytes-for-bytes
   deterministically;
   (b) serde-encoded JSON keys are sorted, matching the
   workspace's deterministic-CBOR / canonical-JSON discipline
   (see round 01's CBOR codec for the same rationale).

   Add `use std::collections::BTreeMap;` at the top of
   `config.rs`.

   The existing `#[serde(deny_unknown_fields)]` stays — old
   configs without the field deserialize fine via
   `#[serde(default)]`; new fields not in the struct are
   still rejected.

2. **Validation**: add a `validate_custom_headers(&BTreeMap<String, String>)
   -> Result<(), CustomHeaderError>` function in `config.rs`
   that runs:

   - **Reserved-name rejection** (case-insensitive): reject
     these names regardless of case (`Authorization`,
     `AUTHORIZATION`, `authorization` all rejected):
     - `authorization`
     - `content-type`
     - `content-length`
     - `host`
     - `transfer-encoding`
     - `connection`

     The first three collide with what `client.rs` sets
     itself (`Authorization` from `api_key`, `Content-Type`
     for the JSON body, `Content-Length` from reqwest's body
     encoder). The last three are hop-by-hop / connection-
     management headers per RFC 7230 §6.1 + general HTTP
     hygiene.

   - **Header-name validity** per RFC 7230 §3.2.6 token chars:
     each name must be a non-empty ASCII string composed of
     token characters: `! # $ % & ' * + - . ^ _ ` | ~`,
     digits `0-9`, and letters `a-z A-Z`. Reject empty names
     and names with any other char (including spaces, control
     chars, non-ASCII).

   - **Header-value validity**: reject values containing CR
     (`\r`, byte `0x0D`) or LF (`\n`, byte `0x0A`). Reject
     values containing any byte in `0x00..=0x08`,
     `0x0B..=0x0C`, `0x0E..=0x1F`, or `0x7F` (control chars
     other than HT `0x09`). Empty values are **allowed**
     (some headers carry an empty string legitimately, e.g.
     `X-Empty-Hint: `). Non-ASCII is **allowed** (RFC 7230
     §3.2.4 deprecated obs-text but reqwest accepts UTF-8
     bytes — match reqwest's tolerance to keep the impl
     thin).

   Define a `CustomHeaderError` variant on the existing
   `Error` enum in `error.rs`:

   ```rust
   /// A custom header is reserved or malformed.
   #[error("custom header {name:?} is invalid: {reason}")]
   InvalidCustomHeader { name: String, reason: String },
   ```

   The `name` field is the offending key (so the operator
   can identify which entry was bad); `reason` is the
   one-line explanation (e.g. `"reserved name"`,
   `"contains CR/LF"`, `"empty name"`, `"name has invalid
   token character"`). Match the existing variants' wording
   style.

3. **Validation point**: validate at **deserialize time** via
   a serde `try_from` shim. Add a private intermediate type:

   ```rust
   #[derive(serde::Deserialize)]
   #[serde(deny_unknown_fields)]
   struct LlmOpenaiCompatConfigRaw {
       base_url: String,
       api_key: String,
       dialect: Dialect,
       #[serde(default = "default_timeout_ms")]
       timeout_ms: u64,
       #[serde(default)]
       custom_headers: BTreeMap<String, String>,
   }

   impl TryFrom<LlmOpenaiCompatConfigRaw> for LlmOpenaiCompatConfig {
       type Error = CustomHeaderError;
       fn try_from(raw: LlmOpenaiCompatConfigRaw) -> Result<Self, Self::Error> {
           validate_custom_headers(&raw.custom_headers)?;
           Ok(Self { /* fields */ })
       }
   }
   ```

   Then change `LlmOpenaiCompatConfig`'s derives to add
   `#[serde(try_from = "LlmOpenaiCompatConfigRaw")]`. (The
   `Serialize` derive stays — serializing always produces
   valid output since the only way to construct
   `LlmOpenaiCompatConfig` is via the validated `try_from`.)

   This catches bad `custom_headers` at every `serde_json::from_value`
   / `from_slice` call site — including the connector-
   service's deserialisation when it decrypts the SCK blob.
   The error surfaces as a deserialization error, which the
   service's existing error handling already turns into a
   typed connector-impl error.

   `CustomHeaderError` should be convertible into the existing
   `Error` enum (via `From<CustomHeaderError> for Error`)
   producing the `InvalidCustomHeader { name, reason }`
   variant, so the impl's request-time path can also propagate
   it cleanly if needed.

   **Why deserialize-time and not execute-time**: the user
   experience is "fail-fast at config write rather than at
   first request." The API server doesn't currently invoke
   per-impl validators at endpoint-write time, so we can't
   reject a bad config at the actual write step in this
   dispatch. But validating at deserialize-time means the
   first time the connector-service decrypts and decodes the
   config, the error surfaces immediately — same first
   request, but with a clean typed error rather than a
   silent header-injection or upstream rejection.
   Forward-compatible with future API-layer per-impl
   validation hooks (out of scope for D12).

4. **Application in `client.rs::execute_one_attempt`**:
   after the existing `.header(AUTHORIZATION, ...)` and
   `.header(CONTENT_TYPE, ...)` calls, attach each entry
   from `config.custom_headers`. The existing two calls take
   precedence — `custom_headers` validation already rejects
   `authorization` / `content-type` so there's no actual
   conflict, but order doesn't matter operationally.

   Use `reqwest::header::HeaderName::try_from(name)` and
   `HeaderValue::try_from(value)` to construct typed
   header-map entries. If `try_from` returns an error
   (shouldn't, since `validate_custom_headers` already
   rejected anything reqwest would refuse — but defensive),
   propagate as `Error::Internal("invalid custom_header
   could not be applied: {err}")` or the existing closest
   `Error` variant. Surface in residual risks if you can't
   find a clean variant.

5. **Tests** in `config.rs` `#[cfg(test)] mod tests` (extend
   the existing module):

   - `default_custom_headers_is_empty_when_omitted` — config
     JSON without `custom_headers` decodes with an empty
     map.
   - `accepts_well_formed_custom_headers` — `{"X-HF-Bill-To":
     "org_abc"}` decodes successfully.
   - `accepts_multiple_custom_headers` — at least three
     entries (e.g. HF, OpenRouter, OpenAI org ones) decode.
   - `rejects_reserved_authorization_case_insensitive` —
     `{"Authorization": "..."}` and `{"authorization": "..."}`
     and `{"AUTHORIZATION": "..."}` all rejected with the
     `InvalidCustomHeader { name: <whichever case>, reason:
     "reserved name" }` shape.
   - `rejects_reserved_content_type` — same.
   - `rejects_reserved_content_length` — same.
   - `rejects_reserved_host` — same.
   - `rejects_reserved_transfer_encoding` — same.
   - `rejects_reserved_connection` — same.
   - `rejects_crlf_in_value` — `{"X-Foo": "bad\r\nInjected:
     yes"}` rejected with reason mentioning CR/LF.
   - `rejects_lf_only_in_value` — `{"X-Foo": "bad\nthing"}`
     rejected.
   - `rejects_cr_only_in_value` — `{"X-Foo": "bad\rthing"}`
     rejected.
   - `rejects_control_char_in_value` — `{"X-Foo": "bad\x01"}`
     rejected.
   - `rejects_empty_name` — `{"": "value"}` rejected.
   - `rejects_invalid_token_char_in_name` — `{"X Foo":
     "value"}` (space) and `{"X:Foo": "value"}` (colon)
     and `{"日本語": "value"}` (non-ASCII) all rejected.
   - `accepts_empty_value` — `{"X-Empty": ""}` decodes
     successfully (empty values are valid).
   - `accepts_horizontal_tab_in_value` — `{"X-Tab":
     "before\tafter"}` decodes (HT is allowed by RFC 7230).

6. **Application-layer test** in `client.rs` (or a new file
   `tests/custom_headers.rs` if `client.rs` doesn't already
   have a test module — check the existing layout): use the
   `wiremock` dev-dep that's already in `Cargo.toml`. Set up
   a wiremock server expecting a request with specific
   `X-HF-Bill-To` and `OpenAI-Organization` headers, run
   `execute_one_attempt` against it with a config containing
   those custom headers, assert the wiremock matcher fires
   (proving the headers reached the upstream). Pick whichever
   wiremock matcher idiom the existing tests use; if there
   are no existing client-level tests, the simplest pattern
   is `MockServer::start().await` + `Mock::given(matchers::header(...)).expect(1)`.

7. **Version bump**: `0.1.0` → `0.1.1` in `Cargo.toml`. Add
   `## [0.1.1] - 2026-05-10` to `CHANGELOG.md` listing the
   `custom_headers` field + validation + reserved-header
   rejection. Forward-compat (existing configs deserialize
   unchanged via `#[serde(default)]`); patch bump is
   correct.

8. **No edits** to `philharmonic-connector-common`,
   `philharmonic-connector-impl-api`, `philharmonic-connector-client`,
   `philharmonic-connector-service`, or any other crate.
   D12 is single-crate by design — that's the whole point
   of placing this work as a "Connector enhancement" rather
   than a Tier 2/3 implementation.

## Cross-deliverable: workspace verification

Run **`./scripts/pre-landing.sh`** before declaring done. It
auto-detects modified crates and runs fmt + check + clippy
(`-D warnings`) + rustdoc + workspace-test + per-crate
`--ignored` test phase. The crate doesn't have testcontainers
deps, so the `--ignored` phase is fast.

Run **`./scripts/test-scripts.sh`** — should pass clean (no
shell scripts touched).

Do **not** run `cargo publish` (or `cargo publish --dry-run`)
on any crate. Publishing is Yuka's gate.

<structured_output_contract>
**Critical: emit this report before `task_complete`.**

Six sections, in this order:

1. **Summary** — one paragraph: what landed. Include the
   verbatim string "RUN STATUS: COMPLETE" or "RUN STATUS:
   PARTIAL — <one-line reason>" so Claude can grep for it.

2. **Touched files** — exhaustive list, one line per file:
   `(new|edited|deleted) <path> — <one-line note>`. Include
   `Cargo.lock`.

3. **Verification results** — exact commands run + outcomes:
   - `./scripts/pre-landing.sh` — pass/fail/exit-code.
   - `./scripts/test-scripts.sh` — pass/fail.
   - Any per-crate cargo command for focused debugging.

4. **Residual risks / known issues** — including:
   - Whether validation runs at deserialize-time via the
     `try_from` shim, or whether you fell back to
     execute-time validation, and why.
   - Whether the wiremock-based client-level test is
     present, partial, or absent.
   - Any case where `HeaderName::try_from` /
     `HeaderValue::try_from` could still fail at runtime
     despite the validate-time check, and how you handle
     that defensively.
   - Behaviour when the upstream provider rejects a custom
     header (out of D12's scope — surface in case it bites
     someone later).

5. **Git state** — current `HEAD` SHAs in the touched
   submodule + parent. Confirm no commits were made.

6. **Open questions** — questions for Yuka or Claude Code:
   - Whether to add a per-impl validation hook at the API
     layer (`philharmonic-api/src/routes/endpoints.rs::validate_abstract_config`)
     so bad configs are rejected at write time rather than
     at first decrypt — out of D12 scope, but a natural
     follow-up.
   - Whether to surface `custom_headers` in the WebUI as a
     dedicated UI element vs leaving it as JSON-edited
     through the existing CodeMirror 6 editor.
</structured_output_contract>

<default_follow_through_policy>
- Run `cargo fmt` + add field-level rustdoc on new `pub`
  items BEFORE `pre-landing.sh`.
- If a test fails, fix the implementation before moving on.
- If you discover that `LlmOpenaiCompatConfig`'s
  `deny_unknown_fields` interacts badly with the `try_from`
  shim, surface in residual risks; the fallback is to drop
  `deny_unknown_fields` from the raw type and rely on the
  validated `LlmOpenaiCompatConfig` field set being the
  closed-world definition. (Should not happen — `try_from`
  with a derived `Deserialize` on the raw type composes
  naturally — but flag if it does.)
- If you find yourself wanting to touch any other crate,
  stop and surface — D12 is single-crate by design.
</default_follow_through_policy>

<completeness_contract>
"Done" means:

- `LlmOpenaiCompatConfig` has the `custom_headers` field with
  `try_from`-based validation.
- Reserved-header rejection (case-insensitive) + CRLF
  rejection + token-char validation all in place.
- Tests for every reserved name, every CRLF / control case,
  every accept case.
- `client.rs::execute_one_attempt` attaches the headers.
- `philharmonic-connector-impl-llm-openai-compat` 0.1.0 →
  0.1.1 with CHANGELOG entry.
- `pre-landing.sh` clean.
- Structured output report emitted before `task_complete`.

Partial completion is acceptable if you hit a token limit or
a genuine blocker — but you must say so explicitly with
"RUN STATUS: PARTIAL — <reason>".

A run without the structured-output report is **incomplete**.
</completeness_contract>

<verification_loop>
1. Edit code.
2. Add/update tests.
3. Run `cargo fmt -p philharmonic-connector-impl-llm-openai-compat`.
4. Add field-level rustdoc on new `pub` items.
5. Run `CARGO_TARGET_DIR=target-main cargo test -p philharmonic-connector-impl-llm-openai-compat`.
6. If green, run `./scripts/pre-landing.sh` once.
7. Emit the structured output report.
8. Then `task_complete`.
</verification_loop>

<missing_context_gating>
If you find yourself needing information not in this prompt
or the cited authoritative docs, **stop** and report what's
missing in the structured output's "Open questions" section.

Specifically: do **not**:

- Touch any other crate (this is single-crate scope).
- Add a public-trait change to
  `philharmonic-connector-impl-api`.
- Edit `philharmonic-connector-common` or
  `philharmonic-connector-service` — those are crypto-
  adjacent and out of D12 scope.
- Change `client.rs::execute_one_attempt`'s overall flow
  beyond adding the new header iteration.
- Change the `Authorization: Bearer <api_key>` or
  `Content-Type: application/json` lines.
</missing_context_gating>

<action_safety>
Files allowed to be created or modified:

- `philharmonic-connector-impl-llm-openai-compat/src/config.rs`
  (edited — new field, raw struct, try_from, validate fn).
- `philharmonic-connector-impl-llm-openai-compat/src/error.rs`
  (edited — `InvalidCustomHeader` variant, possibly
  `CustomHeaderError` type if separate).
- `philharmonic-connector-impl-llm-openai-compat/src/client.rs`
  (edited — header attachment).
- `philharmonic-connector-impl-llm-openai-compat/src/lib.rs`
  (edited only if a new pub type needs re-exporting —
  minimize).
- `philharmonic-connector-impl-llm-openai-compat/Cargo.toml`
  (edited — version bump).
- `philharmonic-connector-impl-llm-openai-compat/CHANGELOG.md`
  (edited — new entry).
- `philharmonic-connector-impl-llm-openai-compat/tests/custom_headers.rs`
  (new — only if `client.rs` doesn't already have a test
  module; otherwise inline).
- `Cargo.lock` (auto-regenerated — leave dirty for Claude
  to commit alongside).

Files NOT to touch (flag if you find a reason to):

- The workspace `Cargo.toml` `[patch.crates-io]` block.
- Any other crate in the workspace.
- Any `.claude/`, `docs/`, `scripts/` content.

Do **not** run `git add`, `git commit`, `git push`,
`commit-all.sh`, `push-all.sh`, or `cargo publish`. Codex
does not commit on this workspace.
</action_safety>

## Git rules (workspace-specific, mandatory)

- **Never** run `git commit` / `git push` / `git add`.
- **Never** invoke `scripts/commit-all.sh` or
  `scripts/push-all.sh`.
- **Never** run `cargo publish`.
- All cargo commands must use `CARGO_TARGET_DIR=target-main`.
- Don't `--no-verify` around any hooks.

Read-only git is fine: `git status`, `git diff`, `git log`,
`git show`, `git branch`, `git submodule status`.

## Verification commands (mandatory before declaring done)

1. `./scripts/pre-landing.sh`.
2. `./scripts/test-scripts.sh`.

Optional, for focused debugging:

- `CARGO_TARGET_DIR=target-main cargo test -p philharmonic-connector-impl-llm-openai-compat`
- `CARGO_TARGET_DIR=target-main cargo clippy -p philharmonic-connector-impl-llm-openai-compat --all-targets -- -D warnings`

</task>
