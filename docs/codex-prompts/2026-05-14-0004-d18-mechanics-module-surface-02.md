# D18 — mechanics-core module-surface refactor (round 02)

**Date:** 2026-05-14 (JST)
**Slug:** `d18-mechanics-module-surface`
**Round:** 02 — adds `mechanics:url` (WHATWG URL via the `url`
crate) on top of R01's feature-gating foundation.
**Subagent:** `codex:codex-rescue`

## Motivation

R01 (`-01.md`) landed the feature-gating foundation + the two
simplest new modules (`console` no-op, `html` htmlize wrapper).
R02 lands `mechanics:url` per HUMANS.md §"MIME module at
`mechanics-core`" — the WHATWG URL spec surface that JS
workflows need for URL composition / parsing without resorting
to ad-hoc regex.

HUMANS.md spec:

> Feature `url` (default): a new WHATWG URL API-compliant API.
> Default export `URL`; named export `URLSearchParams`. Backed
> by the `url` crate.

R03 will land `mime`. R04 will refresh the workflow-authoring
guide.

## Round 02 scope

### What changes

1. **`mechanics-core/Cargo.toml`**:
   - Add `url` to the `[features]` block:
     ```toml
     [features]
     default = ["rand", "encoding", "html", "console", "url"]
     # ... existing rand / encoding / html / console ...
     url = []
     ```
   - `url` crate dep is already present in `[dependencies]`
     (used elsewhere in mechanics-core); no new dep introduction.
     Verify the existing pin + features are sufficient for the
     module's needs (parse, serialise, host/scheme/path/query
     accessors, `URLSearchParams` iteration). If
     `default-features = false` blocks something the
     mechanics:url module needs (e.g. `idna` for IDN handling),
     widen the existing feature list with an inline `# kept:`
     comment per D24 audit discipline.
   - No version bump on mechanics-core. The crate is already at
     0.6.0 (R01 bump); R02 lands inside the same 0.6.0 release
     window. Yuka holds the publish until full D18 is done.

2. **`mechanics-core/src/internal/runtime/builtins/mod.rs`**:
   - Add `mod url;` with `#[cfg(feature = "url")]` attribute,
     mirroring the R01 pattern for `mod console;` / `mod html;`.
   - Add `#[cfg(feature = "url")] url::register(loader,
     context);` to `bundle_builtin_modules`, in the same shape
     as R01's html / console registrations.

