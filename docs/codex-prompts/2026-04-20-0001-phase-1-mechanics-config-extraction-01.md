# Phase 1 — `mechanics-config` extraction (initial prompt)

**Date:** 2026-04-20
**Slug:** `phase-1-mechanics-config-extraction`
**Round:** 01 (initial dispatch)
**Subagent:** `codex:codex-rescue`

## Motivation

Kicks off Phase 1 of the v1 roadmap: split the Boa-free schema
types (`MechanicsConfig`, `HttpEndpoint`, retry policy, slot
specs, URL template AST) out of `mechanics-core` into a new
`mechanics-config` crate so `philharmonic-connector-client` (the
lowerer, not yet implemented) can consume the schema without
pulling Boa into its dependency tree.

## References

- `ROADMAP.md` §Phase 1 — target versions, acceptance criteria,
  wrapper-newtype approach.
- `ROADMAP.md` §2 — submodule discipline, Git-via-scripts rule,
  signoff requirement.
- `docs/design/06-execution-substrate.md` §"Schema extraction
  (settled)" — migration plan, rationale, wrapper pattern.
- `docs/design/13-conventions.md` §Git workflow.

## Context files pointed at

- `mechanics-core/src/internal/http/` (source of the types).
- `mechanics-core/src/internal/http/tests/` (tests to split).
- `mechanics-config/src/lib.rs` (empty 0.0.0 placeholder).

## Outcome

Codex hit ~1.94 M cumulative tokens and silently terminated
mid-task after writing `mechanics-core/src/internal/http/
headers.rs`. `mechanics-config/` schema side is mostly in place;
`mechanics-core/` rewire is partial — workspace does not compile
(13 errors, mostly in `mechanics-core/src/internal/runtime.rs`).
Nothing was committed. A follow-up prompt (`-02`) resumes from
the partial state.

---

## Prompt (verbatim)

<task>
Execute Phase 1 of the Philharmonic workspace roadmap: extract schema types from `mechanics-core` into a new `mechanics-config` crate so downstream consumers (notably `philharmonic-connector-client`, not yet implemented) can depend on the schema without pulling in Boa.

Repository: `/home/mori/philharmonic` — a Rust Cargo workspace of 23 submodules, each its own Git repo at `github.com/metastable-void/*`. The parent `Cargo.toml` uses `[patch.crates-io]` to redirect Philharmonic crate deps to local submodule paths so cross-crate development works without publishing.

Authoritative references — read BEFORE implementing. If anything below contradicts these docs, the docs win:
- `ROADMAP.md` §Phase 1 (target versions, task list, acceptance criteria, wrapper-newtype approach)
- `ROADMAP.md` §2 (submodule discipline, Git-via-scripts rule, signoff requirement)
- `docs/design/06-execution-substrate.md` §"Schema extraction (settled)" (migration plan, rationale, wrapper pattern)
- `docs/design/13-conventions.md` §Git workflow (scripts-only, signoff mandatory)

Current state of the two crates involved:
- `mechanics-config/` — 0.0.0 placeholder. Empty `src/lib.rs`. CHANGELOG and CI already scaffolded.
- `mechanics-core/` — 0.2.2, published on crates.io. Schema types live in `src/internal/http/` (`config.rs`, `endpoint/`, `retry.rs`, `template.rs`, `query.rs`, `headers.rs`). Transport/runtime types live in `transport.rs`, `options.rs`, `endpoint/execute.rs`.

Target versions after this phase:
- `mechanics-config` → `0.1.0` (leave this in `Cargo.toml`; do NOT run `cargo publish` — publishing is a manual next step).
- `mechanics-core` → `0.2.3`.

