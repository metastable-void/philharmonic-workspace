# D18 — mechanics-core module-surface refactor (round 01)

**Date:** 2026-05-14 (JST)
**Slug:** `d18-mechanics-module-surface`
**Round:** 01 — feature-gating refactor of existing modules +
new `console` (no-op) + new `html` (htmlize wrapper).
**Subagent:** `codex:codex-rescue`

## Motivation

D18 is the full `mechanics-core` module-surface refactor per
HUMANS.md §"MIME module at mechanics-core". The setTimeout-
removal sub-piece landed 2026-05-14 (parent `796f83e`); D18's
remaining work is the module-surface redesign.

Per the ROADMAP §3.F D18 captured scope:

- Every existing non-endpoint built-in module is moved behind
  a Cargo feature flag (`rand`, `uuid`, `encoding`).
- Four new modules join (one non-default, three default):
  `html`, `url`, `console`, `mime`.

Round 01 (this prompt) lands the **foundation**: feature-gating
the existing modules + the two simplest new modules (`console`
as a no-op; `html` as a thin htmlize wrapper). Round 02 lands
`url`. Round 03 lands `mime`. Round 04 updates the
workflow-authoring guide. Each round can stand alone — the
existing default-features set means existing consumers see no
behaviour change unless they opt-out.

## Round 01 scope

### What changes

1. **`mechanics-core/Cargo.toml`** — add a `[features]` block:
   ```toml
   [features]
   default = ["rand", "encoding", "html", "console"]
   # Existing modules, now feature-gated. Default-on so
   # downstream consumers using default features see no
   # behaviour change.
   rand = []      # mechanics:rand + mechanics:uuid
   encoding = []  # mechanics:form_urlencoded + base64 + base32 + hex
   # New modules, both default-on. Both are thin wrappers
   # / no-ops — minimal binary-size impact.
   html = []      # mechanics:html (htmlize wrapper)
   console = []  # mechanics:console (no-op, no I/O)
   ```
   `url` and `mime` are NOT listed in round 01 — they're added
   in rounds 02 and 03 respectively.

   New dep on the `htmlize` crate (workspace's
   `default-features = false` discipline applies; pick the
   minimal feature set htmlize needs for `escape_text`,
   `escape_all_quotes`, `unescape`, `unescape_attribute`).

2. **`mechanics-core/src/internal/runtime/builtins/mod.rs`** —
   the `bundle_builtin_modules` function (around line 197):
   ```rust
   pub(super) fn bundle_builtin_modules(loader: &Rc<CustomModuleLoader>, context: &mut Context) {
       // endpoint is always registered — it's the core
       // capability mechanism, not gated.
       endpoint::register(loader, context);

       #[cfg(feature = "encoding")]
       {
           form_urlencoded::register(loader, context);
           base64::register(loader, context);
           hex::register(loader, context);
           base32::register(loader, context);
       }

       #[cfg(feature = "rand")]
       {
           rand::register(loader, context);
           uuid::register(loader, context);
       }

       #[cfg(feature = "html")]
       html::register(loader, context);

       #[cfg(feature = "console")]
       console::register(loader, context);
   }
   ```

3. **`mechanics-core/src/internal/runtime/builtins/{html,console}.rs`** — new module files following
   the existing `rand.rs` / `uuid.rs` shape:

   - **`console.rs`** — registers a `mechanics:console`
     module exposing a default-export `console` object with
     methods `log`, `info`, `warn`, `error`, `debug`. **All
     methods are no-ops** that accept variadic args and
     return `undefined`. No I/O of any kind: no stdout, no
     stderr, no host-side `tracing` emission. WHATWG console
     spec compliance is limited to the method signatures and
     argument-handling shape; format-spec parsing
     (`%s`, `%d`, `%o`) can be a TODO for the future
     capture-into-response work. For round 01, the methods
     just consume args silently. Per Yuka 2026-05-14:
     workflows run in a sandboxed realm where any direct
     I/O would violate the stateless-per-job contract and
     leak host information. **Future work** (separate
     dispatch, possibly breaking): capture pre-return
     `console.*` invocations into a structured field on the
     `RunJobResponse`. Round 01 does NOT implement this
     capture; it just establishes the no-op surface so
     workflow code that uses `console.log` doesn't crash.
     Tests verify the methods exist + accept args + return
     undefined.

   - **`html.rs`** — registers a `mechanics:html` module
     wrapping `htmlize` crate's escape/unescape functions:
     - `escapeText(text: string) -> string` —
       `htmlize::escape_text` (escapes `<`, `>`, `&`).
     - `escapeAttribute(text: string) -> string` —
       `htmlize::escape_all_quotes` (escapes `<`, `>`, `&`,
       `"`, `'`).
     - `unescapeText(html: string) -> string` —
       `htmlize::unescape` (general unescape).
     - `unescapeAttribute(html: string) -> string` —
       `htmlize::unescape_attribute` (attribute-context
       unescape, leaves `<`, `>` alone per HTML spec).

     All four are pure (no I/O, no state). Type-check the
     argument as a JS string; throw a `TypeError` if not.
     Tests verify the canonical input → output mappings
     (e.g. `escapeText("a&b<c")` → `"a&amp;b&lt;c"`,
     `unescapeText("&amp;")` → `"&"`).

