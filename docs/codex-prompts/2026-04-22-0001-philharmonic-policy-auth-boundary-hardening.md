# `philharmonic-policy`: authorization-boundary hardening (Findings #1 + #2)

**Date:** 2026-04-22
**Slug:** `philharmonic-policy-auth-boundary-hardening`
**Round:** 01 (initial dispatch)
**Subagent:** `codex:codex-rescue`

## Motivation

A Codex-authored security review of `philharmonic-policy`
(archived at
[`docs/codex-reports/2026-04-22-0001-philharmonic-policy-security-review.md`](../codex-reports/2026-04-22-0001-philharmonic-policy-security-review.md))
surfaced two real authorization-boundary hardening gaps that
Claude-side verification confirmed:

- **Finding #1 (High):** cross-tenant role confusion in
  `evaluate_permission`. Principal tenant and membership tenant
  are checked, but the **role's own tenant is never verified**.
  A malformed or adversarial `RoleMembership` row that sets its
  own `tenant` field to victim tenant T1 while pointing its
  `role` slot at a privileged `RoleDefinition` in tenant T2
  will cause the evaluator to grant T2's role permissions
  inside T1.
- **Finding #2 (Medium):** `PermissionDocument::deserialize`
  accepts arbitrary string values and `contains` performs raw
  string equality. The canonical list `ALL_ATOMS` exists in
  `permission.rs` but is never consulted during parsing or
  membership checks. Forward-compatibility drift: a
  currently-unknown atom seeded today could silently become a
  privileged atom in a later release without any role-update
  review.

Both findings must be resolved **before** `philharmonic-policy`
ships at `0.1.0`. This dispatch adds the fixes + regression
tests. Version stays at `0.0.0`.

Finding #3 (transitive cargo-audit advisories on `rsa` /
`paste`) is **out of scope per Yuka** — tracked at the workspace
dependency-governance level, not as a policy-crate change.

## References

- [`docs/codex-reports/2026-04-22-0001-philharmonic-policy-security-review.md`](../codex-reports/2026-04-22-0001-philharmonic-policy-security-review.md)
  — the source report with attacker-minded reasoning and
  exploit sketch for Finding #1.
- [`docs/design/09-policy-and-tenancy.md`](../design/09-policy-and-tenancy.md) —
  §"Role membership" (lines 220–234) on the three-way binding,
  §"Permission atoms" (lines 235–361) on the closed atom
  vocabulary.
- [`philharmonic-policy/src/evaluation.rs`](../../philharmonic-policy/src/evaluation.rs)
  — evaluator to extend.
- [`philharmonic-policy/src/permission.rs`](../../philharmonic-policy/src/permission.rs)
  — parser + `ALL_ATOMS` to wire up.
- [`philharmonic-policy/src/entity.rs`](../../philharmonic-policy/src/entity.rs)
  — `RoleDefinition` already has the `tenant` entity slot
  (line 120-121); it just isn't read today.
- [`philharmonic-policy/tests/common/mock.rs`](../../philharmonic-policy/tests/common/mock.rs)
  — `MockStore` + seeders you'll extend.
- [`philharmonic-policy/tests/permission_mock.rs`](../../philharmonic-policy/tests/permission_mock.rs)
  — Wave 1 mock test tier; add regression tests alongside.
- [`philharmonic-policy/tests/permission_mysql.rs`](../../philharmonic-policy/tests/permission_mysql.rs)
  — MySQL tier; add a matching `#[ignore]`-gated integration
  test.
- `docs/design/13-conventions.md` — workspace conventions
  (§Panics and undefined behavior; §Error types; §Testing).
- `AGENTS.md` — Codex's standing rules (no commits, flag-vs-fix
  on crypto paths, `scripts/*.sh` over raw cargo).

## Scope

### In scope

**Finding #1 fix — role-tenant enforcement in
`evaluate_permission`:**

- After loading `role_revision` (currently at
  `evaluation.rs:84`), read `role_revision.entity_attrs["tenant"]`
  and compare to the `tenant` argument the caller passed.
- On mismatch, **`continue` to the next role** (defensive deny,
  matching the existing pattern for `is_retired` roles at line
  89 and membership tenant mismatch at line 64). Do not return
  an error — malformed data should not break authorization for
  other roles the principal legitimately holds.
