# Generic `vendor-upstream` xtask + h3-quinn vendor (round 01)

**Date:** 2026-05-14 (JST)
**Slug:** `vendor-upstream-h3-quinn`
**Round:** 01 — initial dispatch.
**Subagent:** `codex:codex-rescue`

## Motivation

`deny.toml` carries one remaining wrapper exception in the
`ring` ban:

```toml
{ crate = "ring", wrappers = ["quinn-proto"] },
```

The reason is an upstream feature-unification bug in
`h3-quinn 0.0.10`: it depends on `quinn = "..."` without
`default-features = false`, so quinn's default features
(including `rustls-ring`) leak into the workspace dep tree via
cargo's feature unification — even though `mechanics-http-client`
and `mechanics-http-server` both explicitly select
`rustls-aws-lc-rs`. The bridge is `quinn-proto/rustls-ring` →
`ring`. The wrapper currently makes this passable; the goal is
to **eliminate the wrapper entirely**, by vendoring h3-quinn
under a workspace-internal name with the quinn dep patched.

Yuka's HUMANS.md §"h3-quinn should be vendored" specifies the
shape:

> h3-quinn should be vendored - write an xtask/script that
> copies the latest release behind the 3d cooldown into a
> non-submodule in-tree crate, and applies a Cargo.toml
> patches.
>
> patch.crates-io should point to this for h3-quinn, or use our
> own name (mechanics-h3-quinn or so), published at crates.io.

Yuka clarified 2026-05-14: **renamed, but in-tree and not
submoduled.** So the vendored crate goes at
`<workspace-root>/mechanics-h3-quinn/`, is listed in
`[workspace] members`, is NOT a git submodule, and is
publishable to crates.io (no `publish = false` line — the
default `publish = true` applies; a vendored fork that can't
go to crates.io would block downstream publishes of
`mechanics-http-client` / `mechanics-http-server`).
Publication is Claude's responsibility (via
`./scripts/publish-crate.sh`) at the right moment; Codex
never invokes `publish-crate.sh`. This round's mandate stops
at landing the in-tree code in a verifiably-publishable state.
Consumers (`mechanics-http-client`, `mechanics-http-server`)
reference `mechanics-h3-quinn` via path + version so the
consumer Cargo.toml works both for in-workspace `cargo check`
(path resolves to the local crate) and for crates.io
publication (path is stripped at publish time, version
stays):

```toml
[dependencies]
h3-quinn = { package = "mechanics-h3-quinn", path = "../mechanics-h3-quinn", default-features = false, features = [], optional = true }
```

so consumer `src/` keeps writing `use h3_quinn::*;` unchanged.

Two deliverables in this one round:

1. **Generic `vendor-upstream` xtask bin** that vendors any
   crates.io crate into an in-tree path, respecting a 3-day
   release-age cooldown (supply-chain mitigation; matches the
   workspace's Menhera-proxy `/3d/` cooldown index posture for
   build-time deps).
2. **First consumer: `mechanics-h3-quinn`** — vendored from
   upstream `h3-quinn 0.0.10`, with a hand-maintained
   `Cargo.toml` that renames the crate and sets
   `quinn = { default-features = false, features = [...] }` to
   drop the `rustls-ring` default.

Acceptance: after this round, `deny.toml`'s `ring` entry
becomes a **no-wrapper full ban** (matching
`native-tls`/`rustls-platform-verifier`/`rustls-native-certs`).
`cargo tree --workspace --invert ring` prints nothing.

## References (read in this order)

1. `HUMANS.md` §"h3-quinn should be vendored" — the human-
   developer's specification of this task. Authoritative on
   intent.