What moves to `mechanics-config` (Boa-free, reqwest-free, tokio-free):
- `MechanicsConfig` (from `config.rs`).
- `HttpEndpoint`, `UrlParamSpec`, `QuerySpec`, `SlottedQueryMode`, `EndpointBodyType` (the public schema items from `endpoint/mod.rs`).
- `EndpointRetryPolicy` (from `retry.rs`).
- URL template AST and parser (`template.rs`).
- Slot/query/header validation helpers (`query.rs`, `endpoint/validate.rs`, any Boa-free helpers in `headers.rs`).
- `HttpMethod` (the enum).
- Pure structural validation methods: `MechanicsConfig::validate`, `HttpEndpoint::validate_config`.

What stays in `mechanics-core` (Boa + reqwest + tokio dependents):
- `EndpointHttpClient`, `ReqwestEndpointHttpClient`, `EndpointHttpRequest` / `Response` / `Body`, and `EndpointHttpHeaders` (from `transport.rs`).
- `EndpointCallOptions`, `EndpointCallBody`, `EndpointResponse`, `EndpointResponseBody` (from `options.rs`).
- `endpoint/execute.rs` (the actual HTTP call path).
- Any parts of `endpoint/request.rs` that consume runtime types (`EndpointCallOptions`). If chunks of that file are pure schema logic (URL building from schema + per-call strings, header allowlist validation), split cleanly — move the pure chunks to `mechanics-config`.

`HttpMethod` split:
- The enum moves to `mechanics-config`.
- `as_str()` and `supports_request_body()` move with it (no external deps).
- `as_reqwest_method()` stays in `mechanics-core`. You cannot add inherent impls to a foreign type in Rust; use either a free function (`fn to_reqwest_method(m: HttpMethod) -> reqwest::Method`) or a local extension trait.

Wrapper newtypes in `mechanics-core` for Boa GC integration:
Per design doc 06, add newtypes that implement `Trace`, `Finalize`, `JsData` over the `mechanics_config::*` types, using `#[unsafe_ignore_trace]`. Example shape from doc 06:

```rust
#[derive(Trace, Finalize, JsData)]
pub struct BoaMechanicsConfig(
    #[unsafe_ignore_trace] mechanics_config::MechanicsConfig,
);
```

The wrappers are the representation `mechanics-core`'s runtime uses internally; conversion to/from the plain `mechanics_config::*` type should be trivial (`From` impls). `unsafe_ignore_trace` is sound because the config types hold no GC-managed objects — plain data only.

Backward-compatible re-exports in `mechanics-core`:
Every type that was previously reachable via `mechanics_core::endpoint::*` or `mechanics_core::job::MechanicsConfig` MUST remain reachable via the same path after the extraction. Use `pub use mechanics_config::...` to re-export. Any consumer of the 0.2.2 API should build unchanged against 0.2.3 — this is explicitly not a breaking change.

Tests:
- Pure-schema tests (URL template parsing, slot spec resolution, query emission, retry-policy validation, byte-length bounds, etc.) migrate to `mechanics-config`.
- Tests exercising the Boa runtime, the HTTP client, or `reqwest` stay in `mechanics-core`.
- Files in `mechanics-core/src/internal/http/tests/` are candidates — split per-file.

`mechanics-config/Cargo.toml`:
- `description`, `readme`, `license`, `keywords`, `categories` are already populated. Verify they're sensible; adjust only if needed.
- Dependencies: `serde` (derive), `serde_json`, plus the minimum utility crates you need (e.g., `thiserror`). No `boa_engine`, no `boa_gc`, no `reqwest`, no `tokio`, no `url` if you can avoid it. If you need `url` or similar, that's fine — just keep the tree small.
- Remove any `[profile.release]` block (profiles belong only at the workspace root; the workspace currently warns on per-crate profile blocks).
- Update `CHANGELOG.md` with a `0.1.0` entry describing the initial extraction.

