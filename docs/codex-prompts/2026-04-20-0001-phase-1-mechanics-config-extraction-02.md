# Phase 1 — `mechanics-config` extraction (resume prompt)

**Date:** 2026-04-20
**Slug:** `phase-1-mechanics-config-extraction`
**Round:** 02 (resume from partial state)
**Subagent:** `codex:codex-rescue`

## Motivation

Round 01 (see `-01.md`) ran out of context at ~1.94 M cumulative
tokens and silently terminated mid-task. The schema side of
`mechanics-config/` is mostly written; `mechanics-core/` is
partially rewired; the workspace does not compile. This round
picks up from the partial working tree, finishes the rewire,
brings the workspace to green, and commits — without re-reading
the entire design corpus.

## References

Same as round 01. Only the relevant deltas:

- `ROADMAP.md` §Phase 1.
- `docs/design/06-execution-substrate.md` §"Schema extraction
  (settled)".
- `docs/design/13-conventions.md` §Git workflow + §Codex prompt
  archive.
- Round 01 prompt: `2026-04-20-phase-1-mechanics-config-
  extraction-01.md`.

## Context files pointed at

- Current `cargo check --workspace` output (13 errors in
  `mechanics-core`).
- `mechanics-config/src/*` (in-progress, not yet committed).
- `mechanics-core/src/internal/http/{mod.rs,wrappers.rs,
  headers.rs}` (newly written) and `runtime.rs` (needs import
  updates).

## Outcome

_(To be filled in after the round completes.)_

---

## Prompt (verbatim)

<task>
Resume Phase 1 of the Philharmonic workspace roadmap. A previous Codex session wrote most of the `mechanics-config` extraction but terminated mid-task when it hit the session's cumulative token ceiling. Partial work is on disk and uncommitted.

Repository: `/home/mori/philharmonic` — a Rust Cargo workspace of 23 submodules on `github.com/metastable-void/*`. The parent `Cargo.toml` uses `[patch.crates-io]` to redirect Philharmonic crate deps to local paths.

Authoritative references (read the relevant sections; don't re-read the whole corpus):
- `ROADMAP.md` §Phase 1 — goals, target versions, acceptance criteria.
- `docs/design/06-execution-substrate.md` §"Schema extraction (settled)" — wrapper-newtype pattern, backward-compat rationale.
- `docs/design/13-conventions.md` §Git workflow — scripts-only, signoff mandatory.

Current state (inspect first with `git -C mechanics-config status` and `git -C mechanics-core status` from the workspace root):

- `mechanics-config/` is at 0.0.0 on disk with working-tree changes: `Cargo.toml`, `CHANGELOG.md`, `src/lib.rs` modified, plus new untracked `src/config.rs`, `src/endpoint/`, `src/headers.rs`, `src/query.rs`, `src/retry.rs`, `src/template.rs`, `src/tests/`. These are the schema types extracted from `mechanics-core`. Read them to confirm they're the Boa-free shape described in the round-01 prompt; treat them as authoritative starting points (don't redo from scratch unless a specific file is broken).
- `mechanics-core/` has modified `src/internal/http/mod.rs`, `src/internal/http/headers.rs`, plus a new untracked `src/internal/http/wrappers.rs` (the Boa GC newtypes over the mechanics-config types). The rest of `mechanics-core`'s source still references the old in-crate types and does not compile.
- No commits yet in either submodule. Parent workspace has `Cargo.lock` modified and the two submodule pointers dirty.
- `cargo check --workspace` from the workspace root currently fails with ~13 errors — start by running it to see the current state.

