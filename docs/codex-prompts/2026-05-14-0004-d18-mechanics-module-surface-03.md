# D18 — mechanics-core module-surface refactor (round 03)

**Date:** 2026-05-14 (JST)
**Slug:** `d18-mechanics-module-surface`
**Round:** 03 — adds `mechanics:mime` (composer + parser; the
sole non-default new module).
**Subagent:** `codex:codex-rescue`

## Motivation

R01 landed the foundation (feature-gating + `console` no-op +
`html` htmlize wrapper). R02 landed `mechanics:url` (WHATWG
URL + URLSearchParams via the `url` crate). R03 lands
`mechanics:mime` per HUMANS.md §"MIME module at
`mechanics-core`":

> If it is not too offtopic, add a structured MIME composer
> (it doesn't need to know about HTML, etc., just formats) to
> `mechanics-core`: `mechanics:mime`. It should handle Base64,
> multipart messages, etc, cleanly, emitting standard compliant
> MIME messages.
>
> ```js
> import { compose, parse } from `mechanics:mime`;
> ```

And later:

> Feature `mime` (non-default): see the above.
>
> Please note that philharmonic ecosystem enables mechanics-rs's
> `mime` feature unconditionally.

The driver: D7 (SMTP connector, `philharmonic-connector-impl-email-smtp`)
will benefit from `mechanics:mime` for workflow authors who
want to compose structured MIME messages before submission.
Workflow authors who prefer to hand-write the body string can
do so without the `mime` feature — hence non-default.

R04 (workflow-authoring guide refresh, en + jp) follows.

## Round 03 scope

### What changes

1. **`mechanics-core/Cargo.toml`**:
   - Add `mime` to the `[features]` block. **Non-default.**
     The default-features list stays
     `["rand", "encoding", "html", "console", "url"]`.
     Add `mime = ["dep:<mime-backing-crate(s)>"]` style — the
     feature pulls in optional deps that aren't compiled
     otherwise.
   - Pick a backing crate (or crates) for MIME composition +
     parsing. Recommended candidates, Codex's call:
     - `mailparse` (parse-only; small, well-maintained).
     - `mail-builder` (compose-only; small).
     - Combination of both.
     - `lettre`'s `message` module (heavier; would force
       `lettre` into mechanics-core's tree even though SMTP
       lives in a separate connector crate — probably
       overkill).
     - Hand-rolled minimal composer/parser. Acceptable if
       smaller / more vendorable, but moderate complexity
       (RFC 5322 + MIME RFCs + RFC 2047 encoded-word).
   - All new deps with D24 discipline: `default-features =
     false` + explicit feature list. Inline `# kept:`
     comments where features are kept for principled reasons.
   - No version bump on mechanics-core (stays at 0.6.0).

2. **`mechanics-core/src/internal/runtime/builtins/mod.rs`**:
   - Add `#[cfg(feature = "mime")] mod mime;` at module
     level, mirroring R01/R02 patterns.
   - Add `#[cfg(feature = "mime")] mime::register(loader,
     context);` to `bundle_builtin_modules`.