3. **New `mechanics-core/src/internal/runtime/builtins/url.rs`**:
   Register a `mechanics:url` synthetic module exposing:

   - **Default export `URL`** — a constructible JS class whose
     methods + properties mirror the WHATWG URL API:
     - **Constructor**: `new URL(input: string, base?: string)`
       — parses `input` against optional `base`. Throws
       `TypeError` if the URL is invalid.
     - **Properties** (each backed by `url::Url`'s
       getter/setter):
       - `href` (get/set)
       - `origin` (get-only)
       - `protocol` (get/set)
       - `username` (get/set)
       - `password` (get/set)
       - `host` (get/set) — includes port if present
       - `hostname` (get/set)
       - `port` (get/set)
       - `pathname` (get/set)
       - `search` (get/set) — the query string with leading `?`
       - `searchParams` (get-only) — returns a
         `URLSearchParams` bound to this URL's query string;
         mutations on it MUST reflect back into the URL's
         search.
       - `hash` (get/set) — the fragment with leading `#`
     - **Methods**:
       - `toString()` — returns `href`.
       - `toJSON()` — returns `href`.
     - **Static methods**:
       - `URL.canParse(input, base?)` — boolean; doesn't throw.
       - `URL.parseSafely(input, base?)` — returns a URL or
         null. NOT in WHATWG; **omit** unless WHATWG's
         `URL.parse` static lands by spec. WHATWG's
         `URL.parse(input, base?)` IS in the spec as of 2023;
         **include it** returning `URL | null`.

   - **Named export `URLSearchParams`** — a constructible JS
     class mirroring WHATWG URLSearchParams:
     - **Constructor**: `new URLSearchParams(init?: string |
       Iterable<[string, string]> | Record<string, string>)`.
       Accepts:
       - A query-string with optional leading `?`.
       - An iterable of `[name, value]` pairs.
       - A plain object whose own enumerable string-keyed
         properties become `name=value` entries.
     - **Methods**:
       - `append(name, value)` — adds a new entry; doesn't
         replace existing.
       - `delete(name, value?)` — deletes entry(s); value-
         specific delete when `value` provided (WHATWG 2023+
         spec).
       - `get(name)` — first value or null.
       - `getAll(name)` — array of all values.
       - `has(name, value?)` — boolean; value-specific check
         when `value` provided.
       - `set(name, value)` — replaces all entries for `name`
         with a single entry.
       - `sort()` — stable sort by name preserving insertion
         order for same-name pairs.
       - `toString()` — serialised form (no leading `?`).
       - `entries()`, `keys()`, `values()` — JS iterators.
       - `forEach(callback)` — iterate each entry calling
         callback.
     - **Iterable**: `URLSearchParams` is itself iterable,
       yielding `[name, value]` pairs in insertion order
       (same as `entries()`).
     - **Size**: `size` getter returns the entry count.

   Backing: the existing `url` crate's `Url` type + a small
   in-module helper for URLSearchParams (which url::Url
   exposes via `query_pairs`/`query_pairs_mut`). The
   bidirectional binding (`url.searchParams` mutations
   reflecting back) is the trickier piece — pick the cleanest
   shape: e.g. a small `JsUrlState` struct held in the JS
   object's internal data slot, with `URLSearchParams`
   instances either owning their own copy + a write-through
   callback OR holding a reference to the parent URL's state.
   Codex picks; document the chosen shape inline.

   **Hard rules:**

   - No I/O. `mechanics:url` is pure parsing + composition;
     no DNS lookups, no network access, no file-system reads.
     `url::Url::parse` is the heaviest operation and stays
     CPU-only.
   - No non-ES globals introduced. `URL` and `URLSearchParams`
     are accessed via `import URL, { URLSearchParams } from
     "mechanics:url"` per HUMANS.md's `import` example. They
     are NOT installed as realm globals.
   - Type-strict: invalid input types throw `TypeError`. The
     WHATWG spec's coercion rules (e.g. `URL` constructor's
     `String(input)` step) are honoured; implementation
     leans on Boa's `to_string(context)` for arg coercion.
   - Per-job stateless: no shared state across `URL`
     instances or `URLSearchParams` instances. Each `new
     URL(...)` creates an independent object.
   - The `URL` and `URLSearchParams` constructors are
     callable (with `new`) and throw `TypeError` when called
     without `new` (per WHATWG spec).

4. **Tests**: add a new `tests/url.rs` (or extend an existing
   test file) with JS-literal round-trip tests covering:

   - Basic construction:
     `new URL("https://example.com/path?a=1#frag")` and
     property accessors.
   - Base-relative construction:
     `new URL("/path", "https://example.com")`.
   - Invalid input throws TypeError:
     `try { new URL("not a url") } catch (e) { ... }`.
   - Property mutation:
     `url.pathname = "/new"`; `url.href` reflects.
   - `searchParams` round-trip: set/get/append/delete/sort;
     mutations reflect in `url.search`.
   - `URLSearchParams` standalone: from string, from
     iterable, from object.
   - `toString` / `toJSON` semantics.
   - `URL.canParse` / `URL.parse` static methods.

5. **`mechanics-core/CHANGELOG.md`**: append a new bullet under
   the existing `## [0.6.0] - 2026-05-14` entry:
   ```markdown
   - Added `mechanics:url` behind the `url` feature. The
     module exposes WHATWG-compliant `URL` (default export)
     and `URLSearchParams` (named export) classes backed by
     the `url` crate. Includes constructor + property
     accessors (`href`/`origin`/`protocol`/`host`/...) +
     `URLSearchParams` with `append`/`delete`/`get`/`getAll`/
     `has`/`set`/`sort`/`entries`/`keys`/`values`/`forEach`/
     iteration / `size`. Bidirectional binding:
     `url.searchParams` mutations reflect back into
     `url.search`.
   ```
   Also add `"url"` to the round-01 default-features list
   bullet text where the original entry says `default =
   ["rand", "encoding", "html", "console"]` — update to
   `default = ["rand", "encoding", "html", "console", "url"]`.