- Reuse the existing `entity_attr` helper
  (`evaluation.rs:114`).
- No new `PolicyError` variant required; the skip is silent.

**Finding #2 fix — permission-atom validation at parse time:**

- Validate parsed permission strings against `ALL_ATOMS` during
  `PermissionDocument::deserialize` (the custom `Deserialize`
  impl at `permission.rs:82-94`).
- On an unknown atom, return a serde deserialization error.
  Use a typed error variant to make the failure observable at
  the caller:
  - Add `PolicyError::UnknownPermissionAtom { atom: String }`
    in `src/error.rs`, placed alphabetically next to the
    existing permission-related variants (keep the established
    variant ordering style — don't reorder other variants).
  - Map the serde error to this variant at the two call sites
    that deserialize permission documents (currently
    `evaluation.rs:105`). Use `serde_json::from_slice` →
    `.map_err(...)` → `PolicyError::UnknownPermissionAtom` or
    propagate via a new `From<serde_json::Error>` pathway —
    your call on which fits the existing error-propagation
    style in this crate better.
- Keep the two JSON shapes `PermissionDocumentWire` already
  supports (bare array and `{permissions: [...]}` wrapper).
  Validation applies identically to both.
- `PermissionDocument::contains` at `permission.rs:66-68`
  stays as raw string equality — with parse-time validation,
  every stored atom is guaranteed to be in `ALL_ATOMS`, so
  equality is sufficient. Don't add a second layer of
  validation in `contains`.

**New regression tests (three tiers, same pattern as Wave 1
and Wave 2):**

Tier 1 — unit tests in `permission.rs`'s `#[cfg(test)] mod
tests`:
- `permission_document_rejects_unknown_atom_bare` — parse
  `["workflow:template_read","totally:made_up"]`, expect
  deserialization failure.
- `permission_document_rejects_unknown_atom_wrapped` — same
  for `{"permissions":["workflow:template_read","totally:made_up"]}`.
- `permission_document_accepts_empty_array` — parse `[]`,
  expect `Ok` with empty permissions (empty is not "unknown").

Tier 3 — mock tests in
`tests/permission_mock.rs`:
- `permission_evaluation_role_tenant_mismatch_denied` —
  principal in tenant_a, role in tenant_b, membership's
  `tenant` slot set to tenant_a pointing `role` at tenant_b's
  role. `evaluate_permission(principal, tenant_a,
  "audit:read")` → `Ok(false)`. This is the exact Finding #1
  scenario.
- `permission_evaluation_role_tenant_mismatch_skips_to_legit_role`
  — principal has two memberships: one with the malformed
  tenant_a-membership-tenant_b-role pattern (should skip), one
  legit tenant_a membership/role pair granting the target
  atom. Expect `Ok(true)`. Proves the skip doesn't break
  legitimate auth.

Tier 2 — MySQL tests in
`tests/permission_mysql.rs` (`#[ignore]`-gated):
- `permission_evaluation_role_tenant_mismatch_denied_end_to_end`
  — mirror the Tier 3 skip-silently test against a real MySQL
  testcontainer. One test is enough at this tier; the other
  scenarios are exhaustively covered in Tier 3.

**Error variant additions (in `src/error.rs`):**

- `UnknownPermissionAtom { atom: String }` — see Finding #2
  above.
- **Nothing else.** No variant for Finding #1 — the silent
  skip means no new error path.

### Out of scope

- **Crypto code (`src/sck.rs`, `src/token.rs`).** Those are
  Wave 2 Gate-2 review territory. Do not touch, do not add
  tests that exercise them beyond what already exists.
- **Finding #3** (transitive `rsa` / `paste` advisories).
  Per Yuka, this is a workspace-level dependency-governance
  concern, not a policy-crate change. Do not attempt to
  restructure dependencies.
- **Admin-side API validation** of role create/update. The
  policy crate currently has no admin API (that's Phase 8 /
  `philharmonic-api`). Parse-time validation is the only
  enforcement surface available in this crate.
- **Version bump.** `philharmonic-policy/Cargo.toml` `version`
  stays at `"0.0.0"`. Claude bumps + publishes after this and
  the pending Wave 2 Gate-2 review both clear.
- **Commits / pushes / tags / branch ops.** Claude drives Git.
  Leave the working tree dirty.

## Acceptance criteria

Before Claude commits your work:

- `cargo fmt --check` clean on `philharmonic-policy` (run via
  `./scripts/rust-lint.sh philharmonic-policy`).
- `cargo check --workspace` passes.
- `cargo clippy --all-targets -- -D warnings` passes (via
  `rust-lint.sh`).
- `./scripts/rust-test.sh philharmonic-policy` passes. Expect
  the existing 12 mock tests plus the 2 new Tier-3 tests
  (total 14) plus the 3 new Tier-1 unit tests in `permission.rs`
  (existing 3 unit tests there stay, so 6 total).
- `./scripts/rust-test.sh --ignored philharmonic-policy`
  passes against MySQL testcontainers. Expect the existing 11
  MySQL tests plus 1 new (total 12).
- `cargo +nightly miri test -p philharmonic-policy` (via
  `./scripts/miri-test.sh philharmonic-policy`) still clean —
  no new UB introduced. The auth changes are pure in-memory
  logic, so miri should pass.
- No `.unwrap()` / `.expect()` added in library code outside
  the approved narrow-exception patterns (see
  `docs/design/13-conventions.md §Panics and undefined behavior`).
- No `unsafe`, no `anyhow`, no `println!` / `eprintln!` /
  `tracing` in library code.
- `philharmonic-policy/Cargo.toml` `version` unchanged
  (`"0.0.0"`).

## Flag-vs-fix

Per AGENTS.md §Crypto-sensitive paths and the
crypto-review-protocol, flag rather than fix if you encounter:

- Issues in `src/sck.rs` or `src/token.rs` (crypto
  construction) — those are Yuka's Gate-2 scope.
- `unsafe` blocks in neighboring code.
- Zeroization gaps you notice.
- Test vectors that don't match (won't apply here — no new
  crypto vectors in this dispatch).