4. **`mechanics-core/Cargo.toml`** — add `htmlize` to
   `[dependencies]` with `default-features = false` and the
   minimal feature set the wrapper needs. Inspect the htmlize
   crate's feature surface (probably `default = []` so just
   `htmlize = "<version>"` is fine; verify).

5. **Existing test files for rand/uuid/encoding** — gate the
   `#[test]` functions / test modules with `#[cfg(feature =
   "rand")]` or `#[cfg(feature = "encoding")]` as appropriate,
   so a `cargo test -p mechanics-core --no-default-features`
   build succeeds without these modules (won't be common, but
   should compile).

6. **New tests** for `console` and `html` modules: small
   `#[test]` per the existing pattern. JS literal imports the
   module and exercises the API.

7. **`mechanics-core/CHANGELOG.md`** — add an entry under the
   current 0.5.1 / next-version section documenting:
   - New `[features]` block (default = ["rand", "encoding",
     "html", "console"]). Existing consumers using default
     features see no behaviour change.
   - New `mechanics:html` module (htmlize wrapper) under
     the `html` feature.
   - New `mechanics:console` module (no-I/O no-op) under
     the `console` feature.
   - Existing `mechanics:rand` / `mechanics:uuid` now gated
     by `rand`; existing `mechanics:form_urlencoded` /
     `:base64` / `:base32` / `:hex` now gated by `encoding`.
     All default-on; opt-out via `default-features = false`
     + explicit feature list.
   - **mechanics-core version bump**: Codex picks 0.5.2 or
     0.6.0. Recommended: bump to 0.6.0 since the new feature
     flags + new modules are a notable enough surface
     addition to mark a minor version. Default-on means no
     consumer breaks; the bump is semantic clarity, not
     compatibility-driven. Yuka's "no publish mid-work
     unless necessary" rule still applies: this bump
     batches with any future D18-rounds 02/03/04 work until
     publish-time.

### What does NOT change in round 01

- **`url` module** — round 02.
- **`mime` module** — round 03.
- **Workflow-authoring guide rewrite** — round 04.
- **`mechanics:endpoint` module** — unchanged; always
  registered, never gated. It's the core capability
  mechanism.
- **Engine-internal job queue + tail-promise polling** —
  unchanged. D17's behaviour stays.
- **Existing Rust public API** beyond the new feature
  flags themselves — unchanged.