### What does NOT change in round 02

- **`mime` module** — R03.
- **Workflow-authoring guide** — R04.
- **`philharmonic` and `mechanics` dep-pin updates** —
  unchanged. Both reference `mechanics-core = "0.6"` from R01;
  no change here.
- **No new external deps** (the `url` crate is already
  present).
- **No version bumps** — mechanics-core stays at 0.6.0.

### Hard constraints (binding)

- **No non-ES globals.** `URL` and `URLSearchParams` are
  module-imported, not realm globals.
- **No I/O.** Pure parsing + composition.
- **Stateless per job.** No cross-job/cross-instance shared
  state.
- **D24 dep-features discipline.** If the `url` dep needs
  widened features (e.g. for IDN), document with inline
  `# kept:` comment.
- **No `[features]` redesign in published crates besides
  mechanics-core's local additions.**
- **No `philharmonic` / `mechanics` API changes.**

## Per-crate version-bump policy

- `mechanics-core`: stays at 0.6.0. No bump. R01 already
  bumped 0.5.1 → 0.6.0; the 0.6.0 release window covers all
  D18 rounds until publish.
- No other crate bumps.

## References (read in this order)

1. `docs/codex-prompts/2026-05-14-0004-d18-mechanics-module-surface-01.md`
   — R01 prompt + Outcome. R02 inherits its operational
   discipline + the pattern for adding new modules.
2. `HUMANS.md` §"MIME module at `mechanics-core`" — the
   `url` feature spec.
3. `docs/ROADMAP.md` §3.F **D18** — captured scope.
4. `docs/design/06-execution-substrate.md` §"Realm surface
   (no non-ES globals)" — hard rule.
5. `mechanics-core/src/internal/runtime/builtins/`:
   - `mod.rs` — `bundle_builtin_modules` (R01 already gated
     each existing module + html + console; add `url` in the
     same shape).
   - `html.rs` (R01) — closest shape template since `url` is
     also a thin wrapper over an external crate.
   - `endpoint.rs` — more complex module (constructible
     types, methods, async); `url` will need similar
     class-based shape for `URL` / `URLSearchParams`.
6. `url` crate (in workspace tree via mechanics-core's
   existing dep) — the `url::Url` API.
7. WHATWG URL spec — `https://url.spec.whatwg.org/`. Codex
   doesn't need network access for this; cribbing the public
   API surface from the `url` crate + the WHATWG-spec methods
   listed above is sufficient.

## Commit discipline (binding)

Same as R01:

- **Codex does NOT commit.** No `./scripts/commit-all.sh`,
  no `git commit` / `git add` / `git push` / `git stash`.
  Read-only `git status` / `git diff` / `git log` are fine.
- **Codex does NOT publish.** No `./scripts/publish-crate.sh`.
- **Codex CAN run `./scripts/pre-landing.sh`** at the end.
- Every cargo invocation needs `CARGO_TARGET_DIR=target-main`.

## Outcome

Pending — will be updated after Codex round 02 run.

---

<task>
Add the `mechanics:url` synthetic module to `mechanics-core`
behind a new default-on `url` Cargo feature. Exposes
WHATWG-compliant `URL` (default export) and `URLSearchParams`
(named export) classes backed by the existing `url` crate dep.
Builds on R01's feature-gating foundation; mechanics-core stays
at 0.6.0 (no version bump).

**Authoritative references:**

1. `docs/codex-prompts/2026-05-14-0004-d18-mechanics-module-surface-01.md`
   — R01 prompt + landed shape. Inherit the operational
   discipline (no commits, no publish, can run pre-landing.sh).
2. `HUMANS.md` §"MIME module at `mechanics-core`":
   > Feature `url` (default): a new WHATWG URL API-compliant
   > API. Default export `URL`; named export
   > `URLSearchParams`. Backed by the `url` crate.