## Final summary format

When you finish, write a session-end summary covering:

- **File map**: every file created or modified, one line each.
- **API delta**: the new `PolicyError::UnknownPermissionAtom`
  variant's exact signature.
- **Per-tier test results**: Tier 1 (unit), Tier 3 (mock),
  Tier 2 (MySQL `--ignored`) — pass counts.
- **miri status**: `./scripts/miri-test.sh philharmonic-policy`
  pass/fail.
- **Flag list**: empty if nothing surfaced; otherwise itemize.
- If the findings map to anything non-obvious you want
  preserved past this session, write a
  `docs/codex-reports/YYYY-MM-DD-NNNN-<slug>.md` entry and
  mention its path in the summary (per
  AGENTS.md §Reports). A follow-up report is optional for
  this dispatch — only write one if you observed something
  worth archiving beyond what the session summary covers.

---

## Prompt (verbatim text to send to Codex)

<task>
Implement two authorization-boundary hardening fixes in the `philharmonic-policy` crate, plus regression tests. Full spec at `docs/codex-prompts/2026-04-22-0001-philharmonic-policy-auth-boundary-hardening.md` in this repo — **read it verbatim before touching code**. Cross-referenced source at `docs/codex-reports/2026-04-22-0001-philharmonic-policy-security-review.md` (the security review whose findings this dispatch addresses).

The two fixes:

1. **Finding #1 (cross-tenant role confusion):** in `philharmonic-policy/src/evaluation.rs`, the `evaluate_permission` function checks the principal's tenant (lines 26-29) and the membership's tenant (lines 62-65), but never verifies the role's own tenant. `RoleDefinition` carries a `tenant` entity slot (`src/entity.rs:120-121`) that must match the caller's requested tenant. Add the check after loading `role_revision` (currently at line 84): read `role_revision.entity_attrs["tenant"]` via the existing `entity_attr` helper and compare to the `tenant` argument. On mismatch, `continue` to the next role — defensive-skip matching the existing `is_retired` and membership-tenant patterns. No new `PolicyError` variant.

