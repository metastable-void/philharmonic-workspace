# Missing tests batch A: install, webui, reload, config merge (initial dispatch)

**Date:** 2026-05-01
**Slug:** `missing-tests-batch-a`
**Round:** 01 (initial dispatch)
**Subagent:** `codex:codex-rescue`

## Motivation

Four items from the remaining-TODOs list (#6, #8, #11, #12) need
unit tests in the `philharmonic` meta-crate's `server` and `webui`
modules. These are all library-level tests that run without
external dependencies.

## References

- `docs/notes-to-humans/2026-05-01-0001-remaining-todos.md` items 6, 8, 11, 12
- `philharmonic/src/server/install.rs` — `InstallPlan` + `execute_install`
- `philharmonic/src/server/config.rs` — `load_config` + drop-in merge (existing tests at line 107+)
- `philharmonic/src/server/reload.rs` — `ReloadHandle` (existing SIGHUP test at line 104+)
- `philharmonic/src/webui.rs` — `webui_fallback` + `serve_asset`

## Context files pointed at

- `philharmonic/src/server/install.rs`
- `philharmonic/src/server/config.rs`
- `philharmonic/src/server/reload.rs`
- `philharmonic/src/webui.rs`

## Outcome

Pending — will be updated after Codex run.

---

## Prompt (verbatim)

<task>
Add unit tests for four modules in the `philharmonic` meta-crate.
All tests go in existing `#[cfg(test)] mod tests` blocks or as new
test modules in the same file. No new files needed.

## 1. Install subcommand (item #6)

File: `philharmonic/src/server/install.rs`

The `execute_install` function requires root and does filesystem
operations, so don't test the full function. Instead test the
systemd unit template generation. Find the function that builds the
systemd unit content (it's a `fn systemd_unit(...)` or similar
helper) and test that:

- The generated unit contains `Description=<plan.description>`
- The generated unit contains `ExecStart=<bin_path> serve`
- The generated unit has `[Service]` and `[Install]` sections

If the template function is private, add a `#[cfg(test)]` test
that constructs an `InstallPlan` with known values and calls the
template function directly. Read the file to find the exact
function name.

## 2. WebUI module (item #8)

File: `philharmonic/src/webui.rs`

Test `webui_fallback`:
- Request for `/main.js` → response with `application/javascript`
  content type (if the asset exists in the embedded files)
- Request for `/nonexistent-path` → response serves `index.html`
  (SPA fallback)
- Request for `/main.css` → response with `text/css` content type

Since `Assets` uses `rust-embed` with `#[folder = "webui/dist/"]`,
the tests need the dist files to exist at compile time. The files
are committed, so this works. Use `axum::body::to_bytes` to read
the response body if needed.

Also test `mime_for`:
- `.html` → `text/html; charset=utf-8`
- `.js` → `application/javascript; charset=utf-8`
- `.css` → `text/css; charset=utf-8`
- `.svg` → `image/svg+xml`
- `.map` → `application/json`
- unknown → `application/octet-stream`

The `mime_for` and `serve_asset` functions are private. Either
make them `pub(crate)` for testing, or put the tests inside
`webui.rs` as a `#[cfg(test)] mod tests` block.

## 3. SIGHUP reload (item #11)

File: `philharmonic/src/server/reload.rs`

An `#[ignore]` test already exists (`sighup_notifies`). Add a
**non-ignored** test that verifies `ReloadHandle::clone()` behavior:

- Clone a handle
- Bump the generation manually (you'll need to expose a test-only
  method or use the existing SIGHUP test pattern)
- Both the original and clone should see the new generation

If the internals are too private to test without SIGHUP, just add
a test that verifies `ReloadHandle::new()` succeeds and
`Clone` works without panicking. Mark it `#[tokio::test]`.

## 4. TOML drop-in merge with >2 files (item #12)

File: `philharmonic/src/server/config.rs`

Existing tests cover single file, single drop-in, and missing
cases. Add a test with **3 drop-in files** that verifies
lexicographic ordering:

```rust
#[test]
fn drop_in_lexicographic_order() {
    // Create primary with name="base", port=1000
    // Create drop-in dir with:
    //   30-third.toml: port=3000
    //   10-first.toml: port=1000
    //   20-second.toml: port=2000, name="override"
    // Result should be: name="override" (from 20), port=3000 (from 30)
    // because 30-third.toml is applied last
}
```

This verifies that files are sorted by filename, not by insertion
order.

## Verification

After implementing:

1. Run `./scripts/pre-landing.sh` — it auto-detects touched crates
   and runs fmt + check + clippy (-D warnings) + test.
2. Do NOT run raw `cargo fmt/check/clippy/test` — use the scripts.
3. If `pre-landing.sh` finds issues, fix them and re-run.
4. Use `./scripts/build-status.sh` if cargo seems stuck.

## Git rules

- Commit via `./scripts/commit-all.sh "<message>"` ONLY.
- Do NOT run `./scripts/push-all.sh` or `cargo publish`.
- Do NOT run raw `git commit` / `git add` / `git push`.
</task>

<default_follow_through_policy>
If a step produces warnings, errors, or unexpected output, address
them immediately before proceeding to the next step.
</default_follow_through_policy>

<completeness_contract>
The task is complete when:
1. All four test groups are implemented
2. `./scripts/pre-landing.sh` passes cleanly
3. Changes are committed via `./scripts/commit-all.sh`
</completeness_contract>

<verification_loop>
After each significant code change:
1. Run `./scripts/pre-landing.sh`
2. If it fails, fix and re-run
3. Only commit after a clean pass
</verification_loop>

<missing_context_gating>
If you cannot find a type, method, or pattern referenced in this
prompt, grep for it before inventing alternatives.
</missing_context_gating>

<action_safety>
- Never run `./scripts/push-all.sh`
- Never run `cargo publish`
- Never run raw git commands
- Never modify files outside the scope listed above
</action_safety>

<structured_output_contract>
When done, report:
- Summary: what was implemented
- Files touched: list with brief description of changes
- Verification: pre-landing.sh output (pass/fail)
- Git state: commit SHA, branch, pushed=no
</structured_output_contract>
