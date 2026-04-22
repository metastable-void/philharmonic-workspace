# `philharmonic-policy` authorization-boundary hardening closed

**Date:** 2026-04-22

## Outcome

Both authorization-boundary findings surfaced by an
independently-run Codex security review (report at
`docs/codex-reports/2026-04-22-0001-philharmonic-policy-security-review.md`,
committed 603b5f9) are now fixed and merged.

- **Finding #1 (High, cross-tenant role confusion):** fixed in
  submodule commit `1cde0e1`, parent `89dd590`. Evaluator now
  reads `role_revision.entity_attrs["tenant"]` and skips the
  role on mismatch with the caller's tenant. Silent `continue`
  matches the existing defensive-deny patterns for `is_retired`
  and membership-tenant mismatch.
- **Finding #2 (Medium, unvalidated permission atoms):** same
  commit. `PermissionDocument::deserialize` validates each
  atom against the existing `ALL_ATOMS` const at parse time.
  New typed error variant `PolicyError::UnknownPermissionAtom
  { atom: String }`. Call sites use a new
  `parse_permission_document(bytes)` helper that maps the
  serde error back to the typed variant.
- **Finding #3 (Low, transitive cargo-audit advisories on `rsa`
  / `paste`):** explicitly out of scope per your direction —
  workspace dependency-governance concern, not a policy-crate
  change. Advisories remain surfaced via
  `./scripts/cargo-audit.sh` on demand.

## Regression tests (Wave 1 / Wave 2 tier pattern)

- Tier 1 (unit, `permission.rs`): 3 new —
  `permission_document_rejects_unknown_atom_bare`,
  `permission_document_rejects_unknown_atom_wrapped`,
  `permission_document_accepts_empty_array`.
- Tier 3 (mock, `tests/permission_mock.rs`): 2 new —
  `permission_evaluation_role_tenant_mismatch_denied` (exact
  Finding #1 scenario) and
  `permission_evaluation_role_tenant_mismatch_skips_to_legit_role`.
  The skip-to-legit test contains a subtle invariant assertion:
  the malformed role is seeded with a permissions-blob hash
  that's deliberately absent from the content store. If the
  tenant check fired AFTER the permissions-blob lookup, the
  test would hit `MissingPermissionsBlob` instead of returning
  `Ok(true)` — so green-result proves the tenant check runs
  before blob fetch, which is the cheap-path-first property
  you'd want.
- Tier 2 (MySQL, `tests/permission_mysql.rs`): 1 new —
  `permission_evaluation_role_tenant_mismatch_denied_end_to_end`.

All tiers green. Miri also clean on the 3 new unit + 2 new
mock tests under nightly 9ec5d5f32 (post-update).

## Known fragility to flag for future review

The Finding #2 fix uses a string-roundtrip pattern to recover
a typed error through serde's `Deserialize` trait:

```rust
// inside Deserialize impl:
return Err(serde::de::Error::custom(format!(
    "{UNKNOWN_PERMISSION_ATOM_PREFIX}{atom}"
)));

// in the wrapper helper:
if let Some(atom) = unknown_permission_atom_from_parse_error(&error) {
    return PolicyError::UnknownPermissionAtom { atom };
}
```

The constant `UNKNOWN_PERMISSION_ATOM_PREFIX` is private to
the module so only in-module code depends on the roundtrip.
It works today and is robust enough for the test suite — but
if `serde_json` ever changes the `custom` error's message
format (e.g. adds a structured location prefix that doesn't
start with our prefix), the `strip_prefix` falls through and
the typed variant stops being produced. You'd still get a
`PolicyError::PermissionDocumentParse` variant from the
catch-all arm — degraded UX rather than a correctness break.

Cleaner refactor if it becomes a concern: factor validation
out of the `Deserialize` impl and perform it explicitly after
parsing the raw `Vec<String>`. That requires either making
the `permissions` field `pub(crate)` constructible, or adding
a `PermissionDocument::new_validated(permissions) -> Result<Self,
PolicyError>` constructor. Not urgent; flagging so the option
is in your mental model at review time.

## Methodology observation — independent Codex reports pay off

The `docs/codex-reports/` journal directory landed earlier
today (convention in commit `1fe2fcd`). Its first real use —
an independently-run Codex security review — produced a
high-signal report whose three findings were all factually
accurate on Claude-side verification. Two were hardening gaps
I'd missed on my own passes over the crate; the third was
advisory noise but a legitimate observation.

Worth noting for future workflow: independent Codex runs
against the journal-convention stand on their own without
needing Claude-side re-prompting for structure. The prompt
that triggered this dispatch used the report verbatim as the
source spec, cross-referencing the report file from within the
prompt — future similar dispatches can follow the same pattern
(archive report → archive prompt → dispatch → fix).

## Gate-2 context

This work does **not** affect the pending Wave 2 Gate-2
review:

- `src/sck.rs` and `src/token.rs` are unchanged — the crypto
  construction Yuka signed up to review line-by-line is
  identical to what shipped at commit `2c98467`.
- The crate's version stays at `0.0.0`. Publish is still
  blocked by Gate-2 on the crypto code.
- The auth-boundary changes live in `src/evaluation.rs`,
  `src/permission.rs`, `src/error.rs` (new variant only),
  and the test files. Gate-2 reviewers will see the broader
  diff, but the crypto scope is bounded.

After Gate-2 clears, the `philharmonic-policy` 0.1.0 release
will include both the Wave 2 crypto and the auth-boundary
hardening together.