2. `CONTRIBUTING.md`:
   - **§3.1** `[profile.release]` block (the new vendored
     crate's Cargo.toml must include the canonical block).
   - **§4** Git workflow — `scripts/commit-all.sh` only, no
     gitwrite from Codex (per round-03 D24 discipline; see
     below).
   - **§5** Script wrappers + `CARGO_TARGET_DIR` for every
     cargo invocation.
   - **§6** POSIX shell.
   - **§7** External-tool wrappers — `ureq` (workspace tooling)
     is the right HTTP client for the vendor-upstream xtask
     bin (NOT `reqwest`).
   - **§8** xtask in-tree workspace tooling — the bin lives
     under `xtask/src/bin/vendor-upstream.rs`.
   - **§10.3** No panics in library code; xtask bins are exempt
     but should still use `Result` flow rather than
     `.unwrap()`/`.expect()` on reachable paths.
   - **§10.9** HTTP client stack split (xtask uses ureq).
   - **§11** pre-landing.sh + pre-landing.sh --xtask.
3. `deny.toml` — current `[bans]` block with the `ring`
   wrapper. The acceptance is removing the wrapper.
4. `mechanics-http-client/Cargo.toml` + `mechanics-http-server/Cargo.toml`
   — current consumers of `h3-quinn`. After this round, they
   reference `mechanics-h3-quinn` via the cargo `package`
   rename.
5. `xtask/src/bin/` — existing xtask bins as shape templates.
   Especially `crates-io-versions.rs` (uses
   `xtask::http::fetch_text`), `gen-uuid.rs`, `web-fetch.rs`.
6. `xtask/src/http.rs` (or similar) — workspace's `ureq +
   rustls-no-provider + rustls-webpki-roots + aws-lc-rs`
   plumbing.
7. `docs/codex-prompts/2026-05-14-0001-d24-default-features-audit-03.md`
   — for the Codex-no-gitwrite discipline (carried forward
   verbatim).

## Goal — high level

**Part A: `vendor-upstream` xtask bin** (generic framework)

A new xtask bin at `xtask/src/bin/vendor-upstream.rs` that:

1. Reads a manifest file at `vendor/vendor.toml` (relative to
   the workspace root) describing what to vendor:
   ```toml
   # vendor/vendor.toml
   # Each `[[entry]]` describes one vendored crate.
   [[entry]]
   # The upstream crates.io crate name + version to vendor.
   upstream_name = "h3-quinn"
   upstream_version = "0.0.10"
   # The target path (relative to workspace root) where the
   # vendored source goes. Must already exist as a workspace
   # member (manually added to `[workspace] members` in the
   # root Cargo.toml); the bin only writes into it, not the
   # workspace root.
   target_path = "mechanics-h3-quinn"
   # Files / directories to sync from the upstream tarball.
   # Globs allowed. Anything in `target_path/` NOT listed here
   # is preserved (e.g., the hand-maintained Cargo.toml).
   sync = [
       "src/**/*.rs",
       "LICENSE",
       "LICENSE-MIT",
       "LICENSE-APACHE",
       "README.md",
   ]
   # Optional human-readable reason for vendoring.
   reason = "Drop quinn's `rustls-ring` default to eliminate `ring` from the dep tree."
   ```

2. For each `[[entry]]`:
   - Query crates.io's index for the crate's published-version
     metadata, including `created_at` (release timestamp).
   - Refuse to vendor any version that's less than 3 days old
     (the cooldown). If the manifest pins a version younger
     than 3 days, fail with a clear error.
   - Download the crate's `.crate` tarball from
     `https://static.crates.io/crates/<name>/<name>-<version>.crate`
     using the workspace's `ureq` HTTP plumbing
     (`rustls-no-provider` + `aws-lc-rs` provider install in
     main — match the pattern from existing xtask bins).
   - Verify the tarball's checksum against the crates.io
     index's `sha256` (sparse index lookup at
     `https://static.crates.io/index/<...>` or via the
     workspace's existing Menhera proxy — Codex picks the
     simplest available route, documents in the bin's
     header comment).
   - Extract to a temp dir, locate the inner
     `<name>-<version>/` directory, then for each `sync` glob,
     copy matching files into `target_path/` (preserving
     directory structure).
   - **Never touch** files in `target_path/` that don't match a
     `sync` glob. In particular, **never overwrite** an
     existing `target_path/Cargo.toml` (the hand-maintained
     one).
   - Emit a manifest sidecar at
     `target_path/.vendor-stamp.toml`:
     ```toml
     upstream_name = "h3-quinn"
     upstream_version = "0.0.10"
     upstream_sha256 = "<hex>"
     vendored_at = "2026-05-14T<HH:MM:SS>Z"
     vendor_tool = "vendor-upstream"
     ```
     so future re-runs can detect "no change since last vendor".
   - Print a per-entry summary: files copied, files unchanged,
     bytes total.