- **`philharmonic` meta-crate's `[features]` block** —
  unchanged. Philharmonic unconditionally enables
  mechanics-core's default features, so this round doesn't
  cascade. (Future: if philharmonic wants to expose
  mechanics:mime, the meta-crate gains a feature that
  activates mechanics-core's `mime` feature; that's part of
  round 03's scope, not this round.)

### Hard constraints (binding)

- **No non-ES globals** added. The Mechanics realm surface
  stays ES-spec-only globals + `mechanics:*` synthetic
  modules. setTimeout removal landed 2026-05-14; this
  constraint is now solidly upheld.
- **No I/O from `mechanics:console`.** No stdout, no
  stderr, no host-side `tracing` emission. The console
  methods are pure no-ops. Future capture work is
  explicitly out of round-01 scope.
- **No state.** `mechanics:html` is pure. `mechanics:console`
  has no per-invocation state, no cross-call state, no
  cross-job state. Mechanics's stateless-per-job contract
  stands.
- **No `jsdom`-style globals.** Modules expose ES-style
  named/default `import`s only. No implicit globals.
- **No breakage to existing Rust public API** beyond the new
  feature flags. Existing consumers using
  `mechanics-core = "0.5"` with default features see no
  behaviour change.
- **Workspace HTTP-TLS posture** unchanged (no new HTTP
  client deps; mechanics-core doesn't need any).
- **Cargo.toml D24 discipline.** New deps (htmlize) declare
  `default-features = false` + an explicit feature list (or
  `features = []` if htmlize has no defaults that matter).
  Internal D24 audit principle applies.

## Per-crate version-bump policy

- `mechanics-core`: 0.5.1 → 0.6.0 (recommended) or 0.5.2
  (alternative). CHANGELOG entry. The bump batches D18
  rounds 01–04 until publish time.
- No other crate bumps. The new feature flags are additive,
  default-on, so downstream consumers (`mechanics`,
  `philharmonic`, the three bins) are transparent.

## References (read in this order)

1. `HUMANS.md` §"MIME module at `mechanics-core`" — the
   user's spec of the full D18 surface. Round 01 lands the
   foundation per this spec.
2. `docs/ROADMAP.md` §3.F **D18** — the captured scope.
3. `docs/design/06-execution-substrate.md` §"Realm surface
   (no non-ES globals)" — the hard rule that gates what can
   appear in the realm.
4. `mechanics-core/src/internal/runtime/builtins/`:
   - `mod.rs` — `bundle_builtin_modules` is the registration
     site to wrap with `#[cfg(feature = "...")]`.
   - `rand.rs`, `uuid.rs`, `base64.rs`, `base32.rs`,
     `hex.rs`, `form_urlencoded.rs`, `endpoint.rs` — the
     existing module-registration patterns; copy their
     shape for the two new modules.
5. `CONTRIBUTING.md` §§3.1, 4, 5, 10.3, 10.9, 11.
6. `mechanics-core/Cargo.toml` — current dep list; add
   `htmlize` per D24 discipline.

## Tests required (mandatory, the round is incomplete
without them)

- `mechanics:console` smoke test: a JS workflow imports
  `console` from `mechanics:console`, calls each level
  (`log`, `info`, `warn`, `error`, `debug`) with various
  arg shapes (no args / single string / multiple mixed-type
  args / objects), asserts each call returns `undefined`.
- `mechanics:html` round-trip tests: each of `escapeText`,
  `escapeAttribute`, `unescapeText`, `unescapeAttribute`
  exercised with the canonical HTML-escape patterns. Verify
  type-error path (passing a non-string).
- Existing rand/uuid/encoding tests still pass when the
  default features are on.
- `cargo check -p mechanics-core --no-default-features`:
  PASS. Confirms the gated modules compile-out cleanly.
- `cargo check -p mechanics-core --no-default-features
  --features console` etc.: PASS for each feature in
  isolation, to verify the gates are independent.

## Commit discipline (binding)

Same as recent rounds:

- **Codex does NOT commit.** No `./scripts/commit-all.sh`,
  no `git commit` / `git add` / `git push` / `git stash`.
  Read-only `git status` / `git diff` / `git log` are fine.
- **Codex does NOT publish.** No `./scripts/publish-crate.sh`,
  no `cargo publish`.
- **Codex CAN run `./scripts/pre-landing.sh`** at the end.
- Every cargo invocation needs `CARGO_TARGET_DIR=target-main`.
- Per-crate `cargo check -p mechanics-core --all-targets`
  after edits.

## Outcome

Pending — will be updated after Codex round 01 run.

---

<task>
mechanics-core module-surface refactor round 01: introduce a
`[features]` block, gate existing built-in modules
(`mechanics:rand` + `mechanics:uuid` behind `rand`;
`mechanics:form_urlencoded` + `:base64` + `:base32` + `:hex`
behind `encoding`), and add two new default-on modules:
`mechanics:html` (htmlize wrapper) and `mechanics:console`
(no-op, no I/O). `url` and `mime` are out of round-01 scope.

**Authoritative references:**

1. `HUMANS.md` §"MIME module at `mechanics-core`".
2. `docs/ROADMAP.md` §3.F **D18** captured scope, especially
   the bullet under each new module + the hard-constraints
   block.
3. `docs/design/06-execution-substrate.md` §"Realm surface
   (no non-ES globals)" hard rule.
4. `mechanics-core/src/internal/runtime/builtins/`:
   - `mod.rs` `bundle_builtin_modules` (around line 197) —
     the registration site.
   - Existing `rand.rs` / `uuid.rs` / codec modules — copy
     shape.
5. `CONTRIBUTING.md` §§3.1, 4, 5, 10.3, 10.9, 11.

**Concrete tasks:**

1. **`mechanics-core/Cargo.toml`**:
   - Add `[features]` block:
     ```toml
     [features]
     default = ["rand", "encoding", "html", "console"]
     rand = []
     encoding = []
     html = []
     console = []
     ```
   - Add `htmlize` dep with `default-features = false` and
     the minimal feature list its wrapper functions need.
   - Bump version `0.5.1 → 0.6.0`. (Default-on means no
     consumer breakage; bump is semantic clarity.)

2. **`mechanics-core/src/internal/runtime/builtins/mod.rs`**:
   - Wrap each `<module>::register(loader, context);` call
     in `bundle_builtin_modules` with the appropriate
     `#[cfg(feature = "...")]`. `endpoint` stays
     unconditional.
   - Add `mod console;` and `mod html;` at the top with
     `#[cfg(feature = "...")]` attributes so the modules
     are only compiled when their feature is on.

3. **New `mechanics-core/src/internal/runtime/builtins/console.rs`**:
   - Register a `mechanics:console` synthetic module
     exposing a default-export `console` object with methods
     `log`, `info`, `warn`, `error`, `debug`.
   - All methods: variadic args (any JS value), return
     `undefined`, no side effects, no I/O.
   - WHATWG-spec method signatures only; format-spec parsing
     can be a TODO for future capture-into-response work.
   - Shape patterned on existing `rand.rs` / `uuid.rs`.

4. **New `mechanics-core/src/internal/runtime/builtins/html.rs`**:
   - Register a `mechanics:html` synthetic module exposing
     named exports `escapeText`, `escapeAttribute`,
     `unescapeText`, `unescapeAttribute`.
   - Each function takes a single string arg; returns the
     escaped/unescaped string.
   - Backed by `htmlize` crate:
     - `escapeText` → `htmlize::escape_text`
     - `escapeAttribute` → `htmlize::escape_all_quotes`
     - `unescapeText` → `htmlize::unescape`
     - `unescapeAttribute` → `htmlize::unescape_attribute`
   - Type-check the arg as a JS string; throw `TypeError`
     if not.

5. **Tests** (add `#[test]` functions; pattern after
   existing module tests):
   - `mechanics:console` smoke test: JS workflow imports
     `console` from `mechanics:console`, calls each level
     with various arg shapes, asserts return `undefined`.
   - `mechanics:html` round-trip tests for each of the four
     functions (canonical HTML-escape patterns + a non-
     string-arg type-error test).
   - Gate existing rand/uuid/encoding tests with
     `#[cfg(feature = "...")]` so a
     `--no-default-features` build still compiles.

6. **`mechanics-core/CHANGELOG.md`** entry under 0.6.0 (or
   whichever version Codex picks) covering:
   - New `[features]` block; defaults preserve previous
     behaviour for consumers using `default-features = true`.
   - New `mechanics:console` (no-I/O no-op); future capture
     into RunJobResponse is out of scope.
   - New `mechanics:html` (htmlize wrapper).
   - Existing mechanics:rand/:uuid now gated by `rand`;
     mechanics:form_urlencoded/:base64/:base32/:hex now
     gated by `encoding`.

7. **Verification**:
   - `CARGO_TARGET_DIR=target-main cargo check -p
     mechanics-core --all-targets`: PASS.
   - `CARGO_TARGET_DIR=target-main cargo check -p
     mechanics-core --no-default-features --all-targets`:
     PASS.
   - `CARGO_TARGET_DIR=target-main cargo check -p
     mechanics-core --no-default-features --features rand
     --all-targets`: PASS (any single-feature build
     compiles).
   - `./scripts/pre-landing.sh`: PASS.
   - `CARGO_TARGET_DIR=target-main cargo deny check bans`:
     PASS.
   - `CARGO_TARGET_DIR=target-main cargo tree --workspace
     --invert ring --target x86_64-unknown-linux-gnu`:
     empty.

<action_safety>
- Codex does NOT commit. No `./scripts/commit-all.sh`,
  no `git commit`, no `git add`, no `git push`, no other
  gitwrite. Read-only git OK.
- Codex does NOT publish. No `./scripts/publish-crate.sh`,
  no `cargo publish`.
- Codex CAN run `./scripts/pre-landing.sh`.
- Every cargo via `CARGO_TARGET_DIR=target-main`.
- POSIX-ish host. POSIX sh for any shell wrapper.
- Run `./scripts/xtask.sh calendar-jp` at session start and
  before returning. If JST outside 10:00-19:00 (ext 21:00),
  note in the reply.
- TLS posture stays rustls + aws-lc-rs + webpki-roots only.
- **No non-ES globals.** The realm surface gets only
  ES-spec globals + `mechanics:*` synthetic modules. The
  new `mechanics:console` is a synthetic module accessible
  via `import console from "mechanics:console"`, NOT a
  realm global.
- **No I/O from mechanics:console.** Methods are pure
  no-ops. No stdout, no stderr, no tracing emission. The
  function bodies just consume args and return undefined.
</action_safety>

<missing_context_gating>
Before starting, run `./scripts/status.sh` — parent should
be clean (recent docs reconcile commit `3da120c` was the
last change). If the workspace has unrelated dirty work,
STOP and report.

Read the existing `rand.rs` and `uuid.rs` modules to see
the registration pattern. The two new modules should
follow the same shape: a `register(loader, context)`
function that constructs a synthetic module via
`loader.define_module(js_string!("mechanics:..."),
the_module)`.

Read the `htmlize` crate's docs (or its `Cargo.toml.orig`
in `~/.cargo/registry/src/.../htmlize-*/Cargo.toml.orig`)
to verify the function names + signatures. Pinpoint the
minimal feature set htmlize needs.
</missing_context_gating>