3. **New `mechanics-core/src/internal/runtime/builtins/mime.rs`**:
   Register a `mechanics:mime` synthetic module exposing:

   - **Named export `compose(message)`** — takes a structured
     JS object describing the message and returns the
     serialised MIME message as a JS string.

     **Input shape** (Codex picks the exact JS-level types,
     subject to these constraints):

     ```js
     // Simple text message
     {
       headers: {
         "From": "alice@example.com",
         "To": "bob@example.com",
         "Subject": "hi",
       },
       body: "plain text body"
     }

     // Multipart message
     {
       headers: {
         "From": "alice@example.com",
         "To": "bob@example.com",
         "Subject": "multipart hi",
       },
       parts: [
         {
           headers: { "Content-Type": "text/plain; charset=utf-8" },
           body: "plain text"
         },
         {
           headers: { "Content-Type": "text/html; charset=utf-8" },
           body: "<p>html</p>"
         }
       ]
     }

     // Binary attachment via Base64 (autodetected or
     // explicit)
     {
       headers: { ... },
       parts: [
         {
           headers: {
             "Content-Type": "application/octet-stream",
             "Content-Disposition": "attachment; filename=\"data.bin\""
           },
           body: Uint8Array.from([0, 1, 2, 3, ...]),
           encoding: "base64"  // optional override
         }
       ]
     }
     ```

     **Behaviour**:

     - Auto-insert `MIME-Version: 1.0` header if absent.
     - Auto-insert `Date:` header if absent (use a clock
       source — Mechanics doesn't expose `Date.now()` to
       host but Boa's engine has it; if the JS object has
       no Date, the composer can either: (a) leave it out
       and let SMTP submission servers add it, OR (b)
       derive from the engine's time. Codex picks; the
       SMTP connector D7 will set Date itself if absent.
       Recommended: leave Date out unless explicitly
       provided, document inline).
     - Auto-insert `Message-ID:` header if absent. Generate
       in `<random@local>` form, where `random` is a
       generated UUID-shaped token. Document the
       randomness source (boa's Math.random or similar).
     - Auto-generate multipart boundary when `parts` is
       present and no Content-Type boundary was set on
       the outer message.
     - Default outer Content-Type: `multipart/mixed` when
       `parts` is present without one; `text/plain;
       charset=utf-8` for body-only messages. Honor
       explicit Content-Type if set.
     - Auto-pick Content-Transfer-Encoding per part. Yuka
       2026-05-14 directives, in priority order:
       (a) base64 is acceptable as a fallback; (b)
       **quoted-printable is PREFERRED over base64 for
       text** because spam filters treat q-p content more
       favourably (legible text passes content classifiers
       cleanly; base64 hides intent and spammers abuse it).
       Final auto-detection rule:
       - 7-bit ASCII body (no chars > 0x7E, no NUL, no
         CR/LF other than line terminators inside an
         explicit text part) → `7bit`.
       - **UTF-8 text body with non-ASCII characters →
         `quoted-printable`** (preferred for text).
       - **Binary body (`Uint8Array`, or a Content-Type
         that isn't `text/*`) → `base64`** (no realistic
         q-p alternative for binary).
       - Explicit `encoding: <value>` in the part overrides
         the auto-detection. compose accepts the same
         encoding tokens that parse decodes
         (`7bit`/`8bit`/`binary`/`quoted-printable`/`base64`).
     - Encode non-ASCII header values via RFC 2047 encoded-
       word (`=?utf-8?B?...?=`). Subject line is the most
       common case.
     - Header folding per RFC 5322 (CRLF + space for long
       headers).
     - CRLF line endings throughout (per RFC 5322).
     - Throw `TypeError` if the input shape is wrong (e.g.
       `body` not a string / Uint8Array, `parts` not an
       array, `headers` not a plain object).

   - **Named export `parse(rawMessage)`** — takes a MIME
     message string (or Uint8Array) and returns a structured
     JS object mirroring the `compose` input shape.

     **Behaviour**:

     - Accept CRLF or LF line endings on input (be liberal).
     - Decode Content-Transfer-Encoding — **must support
       all common encodings, not just base64** (Yuka
       2026-05-14): `7bit`, `8bit`, `binary`,
       `quoted-printable`, `base64`. The asymmetry vs.
       compose is deliberate: `parse` accepts whatever the
       wire-format throws at it; `compose` canonicalises to
       base64 for non-ASCII output.
     - Decode RFC 2047 encoded-word headers back to UTF-8
       JS strings.
     - Detect multipart vs. simple body via Content-Type +
       boundary param.
     - Recursively parse multipart parts (each part is its
       own `{ headers, body }` or `{ headers, parts }`
       object).
     - Throw `TypeError` (or a more specific error class)
       on malformed input. Document the failure modes
       inline.

   - **Hard rules**:
     - **No I/O.** Pure parsing + composition; no
       file-system, no network, no DNS, no SMTP submission
       (that's D7's job — `philharmonic-connector-impl-email-smtp`).
     - **No realm globals.** `compose` and `parse` are
       imported via `import { compose, parse } from
       "mechanics:mime"`.
     - **Per-job stateless.** No shared state across
       invocations.
     - **Type-strict.** Reject malformed JS input with
       `TypeError`.
     - **No HTML knowledge.** MIME is format-only; HTML
       escaping is in `mechanics:html`, not `mime`.
     - **CRLF line endings on output** per RFC 5322.

4. **`philharmonic/Cargo.toml`** (meta-crate): activate
   mechanics-core's `mime` feature unconditionally when the
   `mechanics` feature is on (per HUMANS.md). Current shape:
   ```toml
   mechanics = ["dep:mechanics", "dep:mechanics-core", "dep:mechanics-config"]
   ```
   Becomes:
   ```toml
   mechanics = [
       "dep:mechanics",
       "dep:mechanics-core",
       "mechanics-core/mime",
       "dep:mechanics-config",
   ]
   ```
   (Cargo syntax: `mechanics-core/mime` activates the
   `mime` feature on the `mechanics-core` dep when the
   `mechanics` feature is enabled on philharmonic.)

   philharmonic CHANGELOG entry + version bump? philharmonic
   is already at 0.3.1 with both the D24 audit edits and the
   bind_h3 BaseArgs field unpublished. Adding `mechanics-core/mime`
   to the mechanics-feature activation is additive (purely
   enabling more downstream surface). Codex's call: bump
   0.3.1 → 0.3.2 or leave at 0.3.1 + append a CHANGELOG bullet
   under 0.3.1. Recommended: leave at 0.3.1 + append
   CHANGELOG (per the workspace's no-mid-work-bumps rule).

5. **`mechanics-core/CHANGELOG.md`**: append a new bullet
   under `## [0.6.0] - 2026-05-14` for `mechanics:mime` (the
   non-default `mime` feature):
   ```markdown
   - Added `mechanics:mime` behind the non-default `mime`
     feature. Named exports `compose` and `parse` over
     structured MIME message objects (`{ headers, body }`
     or `{ headers, parts }`). compose handles Base64 +
     quoted-printable transfer encoding, multipart
     boundary generation, RFC 2047 encoded-word for
     non-ASCII headers, and CRLF line endings per RFC 5322.
     parse decodes the same shapes plus 7bit/8bit/binary
     transfer encodings. No I/O. Backed by <backing
     crate(s)>.
   ```

6. **Tests** at `mechanics-core/src/internal/pool/tests/synthetic_modules.rs`
   (or extend the file the R01/R02 tests landed in). JS-
   literal exercises:
   - `compose` of a simple text message; assert specific
     byte patterns (MIME-Version, Content-Type,
     Content-Transfer-Encoding, CRLF endings).
   - `compose` of a multipart message; assert boundary
     header + part separators.
   - `compose` of a message with a non-ASCII Subject;
     assert RFC 2047 encoded-word format.
   - `compose` of a message with a Uint8Array body; assert
     base64 encoding.
   - `parse` of a simple text message; assert structure.
   - `parse` of a multipart message; assert nested
     structure.
   - Round-trip: `parse(compose(msg))` returns an
     equivalent structure.
   - `parse` of malformed input throws `TypeError` (or
     specific subclass).
   - `cargo check -p mechanics-core --features mime
     --all-targets` PASS (single-feature plus `mime`).

7. **Verification**:
   - `CARGO_TARGET_DIR=target-main cargo check -p
     mechanics-core --all-targets`: PASS (default features
     don't include `mime`; the cfg-gated code shouldn't
     compile).
   - `CARGO_TARGET_DIR=target-main cargo check -p
     mechanics-core --features mime --all-targets`: PASS
     (mime feature compiled).
   - `CARGO_TARGET_DIR=target-main cargo check -p
     mechanics-core --no-default-features --features mime
     --all-targets`: PASS (mime alone, no other defaults).
   - `CARGO_TARGET_DIR=target-main cargo check -p
     philharmonic --all-targets`: PASS (meta-crate's
     `mechanics` feature now activates `mime` transitively).
   - `./scripts/pre-landing.sh`: PASS.
   - `CARGO_TARGET_DIR=target-main cargo deny check bans`:
     PASS.
   - `CARGO_TARGET_DIR=target-main cargo tree --workspace
     --invert ring --target x86_64-unknown-linux-gnu`:
     empty.
   - `./scripts/check-no-registry.sh`: PASS.

### What does NOT change in R03

- **`mechanics:endpoint` / `:rand` / `:uuid` / `:encoding`
  modules** — unchanged.
- **`mechanics:html` / `:console` / `:url`** — unchanged
  (R01 + R02 work stands).
- **No new philharmonic API surface** — only the meta-crate
  feature wiring changes.
- **No SMTP submission** — that's D7. mechanics:mime is pure
  format only.
- **Workflow-authoring guide** — R04.

### Hard constraints (binding)

- No non-ES globals. `compose` and `parse` are module-
  imported only.
- No I/O. No SMTP, no FS, no DNS.
- Per-job stateless.
- D24 dep-features discipline for any new deps.
- Workspace TLS posture unchanged (no new TLS-touching deps).
- CRLF line endings per RFC 5322.

## Per-crate version-bump policy

- `mechanics-core`: stays at 0.6.0 (no bump; same release
  window as R01/R02).
- `philharmonic`: stays at 0.3.1 (no bump; the
  mechanics-core/mime activation is additive). CHANGELOG
  bullet appended under 0.3.1.
- No other crate bumps.

## References (read in this order)

1. `docs/codex-prompts/2026-05-14-0004-d18-mechanics-module-surface-01.md`
   — R01 prompt + operational discipline.
2. `docs/codex-prompts/2026-05-14-0004-d18-mechanics-module-surface-02.md`
   — R02 prompt + landed URL/URLSearchParams shape (closest
   template for class-based module registration; R03 is
   simpler since `compose` and `parse` are plain functions,
   not constructible classes).
3. `HUMANS.md` §"MIME module at `mechanics-core`" — the
   user's spec of the `mechanics:mime` surface +
   philharmonic-enables-mime-unconditionally directive.
4. `docs/ROADMAP.md` §3.F **D18** — captured scope.
5. `docs/design/06-execution-substrate.md` §"Realm surface
   (no non-ES globals)" — hard rule.
6. `mechanics-core/src/internal/runtime/builtins/`:
   - `mod.rs` — `bundle_builtin_modules` (R01 + R02 already
     gated each module; add `mime` in the same shape).
   - `html.rs` (R01) — closest shape template for thin
     wrappers around external crates with named exports.
7. `philharmonic/Cargo.toml` — current `[features]
   mechanics` definition.

## Commit discipline (binding)

Same as R01 / R02:

- **Codex does NOT commit.** No `./scripts/commit-all.sh`,
  no `git commit` / `git add` / `git push` / `git stash`.
  Read-only `git status` / `git diff` / `git log` are fine.
- **Codex does NOT publish.** No `./scripts/publish-crate.sh`.
- **Codex CAN run `./scripts/pre-landing.sh`** at the end.
- Every cargo invocation needs `CARGO_TARGET_DIR=target-main`.

## Codex report (optional but encouraged)

New convention 2026-05-14 (per Yuka): if anything non-obvious
surfaces during this round — a design call you had to make on
the fly, a blocker you worked around, a residual concern Yuka
should know about — write a short report to
`docs/codex-reports/2026-05-14-NNNN-d18-mechanics-mime.md`
per [`docs/codex-reports/README.md`](../codex-reports/README.md).
Pick the lowest unused `NNNN` (four-digit daily counter, this
directory's sequence is independent of the prompt archive).
Routine specified-and-shipped work doesn't need one; the
session summary covers it. Codex leaves the report dirty in
the working tree for Claude to commit alongside the
implementation diff.

Candidate report-worthy items for this round (Codex picks
which, if any, deserve a writeup):
- Backing-crate trade-off rationale (why mailparse vs.
  mail-builder vs. hand-rolled).
- RFC 5322 / RFC 2045-7 corner cases that surfaced
  (header folding, encoded-word boundaries, multipart
  preamble / epilogue / nested boundaries).
- Q-P vs. base64 auto-detect edge cases (the explicit-
  encoding override is the escape hatch; what counts as
  "non-ASCII text" in practice).
- Round-trip stability: if `parse(compose(msg))` doesn't
  return an exactly-equivalent object for some input
  class, that's worth flagging.

## Outcome

Pending — will be updated after Codex round 03 run.

---

<task>
Add the `mechanics:mime` synthetic module to mechanics-core
behind a new non-default `mime` Cargo feature. Named exports
`compose` and `parse` over structured MIME message objects;
handles Base64 + quoted-printable transfer encoding, multipart
boundary generation, RFC 2047 encoded-word for non-ASCII
headers, CRLF line endings per RFC 5322. No I/O.

**Authoritative references:**

1. `HUMANS.md` §"MIME module at `mechanics-core`":
   > If it is not too offtopic, add a structured MIME composer
   > to `mechanics-core`: `mechanics:mime`. It should handle
   > Base64, multipart messages, etc, cleanly, emitting standard
   > compliant MIME messages.
   > `import { compose, parse } from 'mechanics:mime';`
   > Feature `mime` (non-default). philharmonic ecosystem
   > enables mechanics-rs's `mime` feature unconditionally.
2. `docs/codex-prompts/2026-05-14-0004-d18-mechanics-module-surface-{01,02}.md`
   — R01 + R02 prompts. Inherit operational discipline + the
   module-registration pattern.
3. `mechanics-core/src/internal/runtime/builtins/mod.rs` —
   `bundle_builtin_modules` registration site.
4. `mechanics-core/src/internal/runtime/builtins/html.rs`
   (R01) and `url.rs` (R02) — closest shape templates.
5. `philharmonic/Cargo.toml` — `[features] mechanics`
   needs `mechanics-core/mime` added.

**Concrete tasks:**

1. **`mechanics-core/Cargo.toml`**:
   - Add `mime` to `[features]` as a NON-default feature.
     `default = ["rand", "encoding", "html", "console", "url"]`
     unchanged. New: `mime = ["dep:<backing-crate(s)>"]`.
   - Pick MIME backing crate(s) (Codex's call):
     `mailparse` + `mail-builder` is one reasonable pair;
     a hand-rolled minimal composer/parser is acceptable
     if smaller. Use D24 discipline on any new deps
     (`default-features = false` + explicit feature lists).
   - No mechanics-core version bump (stays at 0.6.0).

2. **`mechanics-core/src/internal/runtime/builtins/mod.rs`**:
   - Add `#[cfg(feature = "mime")] mod mime;`.
   - Add `#[cfg(feature = "mime")] mime::register(loader,
     context);` to `bundle_builtin_modules`.

3. **New `mechanics-core/src/internal/runtime/builtins/mime.rs`**:
   - Register `mechanics:mime` synthetic module with named
     exports `compose` and `parse`.
   - `compose(message)` — takes
     `{ headers, body }` or `{ headers, parts }` object,
     returns serialised MIME message string. Auto-inserts
     `MIME-Version: 1.0`, auto-generates multipart
     boundary when needed, defaults outer Content-Type
     (`multipart/mixed` for parts; `text/plain;
     charset=utf-8` for body). **Auto-picked
     Content-Transfer-Encoding (Yuka 2026-05-14):
     quoted-printable PREFERRED over base64 for non-ASCII
     text bodies (spam-filter-friendly), base64 only for
     binary**. Final rule: 7bit ASCII → `7bit`; non-ASCII
     text body → `quoted-printable`; binary
     (`Uint8Array` / non-`text/*` Content-Type) →
     `base64`. Explicit `encoding: "<token>"` per part
     overrides. compose accepts the same encoding tokens
     `parse` decodes
     (`7bit`/`8bit`/`binary`/`quoted-printable`/`base64`).
     RFC 2047 encoded-word for non-ASCII headers. Header
     folding per RFC 5322. CRLF line endings.
   - `parse(raw)` — takes MIME message string or
     `Uint8Array`, returns `{ headers, body }` or
     `{ headers, parts }`. **Must decode all common
     transfer encodings, not just base64** (Yuka
     2026-05-14): 7bit / 8bit / binary / quoted-printable
     / base64. Decodes RFC 2047 encoded-word headers.
     Recursive multipart parsing. Liberal LF-or-CRLF input.
     `TypeError` on malformed input.
   - **No I/O. No realm globals. Per-job stateless.
     Type-strict via Boa's `to_string(context)` /
     `to_object()` for arg coercion.**

4. **`philharmonic/Cargo.toml`**:
   - Update `[features] mechanics = [...]` to include
     `"mechanics-core/mime"` so philharmonic's mechanics
     feature transitively activates mechanics-core's mime
     feature. Cargo syntax: `"mechanics-core/mime"`
     activates the `mime` feature on the `mechanics-core`
     dep.
   - No philharmonic version bump (stays at 0.3.1; this is
     an additive feature activation).
   - Append a bullet under philharmonic CHANGELOG's 0.3.1
     entry documenting the activation.

5. **Tests** at the existing
   `src/internal/pool/tests/synthetic_modules.rs` (or
   wherever R01/R02 tests live). JS-literal exercises:
   compose simple text + multipart + non-ASCII subject +
   Uint8Array body; parse simple + multipart;
   round-trip; malformed-input TypeError. Plus a
   `--features mime` compile gate.

6. **`mechanics-core/CHANGELOG.md`**: append a bullet
   under `## [0.6.0] - 2026-05-14` for `mechanics:mime`,
   documenting the named exports + the supported encoding
   + the no-I/O / no-globals contract + the backing
   crate(s) chosen.

7. **Verification**:
   - `cargo check -p mechanics-core --all-targets`: PASS
     (no mime in defaults; cfg-gated out).
   - `cargo check -p mechanics-core --features mime
     --all-targets`: PASS.
   - `cargo check -p mechanics-core --no-default-features
     --features mime --all-targets`: PASS.
   - `cargo check -p philharmonic --all-targets`: PASS
     (mechanics feature transitively activates mime).
   - `./scripts/pre-landing.sh`: PASS.
   - `cargo deny check bans`: PASS.
   - `cargo tree --workspace --invert ring --target
     x86_64-unknown-linux-gnu`: empty.
   - `./scripts/check-no-registry.sh`: PASS.

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
- TLS posture stays rustls + aws-lc-rs + webpki-roots only.
  Don't pull `lettre` into mechanics-core's tree — SMTP is
  D7's job, separately. `mechanics:mime` is format-only.
- No non-ES globals.
- No I/O from mechanics:mime.
- CRLF line endings on `compose` output (RFC 5322).
</action_safety>

<missing_context_gating>
Before starting, run `./scripts/status.sh` — parent should
be clean (last commit was R02 + the doc trim). If
unrelated dirty work, STOP and report.

Read R01 (`-01.md`) and R02 (`-02.md`) prompts + their
landed code under
`mechanics-core/src/internal/runtime/builtins/{html,console,url}.rs`
to see the registration patterns. `mechanics:mime` is
simpler than `:url` (plain function exports, not
constructible classes).

Survey the MIME-related crates on crates.io for the
backing implementation. `mailparse` (parse) + `mail-builder`
(compose) is one balanced pair; verify their feature
surface + transitive deps + license + maintenance status
before committing. Confirm no `ring` / `native-tls` /
banned-dep paths.
</missing_context_gating>

<default_follow_through_policy>
Land compose + parse + tests + philharmonic feature wiring
+ CHANGELOG in this single round. Don't stop after compose
and report "parse pending" — `compose` and `parse` together
are the contract HUMANS.md spelled out.

If a hard blocker surfaces (e.g. no clean MIME backing
crate available, hand-rolling is too large for one round),
stop, document, report INCOMPLETE. Acceptable fallback for
round 03: ship `compose` only, leave `parse` as a TODO,
document inline + in CHANGELOG. Then round 04 can pick up
the parse half before the workflow-authoring guide refresh.

**Codex report (optional but encouraged; new 2026-05-14
convention):** if anything non-obvious surfaces during this
round — a design call you had to make on the fly, a
blocker you worked around, a residual concern Yuka should
know about — write a short report to
`docs/codex-reports/2026-05-14-NNNN-d18-mechanics-mime.md`
per [`docs/codex-reports/README.md`](../codex-reports/README.md).
Pick the lowest unused `NNNN` for the codex-reports
directory (independent counter from codex-prompts).
Routine specified-and-shipped work doesn't need one; the
session summary covers it. Codex leaves the report dirty
in the working tree for Claude to commit alongside the
implementation diff. Candidates worth a writeup: backing-
crate trade-off rationale, RFC corner cases (header
folding, encoded-word boundaries, multipart
preamble/epilogue/nested boundaries), q-p-vs-base64 auto-
detect edge cases, round-trip stability gaps.
</default_follow_through_policy>

<completeness_contract>
"Complete" means all of:

1. `mechanics-core/Cargo.toml` has non-default `mime`
   feature flag + the chosen backing-crate dep(s).
2. `bundle_builtin_modules` correctly cfg-gates the new
   mime module.
3. New `mime.rs` exists and registers `mechanics:mime`
   with named exports `compose` and `parse`.
4. `philharmonic/Cargo.toml` `[features] mechanics`
   transitively activates `mechanics-core/mime`.
5. Tests cover the major compose / parse / round-trip /
   malformed-input scenarios.
6. All cargo check variants PASS (default, +mime,
   --no-default-features +mime, philharmonic).
7. `pre-landing.sh` PASS.
8. `cargo deny check bans` PASS.
9. `cargo tree --invert ring`: empty (Linux x86_64).
10. `check-no-registry.sh` PASS.
11. CHANGELOG entries (mechanics-core + philharmonic) added.
12. `## Outcome` of this prompt file updated.

If any of 1–10 incomplete, report INCOMPLETE clearly.
</completeness_contract>

<structured_output_contract>
At end of round 03, return:

1. **Summary** (2-3 sentences): what landed; backing
   crate(s) chosen; whether `parse` is included or
   deferred.
2. **Touched files**: grouped (Cargo.toml / builtins/mod.rs /
   new mime.rs / tests / CHANGELOGs / philharmonic
   Cargo.toml).
3. **Public API**: `mechanics:mime` named exports
   (`compose` / `parse`) + the JS-level input/output
   shapes.
4. **Backing-crate choice**: which crate(s); their
   dep-features applied; transitive-dep sanity check
   (no ring / native-tls / banned deps).
5. **Test coverage**: file paths + test names + scenarios.
6. **Verification results**: per-crate cargo check PASS
   list for the four feature combinations + pre-landing +
   cargo deny + cargo tree --invert ring +
   check-no-registry.
7. **Residual risks**: WHATWG-spec gaps, RFC corner cases
   (header folding edge cases, encoded-word boundaries,
   multipart preamble/epilogue handling), or anything
   left as TODO.
8. **Git state**: `./scripts/status.sh` output. NO commits.
9. **Outcome paragraph** for the prompt file's `## Outcome`.
</structured_output_contract>
</task>