3. `mechanics-core/src/internal/runtime/builtins/`:
   - `mod.rs` — extend `bundle_builtin_modules` with `#[cfg
     (feature = "url")] url::register(loader, context);`.
   - `html.rs` (R01) — closest shape template.
   - `endpoint.rs` — class-shape pattern for constructible
     types with methods.
4. The existing `url` crate dep in `mechanics-core/Cargo.toml`.

**Concrete tasks:**

1. **`mechanics-core/Cargo.toml`**:
   - Add `url` to the `[features]` block. Default features
     become `default = ["rand", "encoding", "html",
     "console", "url"]`.
   - Verify the existing `url` dep's features cover the
     module's needs (parsing, host/scheme/path/query
     accessors, `query_pairs_mut`). Widen with inline
     `# kept:` comment if needed.
   - No version bump on mechanics-core (stays at 0.6.0).

2. **`mechanics-core/src/internal/runtime/builtins/mod.rs`**:
   - Add `#[cfg(feature = "url")] mod url;`.
   - Add `#[cfg(feature = "url")] url::register(loader,
     context);` to `bundle_builtin_modules`.

3. **New `mechanics-core/src/internal/runtime/builtins/url.rs`**:
   - Register a `mechanics:url` synthetic module.
   - **Default export `URL`** — a JS class wrapping
     `url::Url`. Constructor `new URL(input: string, base?:
     string)`; throws `TypeError` on invalid input. Property
     accessors for `href` (get/set), `origin` (get-only),
     `protocol`, `username`, `password`, `host`, `hostname`,
     `port`, `pathname`, `search`, `searchParams` (get-only,
     returns `URLSearchParams` bound to the URL),
     `hash`. Methods `toString()`, `toJSON()` returning
     `href`. Static methods `URL.canParse(input, base?)`
     returning bool, `URL.parse(input, base?)` returning
     `URL | null`. Constructor without `new` throws
     `TypeError`.
   - **Named export `URLSearchParams`** — a JS class.
     Constructor accepts a string (with optional leading
     `?`), an iterable of `[name, value]` pairs, or a plain
     object. Methods `append`, `delete(name, value?)`,
     `get`, `getAll`, `has(name, value?)`, `set`, `sort`,
     `toString`, `entries`, `keys`, `values`, `forEach`,
     iteration protocol, `size` getter. Constructor without
     `new` throws `TypeError`.
   - **Bidirectional binding**: when `URLSearchParams` is
     accessed via `url.searchParams`, mutations on it reflect
     back into `url.search`. Codex picks the cleanest shape;
     document inline.
   - **No I/O.** No DNS, no network, no FS.
   - **No globals.** All exports module-imported.
   - **Type-strict.** Invalid input types throw `TypeError`
     per WHATWG-spec coercion rules.

4. **Tests**: JS-literal round-trip tests for:
   - Basic construction + property accessors.
   - Base-relative construction.
   - Invalid input throws TypeError.
   - Property mutation reflecting in `href`.
   - `searchParams` round-trip with append/delete/sort and
     bidirectional binding.
   - `URLSearchParams` from string / iterable / object.
   - `toString` / `toJSON`.
   - `URL.canParse` / `URL.parse` statics.
   - `--no-default-features --features url` build still
     compiles.

5. **`mechanics-core/CHANGELOG.md`**: append a new bullet
   under the existing `## [0.6.0] - 2026-05-14` entry
   documenting `mechanics:url`. Update the default-features
   list bullet in the same entry to include `"url"`.

6. **Verification**:
   - `CARGO_TARGET_DIR=target-main cargo check -p
     mechanics-core --all-targets`: PASS.
   - `CARGO_TARGET_DIR=target-main cargo check -p
     mechanics-core --no-default-features --all-targets`:
     PASS.
   - `CARGO_TARGET_DIR=target-main cargo check -p
     mechanics-core --no-default-features --features url
     --all-targets`: PASS.
   - `./scripts/pre-landing.sh`: PASS.
   - `CARGO_TARGET_DIR=target-main cargo deny check bans`:
     PASS.

<action_safety>
- Codex does NOT commit. No `./scripts/commit-all.sh`, no
  raw git-write. Read-only git OK.