`mechanics-core/Cargo.toml`:
- Bump version to `0.2.3`.
- Add `mechanics-config = "0.1.0"` to `[dependencies]` (the workspace's `[patch.crates-io]` rewrites this to the local path at build time).
- Update `CHANGELOG.md` with a `0.2.3` entry noting: moved schema types to `mechanics-config`; added wrapper newtypes for Boa GC integration; re-exports preserve the public API.

Git workflow (strict):
- Every commit MUST be signed off (`-s`). The workspace's `scripts/commit-all.sh` enforces this; do not run raw `git commit`.
- Commit inside submodules first, then the parent (which bumps submodule pointers). `scripts/commit-all.sh` walks submodules then parent in the right order.
- Do NOT run `scripts/push-all.sh`. Do NOT `git push` manually. Yuka will review the commits before pushing.
- Do NOT run `cargo publish`.

Out of scope (do not touch):
- Any submodule other than `mechanics-config/` and `mechanics-core/`.
- Parent-workspace docs (`CLAUDE.md`, `README.md`, `ROADMAP.md`, `docs/design/*`). They already describe the end state.
- Design decisions — design docs are authoritative; flag conflicts instead of deciding.

Final verification (must all pass from the workspace root):
- `cargo check --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo tree -p mechanics-config | grep -iE 'boa|reqwest|tokio'` — must produce no output.
</task>

<structured_output_contract>
Return:
1. One-paragraph summary of the extraction as delivered.
2. Touched/new files grouped by crate (`mechanics-config/`, `mechanics-core/`, parent-workspace pointer bump only).
3. Verification results: exact command + exit status + short summary for each of `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and `cargo tree -p mechanics-config | grep -iE 'boa|reqwest|tokio'`.
4. Residual risks or follow-ups (anything you couldn't migrate cleanly, any test gaps introduced, any design-doc ambiguity you resolved by interpretation — call these out explicitly).
5. Git state at the end: list of commits created per submodule and in the parent, with SHAs. Confirm NOTHING has been pushed.
</structured_output_contract>

<default_follow_through_policy>
Default to the most reasonable low-risk interpretation consistent with the design docs and keep going.
Only stop to ask questions when a missing detail changes correctness or requires a design-level decision that the docs don't cover.
</default_follow_through_policy>

<completeness_contract>
Resolve the task fully before stopping.
- The workspace must `cargo check --workspace` and `cargo test --workspace` cleanly at the end.
- `mechanics-config` must have no Boa / reqwest / tokio in its dependency tree.
- Existing consumers of `mechanics_core::endpoint::*` and `mechanics_core::job::MechanicsConfig` must keep building with no code changes (backward compat via re-exports).
- Commits exist in both submodules and the parent pointer is bumped.
</completeness_contract>

<verification_loop>
Before finalizing, from the workspace root run all four verification commands listed in `<task>`. If any fails, revise the code and re-run rather than reporting the first draft. Capture the output you'll include in the structured report while the workspace is green.
</verification_loop>

<missing_context_gating>
Do not guess missing facts. Read `docs/design/06-execution-substrate.md` and `ROADMAP.md` §Phase 1 before implementing. If a design doc is silent or ambiguous on a concrete point (e.g., which file a helper should land in), prefer the minimal-move interpretation and note it as a "resolved-by-interpretation" item in the residual-risks section of your report.
</missing_context_gating>

<action_safety>
Keep changes tightly scoped to `mechanics-config/` and `mechanics-core/`. The only allowed parent-workspace change is the submodule-pointer bumps for those two crates (and they happen automatically via `scripts/commit-all.sh`).
Use `scripts/commit-all.sh "<message>"` for every commit — never raw `git commit`, never ad-hoc `git` operations. Never run `scripts/push-all.sh`. Never run `cargo publish`.
If you encounter unexpected state (e.g., unfamiliar modifications in some submodule), STOP and report rather than trying to fix it.
</action_safety>

<progress_updates>
If you provide progress updates, keep them brief and outcome-based. Mention phase transitions — types moved, wrapper newtypes done, tests migrated, verification clean — or blockers. Skip play-by-play.
</progress_updates>
