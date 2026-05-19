# Script-argument shape changes: add `instance: { id, step }`; flatten `subject.tenant_id` and `subject.authority_id` to public V4 strings

**Date:** 2026-05-19 (JST)
**Slug:** `script-arg-instance-field`
**Round:** 01 — initial dispatch. One crate (`philharmonic-workflow`)
plus three doc files; no other submodules touched.
**Subagent:** `codex:rescue`

## Motivation

Two coupled changes to the JSON value workflow JS scripts receive,
both surfaced from
[`philharmonic-workflow::WorkflowEngine::execute_step`](../../philharmonic-workflow/src/engine.rs#L243-L249):

**(1) Add `instance: { id, step }`.** Scripts currently receive
`{context, args, input, subject, data}`. The workflow `context`
field is the caller-mutable state slot — **not** the instance UUID
— so there is no way for a script to know which `WorkflowInstance`
it is running inside, nor which step seq it occupies. Yuka has
decided to surface this as a new top-level field with shape
`instance: { id, step }`.

**(2) Flatten `subject.tenant_id` and `subject.authority_id` to the
public V4 UUID string.** The current
[`SubjectContext::to_script_value`](../../philharmonic-workflow/src/subject.rs#L33-L35)
implementation delegates to `serde_json::to_value(self)`, which
serialises each `EntityId<T>` field through the type's default
`Serialize` impl
([`philharmonic-types/src/entity.rs:259-263`](../../philharmonic-types/src/entity.rs#L259-L263))
— that produces
`{"internal": "<v7-uuid>", "public": "<v4-uuid>"}`. The script
therefore sees the internal V7 UUID, which is a design leak: scripts
are out-of-trust JS and should only see the public V4. Per Yuka
2026-05-19, "no internal ID exposure to scripts". Both `tenant_id`
and `authority_id` (`Option<EntityId<MintingAuthority>>`) are
reshaped to the bare public-V4 string (the latter remains
nullable for the principal-caller case). The `kind`, `id`, and
`claims` fields are unchanged.

**Scope of both changes:** one engine site plus
`philharmonic-workflow/src/subject.rs::to_script_value`, tests, the
`philharmonic-workflow` CHANGELOG `[Unreleased]` section, and three
doc files. Non-crypto. Non-breaking from a JS-author perspective for
(1) — purely additive. **Breaking** for (2) from any JS code that
currently reads `arg.subject.tenant_id.public` or
`arg.subject.authority_id.public`; those callers would now find
`arg.subject.tenant_id` itself a string. Yuka has accepted that
breakage explicitly — the pre-existing `{internal, public}` shape
was a design leak, not a contract. **No publish, no version bump**
— the crate's `Cargo.toml` stays at `0.1.6`; the change lands in
the `[Unreleased]` section of `CHANGELOG.md` and ships with whatever
version Yuka cuts later.

The persisted `StepRecordSubject`
([`subject.rs:51-58`](../../philharmonic-workflow/src/subject.rs#L51-L58))
is **not** changed — audit records keep the full `EntityId`
serialisation for forensic accuracy. Only the in-flight
`script_arg.subject` JSON value is reshaped.

## References (authoritative if anything in this prompt contradicts them)

1. [`docs/design/07-workflow-orchestration.md`](../design/07-workflow-orchestration.md)
   — current spec lists `{context, args, input, subject, data}` at
   lines 241-242, 282-285, 353, and the historical-expansion bullet
   at 492-493. Will become
   `{context, args, input, subject, data, instance}`.
2. [`docs/design/06-execution-substrate.md`](../design/06-execution-substrate.md)
   — script signature example at line 104; statelessness paragraph
   at line 324 currently says the executor receives "no instance ID,
   no correlation" — that line needs careful rework (the executor
   itself remains stateless across calls; the *script arg* now
   carries the instance UUID as an input, not as executor-held
   state).
3. [`docs/guide/workflow-authoring.md`](../guide/workflow-authoring.md)
   §"Script argument" (lines ~717-727).
4. [`CONTRIBUTING.md`](../../CONTRIBUTING.md):
   - **§4** Git workflow — `./scripts/commit-all.sh` only;
     **you do not commit** (see Hand-off shape below).
   - **§5** Script wrappers — every cargo call routes via the
     wrappers (which set `CARGO_TARGET_DIR=target-main`).
   - **§10.3** No panics in library `src/` — no `.unwrap()` /
     `.expect()` on `Result` / `Option`, no `panic!` /
     `unreachable!` / `todo!` / `unimplemented!` on reachable
     paths. Tests are exempt.
   - **§10.4** Library crates take bytes, not paths — file I/O
     belongs in bins. Not relevant to this dispatch directly
     (no new I/O), but keep the boundary intact.
   - **§11** Pre-landing checks — `./scripts/pre-landing.sh` is
     mandatory before declaring done.
   - **§14.6** English as the default for prose.

## Context files pointed at

**Engine site (the `instance` field edit):**

- [`philharmonic-workflow/src/engine.rs`](../../philharmonic-workflow/src/engine.rs)
  — `execute_step` builds `script_arg` at line 243. Add the new
  `instance` field there. For the UUID *in the script arg*, use the
  **public** ID: `instance_id.public().as_uuid()` — consistent with
  the "no internal ID exposure to scripts" rule applied to
  `subject.tenant_id` below. The `step` is the locally-computed
  `step_seq` (line 150-153). (`instance_id.internal().as_uuid()`
  remains correct for the other engine-internal sites that already
  use it — engine.rs:145, 196, 266, 283, 311, 323, 370, etc. —
  those are not script-facing.)

**Subject site (the flattening edit):**

- [`philharmonic-workflow/src/subject.rs`](../../philharmonic-workflow/src/subject.rs)
  — `SubjectContext::to_script_value` (lines 33-35) currently
  delegates to `serde_json::to_value(self)`. Replace with a manual
  build that emits:
  - `kind` — unchanged (`"principal"` or `"ephemeral"` via the
    existing serde rename_all snake_case).
  - `id` — unchanged (the opaque caller-identifier string).
  - `tenant_id` — `self.tenant_id.public().as_uuid().to_string()`
    (hyphenated lowercase V4).
  - `authority_id` — `Option::map` over `self.authority_id`,
    producing `Some(public-v4-string)` or `None`. Serialise the
    `Option<String>` so that `None` lands as JSON `null` (matches
    the previous shape's behaviour for principal callers).
  - `claims` — unchanged (opaque pass-through).

  The cleanest implementation is an inline `json!{...}` in
  `to_script_value`, or a private `#[derive(Serialize)]` wire
  struct (e.g. `ScriptSubject<'a> { kind, id: &'a str, tenant_id:
  String, authority_id: Option<String>, claims: &'a JsonValue }`)
  built from `&self` and then `serde_json::to_value`-d. Either is
  acceptable; pick what reads cleaner. Do **not** mutate the
  `SubjectContext` struct itself or its top-level `Serialize`
  impl — the type is also used for non-script purposes via the
  default Serialize (e.g. potentially logged at debug level), and
  changing the default impl would have action-at-a-distance
  consequences. Custom shape strictly inside `to_script_value`.

**Trait that stays put (do not change its signature):**

- [`philharmonic-workflow/src/executor.rs`](../../philharmonic-workflow/src/executor.rs)
  — `StepExecutor::execute(script, arg: &JsonValue, config)` keeps
  its current shape. The new field travels inside the existing
  `arg` parameter; no trait-level change.

**Audit shape that stays put (do not change):**

- `StepRecordSubject`
  ([`subject.rs:51-58`](../../philharmonic-workflow/src/subject.rs#L51-L58))
  — the persisted-audit shape stays as `EntityId<MintingAuthority>`
  (full `{internal, public}` serialisation) for `authority_id`.
  Forensic audit keeps the V7 reference. Only the in-flight script
  arg is flattened.

**Tests:**

- [`philharmonic-workflow/tests/`](../../philharmonic-workflow/tests/)
  — add coverage that the `instance` field is present with the
  right shape. There are existing harnesses in the crate that
  exercise `execute_step`; mirror their shape if one fits.
  Otherwise the cleanest seam is a small mock `StepExecutor`
  whose `execute` method captures the `arg` parameter into a
  `Mutex<Option<JsonValue>>` so the test can assert on the
  exact assembled value. Implementing the mock in-test is
  preferred over reaching for the real Boa executor — this is a
  shape contract test, not an end-to-end test.

**Version + CHANGELOG:**

- [`philharmonic-workflow/Cargo.toml`](../../philharmonic-workflow/Cargo.toml)
  — **stays at `0.1.6`. Do not bump.** Per Yuka's explicit call
  2026-05-19: "unreleased; no version bump needed."
- [`philharmonic-workflow/CHANGELOG.md`](../../philharmonic-workflow/CHANGELOG.md)
  — there is already an empty `## [Unreleased]` section just below
  the header. Add a bullet under it describing the new field. Do
  NOT create a new dated `## [0.1.x]` heading.

**Docs (the three files):**

- `docs/design/07-workflow-orchestration.md` (parent repo).
- `docs/design/06-execution-substrate.md` (parent repo).
- `docs/guide/workflow-authoring.md` (parent repo).

## Shape (locked)

```json
{
  "context":  { ... },
  "args":     { ... },
  "input":    { ... },
  "subject":  {
    "kind":         "principal" | "ephemeral",
    "id":           "<opaque caller string>",
    "tenant_id":    "<V4 UUID string, lowercased hyphenated>",
    "authority_id": "<V4 UUID string>" | null,
    "claims":       { ... }
  },
  "data":     { ... },
  "instance": {
    "id":   "<V4 UUID string, lowercased hyphenated>",
    "step": <unsigned integer, the step_seq for the step
             currently executing>
  }
}
```

### `instance`

- `instance.id`: the `WorkflowInstance`'s **public** V4 UUID,
  serialised via the default `Uuid::to_string()` (lowercased,
  hyphenated 8-4-4-4-12). Use `instance_id.public().as_uuid()`,
  not `.internal()`.
- `instance.step`: the `step_seq` value the engine computes at the
  top of `execute_step` (1-based;
  `latest.revision_seq.checked_add(1)`). Semantic: "the seq the
  engine is about to assign to the step record being created" —
  matches what the audit log records for this same step.

No other fields on `instance`. Yuka explicitly approved
`{ id, step }`; do NOT add `status`, `template_id`, `tenant_id`,
`correlation_id`, or anything else — those can be added later if a
use case appears.

### `subject` (flattened)

The five fields above are the complete set. Compared to the
previous shape produced by `serde_json::to_value(self)`:

- `kind` — unchanged (`"principal"` | `"ephemeral"`).
- `id` — unchanged (opaque caller identifier string).
- `tenant_id` — was `{"internal": "<v7>", "public": "<v4>"}`; now
  the bare V4 string.
- `authority_id` — was either `null` or
  `{"internal": "<v7>", "public": "<v4>"}`; now either `null` or
  the bare V4 string.
- `claims` — unchanged (opaque pass-through).

No other fields. The persisted `StepRecordSubject` shape is
untouched (`authority_id` stays full `EntityId` there).

## Hard requirements

### Shared

- **No internal-V7-UUID exposure to scripts.** Anywhere a UUID
  ends up inside `script_arg`, it is the **public V4** form. This
  rule covers `instance.id`, `subject.tenant_id`, and
  `subject.authority_id`. Use `EntityId::public().as_uuid()`
  consistently.
- **No version bump.** `philharmonic-workflow/Cargo.toml` stays
  `version = "0.1.6"`. Add the entry to the existing
  `## [Unreleased]` section in `CHANGELOG.md`.
- **No panics in library `src/`** per CONTRIBUTING.md §10.3.
  Tests exempt.
- **Do not change `StepExecutor`'s trait signature.** Adding a
  field inside `arg: &JsonValue` is sufficient.
- **Do not change `StepRecordSubject` or the step-record subject
  persistence shape.** The audit confinement at
  [`engine.rs:233`](../../philharmonic-workflow/src/engine.rs#L233)
  stays limited to `kind` + `id` + `authority_id` (with
  `authority_id` retaining the full `EntityId` shape for forensic
  V7 reference). The instance identity is already implicit at the
  step-record level (the step record's parent entity is the
  instance).

### `instance` field

- Place the `instance` key after `data` in the JSON object source
  ordering, for readability when humans skim engine code or log
  dumps. (`json!{...}` preserves insertion order in source;
  canonical JSON is only applied at hash sites and is not
  relevant to `script_arg` itself.)
- `instance.id` serialises as a JSON **string** (default `Uuid`
  formatting; lowercased hyphenated 8-4-4-4-12). Not a nested
  object, not a byte array.
- `instance.step` serialises as a JSON **number**. Use
  `step_seq`'s native unsigned-integer type; do not stringify.

### `subject` flattening

- `to_script_value` no longer delegates to
  `serde_json::to_value(self)`. Build the JSON shape explicitly
  (inline `json!{...}` or a private wire struct — see Context
  files above).
- `tenant_id` is the public V4 UUID **string**, never an object,
  never `null`. The `Tenant` entity must exist for any valid
  request, so `tenant_id` is non-optional in the source struct
  and stays non-optional in the wire shape.
- `authority_id` is `Option<String>`. `None` → JSON `null`
  (matches the previous shape's `null` for principal callers);
  `Some(EntityId)` → its public V4 as a string.
- Do **not** modify the public `Serialize` impl on
  `SubjectContext` — the change is local to `to_script_value`.
- Do **not** change the `claims` field's shape — it passes
  through as-is.

### Compatibility note for callers

The `subject` reshape is a **breaking change** for any JS that
reads `arg.subject.tenant_id.public` or
`arg.subject.authority_id.public`. Yuka has accepted this — the
`{internal, public}` shape was a design leak from default serde,
not an intended contract. The CHANGELOG `[Unreleased]` entry must
call out the breaking shape change explicitly so the next publish
notes it; the `philharmonic/webui/` chat surface and any other
in-tree consumer must be grep'd for `subject.tenant_id` /
`subject.authority_id` usage during this round and either updated
or flagged. (Surface findings in the session summary; don't fix
silently — Yuka decides scope.)

## Tests

Add to `philharmonic-workflow/tests/` (or extend an existing test
file if one already exercises `execute_step` and admits a small
addition). The two changes share the same capture-mock harness;
cover both in the same test file.

**`instance` field:**

1. **Field presence & shape.** `arg.instance` exists;
   `arg.instance.id` is a JSON string equal to
   `instance_id.public().as_uuid().to_string()`;
   `arg.instance.step` is a JSON number equal to `step_seq`.
2. **Step increments across calls.** Two consecutive
   `execute_step` calls on the same instance produce
   `step = N` then `step = N+1` (where N is whatever the
   harness produces on the first call given its instance-
   creation shape).
3. **Consistency with the step record.** The `step_seq` value
   the engine assigns to the new `StepRecord` equals
   `arg.instance.step` for the same call. (Compare against the
   landed StepRecord's `step_seq` if the harness exposes it;
   otherwise this is implicit in (1).)
4. **`instance.id` is the public V4, not the internal V7.**
   Construct a `SubjectContext` whose `tenant_id`'s
   `EntityId::internal()` and `EntityId::public()` are
   distinct (the normal case — V7 and V4 differ by
   construction). Assert `arg.instance.id` matches `public()`,
   NOT `internal()`.

**`subject` flattening:**

5. **`subject.tenant_id` is a string equal to the public V4.**
   Not an object; not the internal V7.
6. **`subject.authority_id` for principal callers is JSON
   `null`.** Build a principal `SubjectContext`
   (`authority_id: None`); assert the field is present and
   `is_null()`.
7. **`subject.authority_id` for ephemeral callers is a string
   equal to the authority's public V4.** Build an ephemeral
   `SubjectContext` with an authority entity-id; assert the
   field is a string and equals
   `authority.public().as_uuid().to_string()`.
8. **`subject.kind`, `subject.id`, `subject.claims` pass
   through unchanged.** A small round-trip check that the
   three pass-through fields keep their shape and content.

**Other-fields-unchanged guard:**

9. The four other existing fields (`context`, `args`, `input`,
   `data`) remain present in `script_arg` with their shapes
   unchanged. A quick `arg.get("context")` etc. existence
   check is enough — guards against accidental removal during
   the edit.

If the cleanest test seam is a tiny capture-mock `StepExecutor`,
implement it inline in the test file:

```rust
struct CaptureExecutor {
    captured: Mutex<Option<JsonValue>>,
}

#[async_trait::async_trait]
impl StepExecutor for CaptureExecutor {
    async fn execute(
        &self,
        _script: &str,
        arg: &JsonValue,
        _config: &JsonValue,
    ) -> Result<JsonValue, StepExecutionError> {
        *self.captured.lock().unwrap() = Some(arg.clone());
        Ok(json!({ "output": {}, "context": {}, "done": false }))
    }
}
```

(`Mutex::lock().unwrap()` is fine in test code — §10.3's
no-panics rule scopes to library `src/`, not tests.)

## Doc updates

### `docs/design/07-workflow-orchestration.md`

- Lines 241-242: `{context, args, input, subject, data}` →
  `{context, args, input, subject, data, instance}` and a short
  parenthetical "`instance` carries `{id, step}` — the running
  workflow instance's public V4 UUID and the step seq currently
  executing."
- Line 282: "five-field argument" → "six-field argument".
- Line 285: the destructured signature example —
  `function main({context, args, input, subject, data, instance})`.
- Lines 316-332: the existing `subject` shape example uses
  stale field names (`tenant` / `authority` as plain strings —
  doesn't match the actual code, which uses `tenant_id` /
  `authority_id`). Rewrite the example to the **new** flattened
  shape:

  ```javascript
  {
      kind: "ephemeral",  // or "principal"
      id: "opaque-subject-id",
      tenant_id: "00000000-0000-4000-8000-000000000000",
        // tenant's public V4 UUID
      authority_id: "11111111-1111-4111-8111-111111111111",
        // minting authority's public V4 UUID;
        // null for kind="principal"
      claims: {
          // Free-form, tenant-defined for ephemeral subjects;
          // typically empty or minimal for principals.
          user_id: "u_12345",
          locale: "ja-JP"
      }
  }
  ```

  Preserve the prose around the block (the "Scripts that don't
  care…" sentence at line 334 onwards stays).

- Line 353: "Assemble executor arg: `{context, args, input,
  subject, data, instance}`."
- Lines 492-493: the historical-expansion bullet ends at
  "`subject`". Append two new bullets to the same list:
  - "Then expanded again to add `instance: {id, step}` so
    scripts can know which workflow instance and step they
    are executing inside (2026-05-19)."
  - "Concurrently, `subject.tenant_id` and
    `subject.authority_id` were flattened from
    `{internal, public}` objects to bare public V4 UUID
    strings (2026-05-19) — the internal V7 UUID is not a
    script-facing identifier."

### `docs/design/06-execution-substrate.md`

- Line 104: the script signature example —
  `function main({context, args, input, subject, data, instance})`.
- Line 324: the "no instance ID, no correlation" line. Rework
  so the statelessness invariant stays accurate without
  contradicting the new field. Suggested replacement: "The
  executor itself remains stateless across calls — it receives
  a script + arg + config triple per invocation and retains
  nothing across invocations. The `arg.instance` field carries
  the workflow instance UUID and step seq as inputs to the
  script, but those are call-time identifiers, not
  executor-held state." Adapt to the surrounding paragraph's
  flow; don't paste verbatim if it reads awkward.

### `docs/guide/workflow-authoring.md`

- §"Script argument" block (around lines 717-727): extend the
  code block to include the `instance` field, and add prose:

  > `subject.tenant_id` is the tenant's public V4 UUID
  > (string). `subject.authority_id` is the minting authority's
  > public V4 UUID (string) for ephemeral callers, or `null`
  > for principal callers.
  >
  > `instance.id` is the `WorkflowInstance`'s public V4 UUID
  > (string); `instance.step` is the step seq (number, 1-based)
  > currently executing. Useful for log correlation and
  > idempotency-key construction.

  Match the file's existing voice (declarative, short
  sentences).

- Grep the file for `subject.tenant_id` / `tenant_id.public` /
  `subject.authority_id` / `authority_id.public` in the script-
  example snippets — if any example reads either field as an
  object, update it to the bare-string form.

## Version policy

**No bump.** `philharmonic-workflow/Cargo.toml` stays `0.1.6`.
The change rides on the next published version (whenever Yuka
cuts it). Add the entry to the existing `## [Unreleased]`
section in `philharmonic-workflow/CHANGELOG.md`:

```markdown
## [Unreleased]

### Added
- `script_arg.instance: { id, step }` — workflow scripts now
  receive the running `WorkflowInstance`'s public V4 UUID
  (string) and the step seq (number, 1-based) currently
  executing. Non-breaking: scripts that don't read
  `arg.instance` are unaffected.

### Changed
- **Breaking (script-arg shape):** `subject.tenant_id` and
  `subject.authority_id` are now bare public V4 UUID strings
  (or `null` for `authority_id` on principal callers), not
  `{internal, public}` objects. The previous nested shape was
  a serde default leak of the internal V7 UUID into JS
  scripts; the public V4 is the only identity scripts should
  observe. Scripts that read `arg.subject.tenant_id.public`
  must now read `arg.subject.tenant_id` directly. The
  persisted `StepRecordSubject` (audit shape) is unchanged.
```

**Do NOT bump** any other crate; **do NOT publish**. Yuka
publishes via `./scripts/publish-crate.sh` after review.

## Verification (mandatory before declaring done)

Run, once, at the end:

```sh
./scripts/pre-landing.sh
```

Must print `=== pre-landing: all checks passed ===`. The script
auto-detects modified crates (`philharmonic-workflow` here) and
runs fmt + check + clippy `-D warnings` + rustdoc + workspace
test + per-crate `--ignored` test phase. No raw `cargo fmt` /
`cargo clippy` / `cargo test` — `pre-landing.sh` covers them with
the right `CARGO_TARGET_DIR`.

If the box becomes contended mid-pre-landing, check with
`./scripts/xtask.sh resource-pressure` and back off if
`load1/cpus` climbs well above 1.0.

## Hand-off shape: Codex does not commit

**Leave the working tree dirty.** Claude commits via
`./scripts/commit-all.sh` after reviewing the diff. The script
has a `codex-guard` (`scripts/lib/codex-guard.sh`) that walks
the ancestor process chain and aborts if any process is named
`*codex*`; calling `commit-all.sh` from inside a Codex run will
hard-fail. Do not work around the guard.

Specifically:

- Do **not** run `./scripts/commit-all.sh` (any flags, including
  `--dry-run`, `--parent-only`, `--exclude`).
- Do **not** run raw `git commit` / `git push` / `git add`. The
  pre-commit hooks enforce signoff + signature + `Audit-Info:`
  trailer; the codex-guard fires from those hooks too.
- Do **not** run `git commit --no-verify` / `--no-gpg-sign`.
- Do **not** run `git reset` / `git rebase` / `git amend`.
  History is append-only.
- Do **not** run `./scripts/push-all.sh`. Claude pushes after
  reviewing.
- Do **not** run `./scripts/publish-crate.sh`. Yuka publishes.
- Do **not** edit `HUMANS.md`. Agent-readable, agent-writable
  forbidden.

Edits land in the working tree across:

- `philharmonic-workflow/` submodule — `src/engine.rs`,
  `tests/<new-or-extended>`, `CHANGELOG.md`.
- Parent repo — `docs/design/06-execution-substrate.md`,
  `docs/design/07-workflow-orchestration.md`,
  `docs/guide/workflow-authoring.md`, and `Cargo.lock`
  (regenerated automatically if `cargo build` ran).

Codex's session summary should mention which submodules + the
parent have dirty trees so Claude knows where to look.

## Codex report (encouraged)

If anything non-obvious surfaced during this round — a design
call on UUID serialisation, a choice of test seam, an edge case
in `step_seq` semantics, an unexpected interaction with the
existing test harness — write a short report to
`docs/codex-reports/2026-05-19-0001-script-arg-instance-field.md`
per [`docs/codex-reports/README.md`](../codex-reports/README.md).
Routine specified-and-shipped work doesn't need one; the session
summary covers it. Leave the report **dirty** in the working
tree; Claude commits it alongside the implementation diff.

If you skip the report, say so in the session summary.

## Outcome

Implemented. Files touched: `philharmonic-workflow/src/engine.rs`,
`philharmonic-workflow/src/subject.rs`,
`philharmonic-workflow/tests/engine_mock.rs`,
`philharmonic-workflow/CHANGELOG.md`,
`docs/design/07-workflow-orchestration.md`,
`docs/design/06-execution-substrate.md`,
`docs/guide/workflow-authoring.md`, and this prompt archive.
The `instance` field was added at
`philharmonic-workflow/src/engine.rs:249`; `SubjectContext::to_script_value`
now uses an inline `json!{...}` object that emits only public V4
UUID strings for script-facing IDs. Tests extend the existing
`MockExecutor` capture seam in `philharmonic-workflow/tests/engine_mock.rs`
and cover instance shape, step increment, step-record consistency,
principal subject null authority, and ephemeral subject public-ID
flattening. Consumer grep found no in-tree JS reads of
`arg.subject.tenant_id.public`, `arg.subject.tenant_id.internal`,
`arg.subject.authority_id.public`, or
`arg.subject.authority_id.internal`; the only broader stale-looking
doc-text hit is the lowerer proposal sketch at
`docs/crypto/proposals/2026-04-30-phase-9-config-lowerer.md:85`,
which is outside this dispatch's script-argument scope and was left
unchanged. Blockers: none; residual risk is downstream workflow
scripts outside this checkout that still read the old nested subject
shape. Hand-off heads: parent `3f71fc4`, `philharmonic-workflow`
`1324edd`.

---

<task>
Two coupled edits to the JSON value assembled in
`philharmonic-workflow::WorkflowEngine::execute_step`
([`philharmonic-workflow/src/engine.rs:243-249`](philharmonic-workflow/src/engine.rs#L243-L249)),
the shape JS workflow scripts receive as their default-export
function argument.

**(1) Add a sixth top-level field `instance: { id, step }`.**
`id` is the running `WorkflowInstance`'s **public V4** UUID
formatted as a hyphenated lowercase string
(`instance_id.public().as_uuid().to_string()`); `step` is the
`step_seq` value the engine computes at the top of
`execute_step` (1-based, the seq the engine will assign to the
step record being created).

**(2) Flatten `subject.tenant_id` and `subject.authority_id` to
bare public V4 UUID strings** by rewriting
`SubjectContext::to_script_value`
([`philharmonic-workflow/src/subject.rs:33-35`](philharmonic-workflow/src/subject.rs#L33-L35))
so it stops delegating to `serde_json::to_value(self)` and
instead builds the JSON shape explicitly. `tenant_id` becomes a
string; `authority_id` becomes `Option<String>` → JSON
`null | "<v4>"`. The other three fields (`kind`, `id`,
`claims`) pass through unchanged. Per Yuka 2026-05-19 — "no
internal ID exposure to scripts". The persisted
`StepRecordSubject` audit shape is **not** changed.

The `StepExecutor` trait signature does **not** change. Both
edits travel inside the existing `arg: &JsonValue` parameter.

**Reference docs (authoritative if they contradict this prompt):**

1. `docs/design/07-workflow-orchestration.md` lines 241-242,
   282-285, 353, 492-493.
2. `docs/design/06-execution-substrate.md` lines 104, 324.
3. `docs/guide/workflow-authoring.md` §"Script argument".
4. `CONTRIBUTING.md` §§4, 5, 10.3, 11, 14.6 plus the banned-dep
   posture in `deny.toml` (no new deps expected — none of the
   listed banned crates are at risk here).
5. The full preamble above (this prompt's `## …` sections;
   especially "Shape (locked)", "Hard requirements", "Tests",
   "Doc updates", "Version policy" = NO BUMP, "Verification").

**Hard constraints (locked):**

- **No internal V7 UUID exposure to scripts.** Every UUID
  reachable from `script_arg` is the public V4 form
  (`EntityId::public().as_uuid()`). This covers `instance.id`,
  `subject.tenant_id`, and `subject.authority_id`.
- `instance.id` serialises as a JSON string (default `Uuid`
  formatting); `instance.step` serialises as a JSON number.
- `instance` is the sixth and only new top-level field; do not
  add `status` / `template_id` / `tenant_id` / `correlation_id`
  on `instance`.
- `subject.tenant_id` is a bare JSON string (never an object,
  never `null`). `subject.authority_id` is a JSON string or
  `null`. Do **not** modify `SubjectContext`'s public
  `Serialize` impl — the wire-shape change is local to
  `to_script_value`.
- `StepExecutor` trait signature unchanged.
- `StepRecordSubject` persistence shape unchanged — audit
  confinement stays at `kind` + `id` + `authority_id` only,
  and `authority_id` keeps its full `EntityId` serialisation
  there.
- No panics in `philharmonic-workflow/src/` per
  CONTRIBUTING.md §10.3. Tests exempt.
- **No version bump.** `philharmonic-workflow/Cargo.toml` stays
  at `0.1.6`. CHANGELOG entry goes under the existing
  `## [Unreleased]` section, NOT a new dated heading. Note both
  the additive `instance` change AND the breaking `subject`
  flattening in that entry.
- **No publish.** Yuka publishes after review.
- During this round, grep the workspace for existing
  `subject.tenant_id.public` / `.internal` / `authority_id.public`
  / `.internal` usage in JS examples, doc text, and any in-tree
  script samples. Surface findings in the session summary; do
  not silently rewrite consumer code outside this prompt's
  scope (the framework's `philharmonic/webui/` is owned
  elsewhere — flag and stop).

**Per-file scope (the full set of edits):**

- `philharmonic-workflow/src/engine.rs` — add `"instance":
  json!({ "id": <public-v4 string>, "step": <step_seq number> })`
  to the `script_arg` `json!{...}` at line 243. Use
  `instance_id.public().as_uuid()`, NOT `.internal()`.
- `philharmonic-workflow/src/subject.rs` — rewrite
  `SubjectContext::to_script_value` to build the JSON shape
  explicitly (inline `json!{...}` or a private wire struct).
  Flatten `tenant_id` and `authority_id` to bare public V4
  strings per the preamble.
- `philharmonic-workflow/tests/<file>` — add shape coverage
  for both edits per the Tests block in the preamble (use a
  capture-mock `StepExecutor` if no existing harness fits).
- `philharmonic-workflow/CHANGELOG.md` — add an `### Added`
  bullet and a `### Changed` bullet under the existing
  `## [Unreleased]` section; mark the subject flattening as
  breaking.
- `docs/design/07-workflow-orchestration.md` — update the four
  script-arg references (lines 241-242, 282-285, 353,
  492-493) per the preamble. Mention the subject reshape if
  the file describes the `subject` field's wire shape.
- `docs/design/06-execution-substrate.md` — update the
  signature example (line 104) and rework the statelessness
  paragraph (line 324) per the preamble.
- `docs/guide/workflow-authoring.md` — extend the §"Script
  argument" code block (both `instance` and the new flat
  `subject` shape) and add prose describing both. Sweep the
  file's example snippets for `tenant_id.public` /
  `tenant_id.internal` / `authority_id.public` /
  `authority_id.internal` and rewrite to the new shape.

**Verification (must run + pass before declaring done):**

- `./scripts/pre-landing.sh` — clean.

That is the entire mandatory verification surface for this
scope.

<default_follow_through_policy>
Codex is expected to land the engine edit, tests, CHANGELOG,
and all three doc updates in this single round. "Engine done,
docs pending" or "engine + docs done, tests pending" is **not**
a complete result — keep going.

If a hard blocker surfaces (e.g. the existing test harness's
shape makes a clean capture-mock impossible without invasive
refactoring), **STOP and report the blocker before partial
landing**. A partial result that mixes the engine change with
unfinished tests is worse than a clean "blocker found, here's
what I'd recommend" report.

If `pre-landing.sh` fails on something orthogonal (a
pre-existing flake unrelated to this change), **fix forward**
only if the fix is mechanical and local. If it's structural,
**STOP and report**.
</default_follow_through_policy>

<completeness_contract>
"Complete" means:

1. `philharmonic-workflow/src/engine.rs::execute_step` builds
   `script_arg` with the new `instance` field present, shaped
   `{ id: <public-v4 string>, step: <number> }`, using
   `instance_id.public().as_uuid()`.
2. `philharmonic-workflow/src/subject.rs::to_script_value` is
   rewritten to flatten `tenant_id` and `authority_id` to bare
   public V4 strings (or `null` for `authority_id` on principal
   callers). The public `Serialize` impl on `SubjectContext` is
   NOT modified.
3. `philharmonic-workflow/tests/` covers the full Tests matrix
   from the preamble:
   - `instance` field: presence/shape, step increment,
     consistency with the step record, and public-V4-not-V7
     assertion.
   - `subject` flattening: `tenant_id` as V4 string,
     `authority_id` JSON `null` for principal callers,
     `authority_id` as V4 string for ephemeral callers,
     pass-through for `kind`/`id`/`claims`.
   - Other fields (`context`/`args`/`input`/`data`) remain
     present.
4. `philharmonic-workflow/CHANGELOG.md` has both new bullets
   (`### Added` for `instance`, `### Changed` for the
   breaking `subject` reshape) under the existing
   `## [Unreleased]` section. **No new dated heading; no
   version bump.**
5. `philharmonic-workflow/Cargo.toml` is unchanged (`version =
   "0.1.6"` stays).
6. `docs/design/07-workflow-orchestration.md` has the four
   script-arg references updated; subject reshape noted if the
   file describes the `subject` wire shape.
7. `docs/design/06-execution-substrate.md` has the signature
   example + statelessness paragraph updated.
8. `docs/guide/workflow-authoring.md` §"Script argument" has
   the code block extended (both `instance` and flat
   `subject`), descriptions added, and any `tenant_id.public`
   / `authority_id.public` example snippets rewritten to the
   new shape.
9. `./scripts/pre-landing.sh` passes.
10. Working tree left dirty across `philharmonic-workflow/` +
    parent. **No commits, no pushes** — Claude commits and
    pushes after reviewing the diff.
11. Session summary lists which submodule + the parent have
    dirty trees so Claude can scope the `commit-all.sh` run.
    The summary also lists any external consumers of
    `arg.subject.tenant_id` / `arg.subject.authority_id` found
    via repo-wide grep, flagged for Yuka.
12. Outcome section of this prompt file updated with: (a)
    list of files touched, (b) the line in `engine.rs` where
    the `instance` field was added, (c) the
    `to_script_value` implementation approach taken
    (`json!{...}` vs private wire struct), (d) test seam
    chosen, (e) consumer-grep findings, (f) any blockers,
    (g) residual risks, (h) submodule + parent head SHAs at
    hand-off.

If any of (1)–(11) is incomplete, the dispatch is INCOMPLETE.
Report INCOMPLETE clearly with what's done and what's left,
and STOP — don't synthesise a half-result.
</completeness_contract>

<verification_loop>
During implementation (between rounds of edits):

  CARGO_TARGET_DIR=target-main cargo check -p philharmonic-workflow --all-targets

Per-crate tests:

  CARGO_TARGET_DIR=target-main cargo test -p philharmonic-workflow --all-targets

Final, single run:

  ./scripts/pre-landing.sh

If `pre-landing.sh` fails, read the failure carefully:

1. If a clippy / doctest / test in `philharmonic-workflow`
   caused it, that's a local fix — make the fix, re-run
   pre-landing.
2. If it's a workspace-wide failure (e.g. a downstream crate
   no longer compiles because of an unintended trait/type
   change), back the change out of that crate, re-run.
   `philharmonic-workflow` is a published-crate dep; downstream
   `philharmonic-api` and the bins consume it.
3. If you're tight-looping pre-landing.sh on a slow box, run
   `./scripts/xtask.sh resource-pressure` first to confirm the
   host has headroom; back off if it doesn't.

Do not run raw `cargo fmt` / `cargo clippy` / `cargo test` —
`pre-landing.sh` covers them with the right `CARGO_TARGET_DIR`
and feature flags.

Do not run `cargo build --workspace` standalone as a "check" —
the per-crate `cargo check -p` covers it without the full
link-time cost.
</verification_loop>

<missing_context_gating>
Before you start editing, the workspace state must match the
prompt's claims:

  ./scripts/status.sh

Should print `(clean)` for the parent repo and all submodules.
If it doesn't, **STOP and report**. The prompt assumes a clean
starting tree — uncommitted changes in unrelated submodules
mean someone else is mid-edit; don't conflict.

If the `## [Unreleased]` heading in
`philharmonic-workflow/CHANGELOG.md` has already collected
unrelated bullets that don't appear in `git status`, treat them
as in-progress work and append your bullet beneath them without
disturbing them. If the section is empty (a single blank line
under the heading), add an `### Added` subheading then your
bullet, mirroring the file's previous-version subheading style.

If the existing test layout makes a capture-mock executor
genuinely impossible without restructuring the harness, **STOP
and report**. Don't refactor the existing test harness as part
of this dispatch — propose a follow-up shape instead.
</missing_context_gating>

<action_safety>
- **You do not commit.** Leave the working tree dirty across
  `philharmonic-workflow/` + parent. `./scripts/commit-all.sh`
  (any flags) and raw `git commit` / `git push` / `git add` /
  `git reset` / `git rebase` / `git amend` are all forbidden.
  The script's `codex-guard` will hard-abort if you try; the
  same guard fires from the pre-commit hooks. Claude commits +
  pushes after reviewing the diff.
- **Never** invoke `./scripts/push-all.sh`. Claude pushes.
- **Never** invoke `./scripts/publish-crate.sh`. Yuka publishes.
- **Never** edit `HUMANS.md`. Agent-readable, agent-writable
  forbidden.
- Every `cargo` invocation needs
  `CARGO_TARGET_DIR=target-main` (the wrappers in `scripts/`
  set this; if you call cargo directly, set it yourself).
- POSIX-ish host: no `bash`-only constructs in any shell you
  invoke. The wrappers are POSIX `#!/bin/sh`.
- The workspace's authoritative timezone is JST (Asia/Tokyo).
  Any wall-clock value you generate for the CHANGELOG or the
  codex-report belongs in JST; today is 2026-05-19 (Tue). The
  CHANGELOG entry goes under `## [Unreleased]` and does not
  carry a date.
</action_safety>

<structured_output_contract>
At the end of the dispatch, return:

1. **Summary** (2-3 sentences): the engine + subject sites
   touched, the test seam chosen, total new/changed lines
   split engine vs subject vs tests vs docs.
2. **Touched files**: full list, grouped by submodule + parent.
3. **`script_arg` diff**: paste the before/after of the
   `json!{...}` call at `engine.rs:243` so the reviewer can
   eyeball the new `instance` field's placement.
4. **`to_script_value` diff**: paste the before/after of
   `subject.rs::to_script_value` so the reviewer can confirm
   the flattening shape and that no other field's behaviour
   shifted.
5. **Test coverage**: number of new test cases, what each one
   asserts. Note whether you used a capture-mock or extended
   an existing harness. Confirm both the `instance` matrix
   and the `subject` matrix are covered.
6. **Doc updates**: list each of the three doc files with the
   specific anchors / line ranges touched. Call out any
   example-snippet rewrites in `workflow-authoring.md` that
   touched `subject.tenant_id` / `subject.authority_id`
   reads.
7. **CHANGELOG entry**: paste both bullets (the `### Added`
   `instance` bullet and the `### Changed` breaking-`subject`
   bullet) verbatim so the reviewer can confirm placement
   under `[Unreleased]`.
8. **Consumer-grep findings**: list any occurrences of
   `arg.subject.tenant_id.public` / `.internal` /
   `arg.subject.authority_id.public` / `.internal` found in
   the workspace (parent + submodules) — file path + line +
   one-line context. If none, say "no consumer reads found".
9. **Verification results**:
   - `pre-landing.sh`: PASS / FAIL (with one-line summary if
     FAIL).
10. **Working-tree state at hand-off**:
    - List which submodule + parent have dirty trees.
    - No commits expected from you. Claude will commit + push
      after reviewing the diff.
11. **Codex report**: if you wrote
    `docs/codex-reports/2026-05-19-0001-script-arg-instance-field.md`,
    note its presence (dirty in working tree; Claude commits
    it). If you skipped, say so.
12. **Residual risks**: anything you'd flag for Claude or
    Yuka before publish (e.g. a downstream consumer that
    will break on the `subject` flattening; an example in
    `workflow-authoring.md` that you weren't sure how to
    rewrite).
13. **Outcome paragraph** for the prompt-archive file: 4-6
    sentences summarising the round for posterity, ready to
    drop into `## Outcome` of this file.
</structured_output_contract>
</task>