<default_follow_through_policy>
Land all six concrete-task items in this single round.
Don't stop after the Cargo.toml + cfg gating and report
"new modules pending" — the new modules are what proves
the foundation works. If a hard blocker surfaces (e.g.
htmlize's API doesn't match what HUMANS.md sketched), stop,
document, report INCOMPLETE. Don't paper over with a
partial-module stub.
</default_follow_through_policy>

<completeness_contract>
"Complete" means all of:

1. `mechanics-core/Cargo.toml` has `[features]` block,
   htmlize dep, version bumped.
2. `bundle_builtin_modules` correctly gates each existing
   module's registration.
3. New `console.rs` and `html.rs` modules exist + register
   their `mechanics:*` synthetic modules + are conditionally
   compiled per `#[cfg(feature = "...")]`.
4. Tests for console + html exist; existing tests gated
   correctly.
5. `cargo check -p mechanics-core --all-targets` PASS.
6. `cargo check -p mechanics-core --no-default-features
   --all-targets` PASS.
7. `cargo check` for each single-feature variant PASS.
8. `pre-landing.sh` PASS.
9. `cargo deny check bans` PASS.
10. `cargo tree --invert ring`: empty.
11. CHANGELOG entry written.
12. `## Outcome` of this prompt file updated.

If any of 1–10 incomplete, report INCOMPLETE clearly.
</completeness_contract>

<structured_output_contract>
At end of round 01, return:

1. **Summary** (2-3 sentences): what landed; version chosen
   (0.6.0 vs 0.5.2); any deviation from prompt.
2. **Touched files** grouped by category (Cargo.toml /
   builtins module entries / new module files / tests /
   CHANGELOG).
3. **Public API changes**: new `[features]` flags (rand,
   encoding, html, console); new `mechanics:html` named
   exports; new `mechanics:console` default export shape.
4. **Test coverage**: file paths + test names + what each
   asserts.
5. **Verification results** per the verification block
   above.
6. **Residual risks**: anything left as TODO (e.g. WHATWG
   console format-spec parsing deferred; htmlize feature-
   set assumption; any unexpected upstream-htmlize API
   wrinkles).
7. **Git state**: `./scripts/status.sh` output. NO commits
   made.
8. **Outcome paragraph** for the prompt file's `## Outcome`.
</structured_output_contract>
</task>