- Codex does NOT publish.
- Codex CAN run `./scripts/pre-landing.sh`.
- Every cargo via `CARGO_TARGET_DIR=target-main`.
- POSIX-ish host. POSIX sh.
- Run `./scripts/xtask.sh calendar-jp` at session start and
  before returning. If JST outside 10:00-19:00 (ext 21:00),
  note in reply.
- TLS posture unchanged.
- No non-ES globals.
- No I/O from mechanics:url.
</action_safety>

<missing_context_gating>
Before starting, run `./scripts/status.sh` — parent should
be clean (last commit was R01 + workspace hardening). If
unrelated dirty work, STOP and report.

Read the existing `mechanics-core/src/internal/runtime/builtins/html.rs`
(landed in R01) as the closest shape template — it's a thin
wrapper over an external crate with named exports, same
pattern as `mechanics:url`'s static methods.

For the class-shape (constructible `URL` and
`URLSearchParams`), read `mechanics-core/src/internal/runtime/builtins/endpoint.rs`
or the existing rand/uuid modules to see how Boa-engine
classes / constructors are typically wired in this codebase.

Read the `url` crate's actual public API in
`~/.cargo/registry/src/.../url-2.5.*/src/lib.rs`. Verify
that `url::Url` exposes everything the WHATWG-spec accessor
list above needs (host/hostname/port/path/query/fragment +
mutation).
</missing_context_gating>

<default_follow_through_policy>
Land the full URL + URLSearchParams class shape + tests in
this single round. Don't stop after the basic constructor
+ accessor surface and report "URLSearchParams pending" —
the bidirectional binding is the trickiest piece and is
load-bearing for actual WHATWG-compat usage.

If a hard blocker surfaces (e.g. `url::Url` doesn't expose a
needed accessor in a way that makes bidirectional
`searchParams` binding clean), stop, document, report
INCOMPLETE. Acceptable fallback: implement URLSearchParams
as a SEPARATE owning class that doesn't bind back to a
parent URL, and document the deviation in residual risks.
The WHATWG bidirectional-binding test cases would then need
to be marked as expected-failures.
</default_follow_through_policy>

<completeness_contract>
"Complete" means all of:

1. `mechanics-core/Cargo.toml` has `url` in `[features]
   default = ...]`.
2. `bundle_builtin_modules` correctly cfg-gates the new
   url module.
3. New `url.rs` exists and exports the `mechanics:url`
   module with `URL` default + `URLSearchParams` named
   classes.
4. WHATWG accessor + method surface as specified is
   implemented (or any deviations documented).
5. Tests for each major behaviour exist.
6. `cargo check -p mechanics-core --all-targets`,
   `--no-default-features --all-targets`, and
   `--no-default-features --features url --all-targets`
   all PASS.
7. `pre-landing.sh` PASS.
8. `cargo deny check bans` PASS.
9. `cargo tree --invert ring` empty (Linux x86_64 targets).
10. CHANGELOG entry under 0.6.0 covers `mechanics:url`.
11. `## Outcome` of this prompt file updated.

If any of 1–9 incomplete, report INCOMPLETE clearly.
</completeness_contract>

<structured_output_contract>
At end of round 02, return:

1. **Summary** (2-3 sentences): URL + URLSearchParams shipped;
   any deviations from WHATWG spec.
2. **Touched files**: grouped (Cargo.toml / builtins/mod.rs /
   new url.rs / tests / CHANGELOG).
3. **WHATWG coverage**: which accessors/methods landed; any
   omissions and why.
4. **Bidirectional binding implementation**: which design
   shape was picked (owning copy + write-through callback /
   shared state / something else); residual edge cases.
5. **Test coverage**: file paths + test names + scenarios.
6. **Verification results**: per-crate cargo check PASS list
   for the three feature combinations; pre-landing.sh,
   cargo deny, cargo tree --invert ring.
7. **Residual risks**: anything left as TODO; WHATWG-spec
   corner cases that differ from `url::Url`'s default
   behaviour; bidirectional-binding edge cases.
8. **Git state**: `./scripts/status.sh` output. NO commits.
9. **Outcome paragraph** for the prompt file's `## Outcome`.
</structured_output_contract>
</task>