3. CLI shape:
   - `./scripts/xtask.sh vendor-upstream` — process every
     `[[entry]]` in `vendor/vendor.toml`.
   - `./scripts/xtask.sh vendor-upstream -- --entry h3-quinn` —
     process only the entry whose `upstream_name` is
     `h3-quinn` (or `target_path` matches `h3-quinn`; Codex
     picks the clearer key).
   - `./scripts/xtask.sh vendor-upstream -- --check` — read-
     only; report whether each entry is up-to-date vs. its
     `.vendor-stamp.toml` without writing anything.

4. Add an entry to `scripts/xtask.sh`'s help/dispatch table
   (the bin should be auto-discoverable per existing
   convention).

5. Tests — at minimum:
   - Manifest parsing (valid + invalid TOML).
   - Cooldown check (mock release timestamps).
   - Sync glob matching (fixture tarball under
     `xtask/tests/fixtures/`).
   - "Cargo.toml not overwritten" invariant.

   Codex picks the right granularity; unit tests under
   `xtask/src/bin/vendor_upstream/` or in-bin
   `#[cfg(test)] mod tests` is fine. The framework is small
   enough that integration tests against a real crates.io
   download aren't needed — mock the HTTP layer.

**Part B: First consumer: `mechanics-h3-quinn`**