Target end state (same as round 01):
- `mechanics-config` at 0.1.0 on disk, compiling standalone, no Boa / reqwest / tokio in its dependency tree.
- `mechanics-core` at 0.2.3 on disk, consuming `mechanics-config = "0.1.0"`, wrapping the extracted types with Boa GC newtypes via `#[unsafe_ignore_trace]`, and re-exporting every type that was previously reachable via `mechanics_core::endpoint::*` or `mechanics_core::job::MechanicsConfig` (backward-compatible — the 0.2.2 public API must keep working unchanged).
- Workspace cleanly passes: `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.
- `cargo tree -p mechanics-config | grep -iE 'boa|reqwest|tokio'` produces no output.
- Changes committed via `scripts/commit-all.sh` (which signs off with `-s` and walks submodules then parent). Do NOT push. Do NOT publish.

Finish-up work to focus on (not exhaustive — discover from cargo output):
1. Fix `mechanics-core/src/internal/runtime.rs` (and any other internal callers) so they use the new wrapper newtypes or the re-exported paths from `mechanics_config`. The round-01 errors clustered there (`E0282`, `E0308`, `E0432`).
2. Ensure `mechanics-core/src/lib.rs` re-exports every type that used to live at `mechanics_core::endpoint::*` (method, HttpEndpoint, UrlParamSpec, QuerySpec, SlottedQueryMode, EndpointBodyType, EndpointRetryPolicy) and `mechanics_core::job::MechanicsConfig` via `pub use mechanics_config::...`.
3. Ensure `as_reqwest_method` exists somewhere in `mechanics-core` as either a free function or local extension-trait method (remember: inherent impls on the foreign `HttpMethod` enum are illegal in Rust).
4. Update `mechanics-core/Cargo.toml` version to 0.2.3, add `mechanics-config = "0.1.0"` as a dependency, and append a 0.2.3 CHANGELOG entry. Check that `mechanics-config/Cargo.toml` version is 0.1.0 and its CHANGELOG has a 0.1.0 entry.
5. Run all four verification commands above; iterate until green.

Git workflow (hard constraints):
- All commits go through `scripts/commit-all.sh "<message>"`. No raw `git commit`. No `git push`. No `cargo publish`.
- `scripts/commit-all.sh` walks submodules then parent in the right order and signs every commit with `-s`.
- Keep scope tight: only `mechanics-config/` and `mechanics-core/` content changes. The parent commit bumps those two submodule pointers and updates `Cargo.lock`; nothing else in the parent.

If you encounter anything unexpected (additional unfamiliar modifications, files you don't recognize from round 01's scope), STOP and report rather than mutating state.
</task>

<structured_output_contract>
Return:
1. One-paragraph summary of what round 02 added on top of round 01's partial state.
2. Touched/new files grouped by crate. Separate "completed in round 01 and verified" from "changed in round 02."
3. Verification results: exact command + exit status + short summary for `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and `cargo tree -p mechanics-config | grep -iE 'boa|reqwest|tokio'`.
4. Residual risks or follow-ups (test gaps, anything you resolved by interpretation of ambiguous design docs, any re-export you're unsure preserves backward compat).
5. Git state: list of commits created per submodule and in the parent, with SHAs. Confirm NOTHING has been pushed.
</structured_output_contract>

<default_follow_through_policy>
Default to the most reasonable low-risk interpretation consistent with the design docs and keep going.
Stop only when a missing detail changes correctness or requires a design-level decision the docs don't cover.
</default_follow_through_policy>

<completeness_contract>
Resolve the task fully before stopping.
- `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` must all exit 0.
- `mechanics-config` must have no Boa / reqwest / tokio in its dependency tree.
- Existing consumers of `mechanics_core::endpoint::*` and `mechanics_core::job::MechanicsConfig` must build unchanged.
- Commits exist in both submodules and the parent.
</completeness_contract>

<verification_loop>
After applying changes, run the four verification commands listed in `<task>` from the workspace root. If any fails, revise and re-run instead of reporting the first draft.
</verification_loop>

<missing_context_gating>
Do not guess. Start by running `git status`, `git -C mechanics-config status`, `git -C mechanics-core status`, and `cargo check --workspace` to see the current state — the working tree is the source of truth for where round 01 left off. If a design-doc ambiguity forces an interpretive choice, note it as a "resolved-by-interpretation" item in the residual-risks section.
</missing_context_gating>

<action_safety>
Scope is exactly `mechanics-config/` and `mechanics-core/` (plus auto submodule-pointer bumps + `Cargo.lock` in the parent). Nothing else.
All commits via `scripts/commit-all.sh`. Never raw `git commit`. Never `scripts/push-all.sh`. Never `cargo publish`.
If you see unfamiliar modifications (anything that doesn't look like round-01 partial work or expected round-02 follow-up), STOP and report.
</action_safety>

<progress_updates>
Keep updates brief and outcome-based. Mention phase transitions (runtime.rs compiling, re-exports wired, tests green, verification clean) or blockers. Skip play-by-play.
</progress_updates>