2. **Finding #2 (atom validation):** in `philharmonic-policy/src/permission.rs`, the custom `Deserialize` impl for `PermissionDocument` (lines 82-94) accepts arbitrary strings. Validate each parsed atom against the existing `ALL_ATOMS` const (line 35). On an unknown atom, return a deserialization error typed as `PolicyError::UnknownPermissionAtom { atom: String }` (new variant in `src/error.rs`, placed alphabetically near the existing permission-related variants). Both JSON wire shapes (bare array, `{permissions: [...]}` wrapper) apply the same validation. `PermissionDocument::contains` stays as raw string equality — parse-time validation makes that safe.

Regression tests (three tiers, mirroring Wave 1 and Wave 2 discipline):

- Tier 1 (unit) in `permission.rs`'s `#[cfg(test)] mod tests`: `permission_document_rejects_unknown_atom_bare`, `permission_document_rejects_unknown_atom_wrapped`, `permission_document_accepts_empty_array`.
- Tier 3 (mock) in `tests/permission_mock.rs`: `permission_evaluation_role_tenant_mismatch_denied` (the exact Finding #1 scenario — membership tenant set to caller tenant, role in a different tenant — expect `Ok(false)`) and `permission_evaluation_role_tenant_mismatch_skips_to_legit_role` (two memberships, one malformed, one legit — expect the legit one still grants).
- Tier 2 (MySQL, `#[ignore]`-gated) in `tests/permission_mysql.rs`: `permission_evaluation_role_tenant_mismatch_denied_end_to_end`. One test at this tier; Tier 3 covers the exhaustive cases.

Error variant: add `PolicyError::UnknownPermissionAtom { atom: String }` in `src/error.rs`. Nothing else new.

**Out of scope:**
- `src/sck.rs` and `src/token.rs` (crypto construction — Wave 2 Gate-2 review scope; do not touch).
- Finding #3 (transitive `rsa` / `paste` cargo-audit advisories) — workspace dependency governance, not a policy-crate change.
- Admin-side API validation (no admin API in this crate yet; Phase 8 work).
- `Cargo.toml` version bump — stays at `"0.0.0"`.
- Commits / pushes / tags — leave the working tree dirty; Claude drives Git via `scripts/*.sh`.

**Hard constraints** (from `docs/design/13-conventions.md` and `AGENTS.md`):
- No `.unwrap()` / `.expect()` on `Result` / `Option` in library code outside the narrow-exception patterns already present in this crate (OS RNG `.expect` in `sck.rs` / `token.rs` — do not touch).
- No `panic!` / `unreachable!` / `todo!` / `unimplemented!` on reachable paths.
- No unbounded indexing; use `.get(...)` / `.first()` / `.last()` → `Option`.
- No unchecked integer arithmetic on untrusted input; use `checked_*` / `saturating_*` / `wrapping_*` to declare intent.
- No `unsafe`, no `anyhow`, no `println!` / `eprintln!` / `tracing` in library code.

**Acceptance gates** (must be green before the session ends):
- `./scripts/rust-lint.sh philharmonic-policy` — fmt + check + clippy `-D warnings`.
- `./scripts/rust-test.sh philharmonic-policy` — workspace-skipping-ignored; expect 3 new Tier-1 unit tests + 2 new Tier-3 mock tests + all Wave 1 / Wave 2 tests still green.
- `./scripts/rust-test.sh --ignored philharmonic-policy` — MySQL testcontainer; expect 1 new Tier-2 test + all Wave 1 / Wave 2 ignored tests still green (total 12).
- `./scripts/miri-test.sh philharmonic-policy` — no new UB (pure logic changes should stay miri-clean; Wave 2 crypto already passes).

**Git handling**: do not run any state-changing git command. Leave the working tree dirty. Claude reviews and commits via `./scripts/commit-all.sh` + `./scripts/push-all.sh`.

When done, write a short summary covering: the file map (created / modified); the final signature of `PolicyError::UnknownPermissionAtom`; per-tier test results (Tier 1 unit, Tier 3 mock, Tier 2 MySQL); miri pass/fail; the flag list (empty if nothing surfaced). If something substantial surfaced during implementation that the summary won't capture, write a `docs/codex-reports/2026-04-22-NNNN-<slug>.md` entry and cite its path — per AGENTS.md §Reports — but only if it's actually worth archiving. Don't commit. Don't push.
</task>