1. Create the workspace-internal crate at
   `<workspace-root>/mechanics-h3-quinn/`:
   - `Cargo.toml` (hand-maintained):
     - `name = "mechanics-h3-quinn"`
     - `version = "0.0.10"` (mirror the upstream version for
       clarity).
     - `edition = "2024"` (workspace standard, even though
       upstream uses `2021`)
     - `rust-version = "1.88"` (workspace standard)
     - `description = "Vendored fork of h3-quinn 0.0.10 with quinn's rustls-ring default dropped — eliminates ring from the workspace dep tree."`
     - `license = "MIT"` (upstream is MIT-only)
     - `authors`, `repository`, `homepage` — match upstream's
       attribution (vendored credit).
     - **No `publish = false` line.** The default
       `publish = true` applies. Publication is Claude's
       responsibility via `./scripts/publish-crate.sh` —
       out of this round's scope. Codex never invokes
       `publish-crate.sh`.
     - `[profile.release]` canonical block per CONTRIBUTING.md
       §3.1.
     - `[dependencies]` — copy from upstream's
       `Cargo.toml.orig` (look at
       `~/.cargo/registry/src/.../h3-quinn-0.0.10/Cargo.toml.orig`
       or the registry's normalized `Cargo.toml`), with one
       targeted patch: the `quinn` dep gets `default-features
       = false` and an explicit feature list of what the
       crate actually uses (read upstream's `src/` to
       determine; typically `["runtime-tokio",
       "rustls-aws-lc-rs"]` to keep the aws-lc-rs path).
     - **Apply D24 discipline**: every direct dep here gets
       `default-features = false` with a grep-narrowed
       explicit feature list. Inline `# kept: <reason>`
       comments where appropriate.
     - `[features]` — mirror upstream's feature gates
       (`datagram` etc.) so the rename is a true drop-in.
   - `src/lib.rs` (and any other src files) — vendored from
     upstream by the `vendor-upstream` xtask bin (Part A above).
     **Do not hand-edit `src/`.**
   - `LICENSE`, `README.md` — vendored from upstream by the
     bin.
   - `.vendor-stamp.toml` — written by the bin.

2. Add `mechanics-h3-quinn` to the workspace root `Cargo.toml`:
   - `[workspace] members` list: append
     `"mechanics-h3-quinn"`.
   - `[patch.crates-io]` block: add `mechanics-h3-quinn = {
     path = "mechanics-h3-quinn" }`. **Not strictly necessary**
     (since no other crate names it via crates.io), but
     consistent with the workspace pattern; Codex may skip if
     consistency-with-pattern doesn't apply.

3. Update consumers:
   - `mechanics-http-client/Cargo.toml`:
     ```toml
     h3-quinn = { package = "mechanics-h3-quinn", version = "0.0.10", default-features = false, features = [], optional = true }
     ```
     The cargo `package` rename keeps the consumer-side import
     name unchanged (`use h3_quinn::*` continues to work).
   - `mechanics-http-server/Cargo.toml`:
     Same shape.

4. Update `deny.toml`:
   - Change the `ring` entry from
     `{ crate = "ring", wrappers = ["quinn-proto"] }` to a
     plain `"ring",` (no-wrapper full ban), matching
     `native-tls`, `rustls-platform-verifier`,
     `rustls-native-certs`.
   - Update the surrounding comment to note that the wrapper
     exception was eliminated 2026-05-14 via h3-quinn
     vendoring under `mechanics-h3-quinn`.

5. **Run the `vendor-upstream` xtask bin** (after writing it)
   to populate `mechanics-h3-quinn/src/`,
   `mechanics-h3-quinn/LICENSE`, `mechanics-h3-quinn/README.md`,
   and `mechanics-h3-quinn/.vendor-stamp.toml`. The Cargo.toml
   stays hand-maintained.

6. Run `CARGO_TARGET_DIR=target-main cargo check -p
   mechanics-h3-quinn --all-targets`, then `-p
   mechanics-http-client --all-targets`, then `-p
   mechanics-http-server --all-targets`. All must pass.

7. Run `CARGO_TARGET_DIR=target-main cargo tree --workspace
   --invert ring -e all --target all` — must print nothing
   (or "package not found in the dependency graph").

8. Run `CARGO_TARGET_DIR=target-main cargo deny check bans` —
   must end with `bans ok`.

## Non-goals (explicit)

- **No publishing during this Codex round.** Codex does not
  invoke `publish-crate.sh`. The vendored crate is intended
  for crates.io publication; Claude publishes it
  post-Codex / post-commit / post-verification.
- **No version bumps** on `mechanics-http-client` /
  `mechanics-http-server` for this round — per Yuka's
  2026-05-14 rule "we don't publish crates mid-work unless
  necessary; bumps are not always strictly necessary because
  of it." mhc and mhs already had their D24 bumps land today
  (0.2.2 / 0.1.1); those publishes haven't happened yet and
  this round's edits roll into them when published.
- **No submodule creation** for `mechanics-h3-quinn`. In-tree
  only.
- **No upstream-fork PRs** or anything that touches the
  upstream `h3-quinn` repo.
- **No quinn-fork** — only h3-quinn's dep declaration changes,
  not quinn itself. Quinn already supports
  `default-features = false, features = ["rustls-aws-lc-rs"]`
  cleanly; the workspace just couldn't *propagate* that
  through h3-quinn's unconstrained quinn dep.
- **No HTTP/3 functional changes** — the vendor is a
  byte-for-byte source copy. Behaviour must be identical to
  upstream h3-quinn 0.0.10.
- **No new external HTTP clients** in xtask — `ureq` only.
- **No reqwest** anywhere.
- **No `[features]` redesign** in mhc / mhs.

## Commit discipline (binding — Codex does NOT commit)

Per `CLAUDE.md` and the D24 round 03 discipline:

> Codex itself never runs `commit-all.sh` (including
> `--dry-run` and `--exclude`); the codex-guard in the script
> aborts under any Codex ancestor process.

So for every edit Codex makes:

1. Edit the file in the working tree.
2. Run per-crate `CARGO_TARGET_DIR=target-main cargo check -p
   <crate> --all-targets` after each crate's edits.
3. Leave everything in the dirty working tree. Do NOT call
   `./scripts/commit-all.sh`, `git commit`, `git add`,
   `git push`, `git stash`, or any other gitwrite. Read-only
   git calls (`git status`, `git diff`, `git log`) are fine.
4. At the end: run `./scripts/status.sh` (read-only) and
   report the dirty tree. **Do NOT run `pre-landing.sh`** —
   Claude runs it after committing.

Claude will commit + push + run `pre-landing.sh` +
`pre-landing.sh --xtask` + `cargo deny check bans` + banned-
dep `cargo tree --invert` after Codex returns.

## Concrete tasks (sequencing)

Recommended order (Codex picks within reason):

1. **Workspace plumbing first** (cheap):
   - Add `mechanics-h3-quinn` to `[workspace] members`.
   - Hand-write `mechanics-h3-quinn/Cargo.toml` with the
     dep-list adapted from upstream's `Cargo.toml.orig` and
     the quinn-dep patch.
   - Add `mechanics-h3-quinn/.gitignore` if needed (probably
     not — `target-main/` is workspace-shared).
   - At this point, `cargo metadata` should succeed for the
     workspace; mechanics-h3-quinn is a member with no src/
     yet, which `cargo check` will fail. That's expected
     until step 3.
2. **Write the `vendor-upstream` xtask bin** (Part A).
   Includes:
   - The bin's main file at
     `xtask/src/bin/vendor-upstream.rs`.
   - Helper modules under `xtask/src/bin/vendor_upstream/` if
     needed for organisation.
   - Tests as appropriate.
   - Dependency-block additions in `xtask/Cargo.toml` if any
     new deps are needed (use `default-features = false` per
     D24 audit discipline).
   - `vendor/vendor.toml` initial manifest with the single
     h3-quinn entry.
   - `./scripts/xtask.sh vendor-upstream -- --check` should
     run cleanly even before any actual vendoring.
3. **Run the bin** to populate
   `mechanics-h3-quinn/{src/, LICENSE, README.md,
   .vendor-stamp.toml}`. After this, `mechanics-h3-quinn`
   should `cargo check` cleanly.
4. **Wire consumers**:
   - Update `mechanics-http-client/Cargo.toml` to use the
     `package = "mechanics-h3-quinn"` rename trick.
   - Update `mechanics-http-server/Cargo.toml` similarly.
   - Run per-crate `cargo check -p mechanics-http-client
     --all-targets` + `cargo check -p mechanics-http-server
     --all-targets`. Both must pass.
5. **Banned-dep verification**:
   - `cargo tree --workspace --invert ring -e all --target
     all` must print nothing (or the equivalent
     "package not found").
   - If it still shows ring, the quinn-feature-list in
     `mechanics-h3-quinn/Cargo.toml` is wrong — iterate.
6. **Update `deny.toml`**:
   - Change the ring entry from wrappered to no-wrapper full
     ban.
   - Update the surrounding comment to record the elimination.
7. **Run `cargo deny check bans`** — must end with `bans ok`.

## Verification checklist (before declaring done)

- [ ] `vendor-upstream` xtask bin exists at
      `xtask/src/bin/vendor-upstream.rs`.
- [ ] `vendor/vendor.toml` exists with at least the h3-quinn
      entry.
- [ ] `mechanics-h3-quinn/` exists with:
      - hand-written `Cargo.toml` (`publish = false`, quinn
        dep patched).
      - vendored `src/`, `LICENSE`, `README.md`.
      - `.vendor-stamp.toml`.
- [ ] `Cargo.toml` (workspace root) has `mechanics-h3-quinn`
      in `[workspace] members`.
- [ ] `mechanics-http-client/Cargo.toml` and
      `mechanics-http-server/Cargo.toml` use the
      `package = "mechanics-h3-quinn"` rename.
- [ ] Consumer `src/` is unchanged (`use h3_quinn::*` continues
      to work).
- [ ] `cargo check -p mechanics-h3-quinn`: PASS.
- [ ] `cargo check -p mechanics-http-client`: PASS.
- [ ] `cargo check -p mechanics-http-server`: PASS.
- [ ] `cargo tree --workspace --invert ring`: empty / not in
      tree.
- [ ] `deny.toml`: `ring` is a no-wrapper full ban; comment
      updated.
- [ ] `cargo deny check bans`: PASS (`bans ok`).
- [ ] `./scripts/status.sh` shows the dirty tree (Codex
      doesn't commit).
- [ ] **NO commits made** by Codex. **NO `pre-landing.sh`**
      run by Codex.

## Outcome

Pending — will be updated after Codex round 01 run.

---

<task>
Build a generic `vendor-upstream` xtask bin and use it to
eliminate the last remaining `ring` wrapper exception in
`deny.toml`, by vendoring upstream `h3-quinn 0.0.10` into an
in-tree non-submodule workspace member `mechanics-h3-quinn`
with a patched quinn dep.

**Authoritative references (read first; the prompt above
elaborates):**

1. `HUMANS.md` §"h3-quinn should be vendored" — Yuka's spec.
2. `CONTRIBUTING.md` §§3.1, 4, 5, 6, 7, 8, 10.3, 10.9, 11.
3. `deny.toml` — current `[bans]` block.
4. `mechanics-http-client/Cargo.toml`,
   `mechanics-http-server/Cargo.toml` — h3-quinn consumers.
5. `xtask/src/bin/*.rs` and `xtask/src/http.rs` — existing
   patterns for ureq HTTP + crates.io interactions.
6. `~/.cargo/registry/src/.../h3-quinn-0.0.10/Cargo.toml.orig`
   — the upstream Cargo.toml shape before crates.io
   normalisation (use this as the source of truth for the
   hand-maintained `mechanics-h3-quinn/Cargo.toml`'s dep list).

**Two deliverables in one round:**

**Part A — `xtask/src/bin/vendor-upstream.rs`** (generic
framework):
- Reads `vendor/vendor.toml` (workspace-root-relative).
- Per `[[entry]]`: download upstream tarball from
  `https://static.crates.io/crates/<name>/<name>-<version>.crate`
  via the workspace's `ureq + rustls-no-provider + aws-lc-rs`
  pattern, verify SHA-256 against the crates.io sparse index,
  enforce ≥3-day-old release cooldown (refuse younger), extract
  to temp dir, sync the entry's `sync = [...]` globs into
  `target_path/` (preserving existing files like
  hand-maintained `Cargo.toml`), write
  `target_path/.vendor-stamp.toml` recording the vendor event.
- CLI: bare (`./scripts/xtask.sh vendor-upstream`) processes
  all entries; `--entry <upstream_name>` narrows;
  `--check` is read-only.
- Tests covering manifest parsing, cooldown check, sync-glob
  matching, "Cargo.toml not overwritten" invariant.

**Part B — `mechanics-h3-quinn` first consumer:**
- New workspace member at `mechanics-h3-quinn/` (NOT a git
  submodule, `publish = false`).
- Hand-written `Cargo.toml` adapted from upstream
  `h3-quinn 0.0.10`'s `Cargo.toml.orig` with one targeted
  patch: `quinn = { default-features = false, features =
  ["runtime-tokio", "rustls-aws-lc-rs"] }` (verify the
  features list against what h3-quinn's `src/` actually uses
  of quinn).
- Apply D24 discipline (every direct dep with
  `default-features = false` + explicit feature list).
- `src/`, `LICENSE`, `README.md` vendored from upstream by
  the `vendor-upstream` bin (Part A above) — do NOT hand-write
  these.
- Consumers (`mechanics-http-client`, `mechanics-http-server`):
  change their `h3-quinn` dep to use the cargo `package`
  rename:
  ```toml
  h3-quinn = { package = "mechanics-h3-quinn", version = "0.0.10", default-features = false, features = [], optional = true }
  ```
  Their `src/` stays unchanged (`use h3_quinn::*` still
  resolves).
- Update `deny.toml` to change the `ring` entry from
  wrapper-allowed to no-wrapper full ban; update the
  surrounding comment to record the elimination.

**Per-crate verification required:**

- `CARGO_TARGET_DIR=target-main cargo check -p
  mechanics-h3-quinn --all-targets`: PASS.
- `cargo check -p mechanics-http-client --all-targets`: PASS.
- `cargo check -p mechanics-http-server --all-targets`: PASS.
- `cargo tree --workspace --invert ring -e all --target all`:
  empty (or "not found").
- `cargo deny check bans`: PASS.

**Hard rules:**

<action_safety>
- **Codex does NOT commit.** No `./scripts/commit-all.sh`, no
  `git commit`, no `git add`, no `git push`, no `git stash`,
  no other gitwrite. Read-only `git status` / `git diff` /
  `git log` are fine. Leave everything in the dirty working
  tree.
- **Codex does NOT run `pre-landing.sh`.** Claude runs it
  post-commit.
- Every cargo invocation needs `CARGO_TARGET_DIR=target-main`
  (or `target-xtask` for xtask-side; `xtask/src/bin/` builds
  under `target-xtask/`).
- POSIX-ish host. No bash-only constructs in shell.
- `./scripts/xtask.sh calendar-jp` at session start and again
  before returning, to ground the JST timestamp. If JST is
  outside regular hours (10:00–19:00, ext 21:00), add a
  one-line "(JST now HH:MM <day> — outside regular hours;
  proceeding.)" note in the final reply.
- The Codex sandbox's `.git` mount may be read-only; treat
  any "read-only filesystem" error on git as the expected
  gitwrite-forbidden state, not a problem to fix.
- xtask uses `ureq + rustls-no-provider +
  rustls-webpki-roots + aws-lc-rs`. Match the existing
  xtask::http pattern (look at `crates-io-versions.rs` or
  `web-fetch.rs` for the right idiom). Do NOT introduce
  reqwest in xtask, do NOT use any other crypto provider.
- POSIX shell for any shell wrapper. xtask bins are Rust.
</action_safety>

<missing_context_gating>
Before starting, run `./scripts/status.sh` — it should print
the parent-clean state plus the dirty mechanics-core (the
intl-block doc note that's about to land via Claude's next
commit; if Claude has already pushed it, parent may be clean).
If the workspace is mid-flight on unrelated dirty work that
isn't part of this dispatch's mandate, STOP and report the
divergence.

Read `~/.cargo/registry/src/index.crates.menhera.org-*/h3-quinn-0.0.10/Cargo.toml.orig`
to confirm the upstream dep list and feature shape. If that
file doesn't exist locally, run `cargo fetch -p h3-quinn` (or
similar) first.

Read upstream `h3-quinn`'s `src/lib.rs` to determine which
quinn features the crate actually uses. Look for `use
quinn::...` patterns and any quinn-feature-gated APIs the
crate calls.
</missing_context_gating>

<default_follow_through_policy>
Land both Part A (the bin) and Part B (mechanics-h3-quinn + the
two consumer rewires + deny.toml change) in this single round.
Don't stop after Part A and report "Part B pending". The two
are entangled (Part B requires Part A to populate src/), and
the value proposition is Part B's elimination of the ring
wrapper.

If a hard blocker surfaces (e.g., upstream h3-quinn 0.0.10
uses a quinn feature whose disablement breaks compilation in a
way that can't be resolved by widening the explicit feature
list), stop, document the blocker, and report INCOMPLETE.
Don't paper over with a wrapper-still-needed compromise.
</default_follow_through_policy>

<completeness_contract>
"Complete" means all of:

1. `xtask/src/bin/vendor-upstream.rs` exists, builds, tests
   pass.
2. `vendor/vendor.toml` exists with the h3-quinn entry.
3. `mechanics-h3-quinn/` exists with hand-written Cargo.toml +
   vendored `src/`, `LICENSE`, `README.md`,
   `.vendor-stamp.toml`.
4. Workspace root `Cargo.toml` lists `mechanics-h3-quinn` in
   `[workspace] members`.
5. `mechanics-http-client/Cargo.toml` and
   `mechanics-http-server/Cargo.toml` use the `package = "mechanics-h3-quinn"`
   rename.
6. `deny.toml` `ring` is a no-wrapper full ban with updated
   comment.
7. All per-crate `cargo check -p <crate>` PASS.
8. `cargo tree --workspace --invert ring`: empty.
9. `cargo deny check bans`: PASS.
10. `./scripts/status.sh` shows the dirty tree (no commits
    made).
11. `## Outcome` section of this prompt file updated with a
    paragraph summarising what landed, residual risks, and
    verification results.

If any of 1–9 is incomplete, report INCOMPLETE clearly with
what's done and what's left.
</completeness_contract>

<structured_output_contract>
At end of round 01, return:

1. **Summary** (2–3 sentences): what landed; whether the ring
   wrapper was eliminated.
2. **Touched files**: grouped by category (xtask bin / vendor
   manifest / mechanics-h3-quinn / consumers / deny.toml).
3. **vendor-upstream CLI shape**: brief usage doc that Codex
   wrote into the bin's `--help` output.
4. **Quinn feature list arrived at**: which features
   mechanics-h3-quinn's Cargo.toml enables on quinn, and the
   src/ evidence that supports each.
5. **Verification results**:
   - per-crate cargo check PASS list.
   - `cargo tree --invert ring`: empty / non-empty.
   - `cargo deny check bans`: PASS / FAIL.
6. **Residual risks**:
   - Any feature kept on quinn that's only there to satisfy
     compilation (with src/ evidence).
   - Any deviation from the prompt's intent that Codex made
     a judgment call on.
   - Anything that could regress on a future upstream h3-quinn
     release (e.g., new feature surface that the vendor would
     need to know about).
7. **Git state**: `./scripts/status.sh` output. **NO commits
   made.**
8. **Outcome paragraph** suitable for dropping into this
   prompt file's `## Outcome` section.
</structured_output_contract>
</task>
