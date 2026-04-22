# `philharmonic-policy` security review (deep pass)

**Date:** 2026-04-22  
**Prompt:** docs/codex-prompts/2026-04-21-0002-phase-2-wave-2-crypto-foundation.md

## Context

This report captures a focused security review requested in-session for
the workspace member crate `philharmonic-policy`. The review covered:

- Authorization evaluation logic in `philharmonic-policy/src/evaluation.rs`.
- Permission document parsing and atom handling in
  `philharmonic-policy/src/permission.rs`.
- Endpoint-config crypto helpers in `philharmonic-policy/src/sck.rs`.
- Long-lived token generation/parsing in `philharmonic-policy/src/token.rs`.
- Mock and MySQL integration tests under `philharmonic-policy/tests/`.
- Dependency advisory state via `./scripts/cargo-audit.sh`.

## Review approach

I treated this as an attacker-minded pass across trust boundaries:

1. Read all crate source files and test files.
2. Compared implementation behavior against
   `docs/design/09-policy-and-tenancy.md`.
3. Executed lint/tests with project wrappers:
   - `./scripts/rust-lint.sh philharmonic-policy`
   - `./scripts/rust-test.sh philharmonic-policy`
   - `./scripts/rust-test.sh --ignored philharmonic-policy`
4. Ran `./scripts/cargo-audit.sh` to capture known dependency advisories.

All tests and lint checks passed. Findings below are logic and
hardening findings, not build failures.

## Findings

### 1) High severity: cross-tenant role confusion in permission evaluation

**Summary.**  
`evaluate_permission` verifies principal tenant and membership tenant, but
does not verify that the referenced `RoleDefinition` is in the same
tenant before consuming its `permissions` slot.

**Code locations.**

- Principal tenant gate is present:
  `philharmonic-policy/src/evaluation.rs` (lines 26-29).
- Membership tenant gate is present:
  `philharmonic-policy/src/evaluation.rs` (lines 62-65).
- Role is loaded and used, but role tenant is never checked:
  `philharmonic-policy/src/evaluation.rs` (lines 71-109).

**Why this matters.**  
The design describes `RoleMembership` as a three-way binding across
principal, role, and tenant, with tenant redundant but expected to match
both principal and role tenant:
`docs/design/09-policy-and-tenancy.md` (lines 231-233).

If a malformed or malicious `RoleMembership` row references a role from
another tenant while setting its own `tenant` field to the caller tenant,
the current evaluator can grant permissions based on that foreign role.

**Exploit sketch.**

1. Attacker obtains ability to insert/alter a `RoleMembership` row
   (through bug, weak admin path, or direct store access).
2. `RoleMembership.tenant` is set to victim tenant `T1`.
3. `RoleMembership.role` points to privileged role in tenant `T2`.
4. `evaluate_permission(..., tenant=T1, ...)` will accept membership and
   then read the foreign role permissions.
5. Authorization can be incorrectly granted inside `T1`.

**Current test gap.**  
Tests cover principal cross-tenant denial, but not role-tenant mismatch in
membership:
`philharmonic-policy/tests/permission_mock.rs` (`permission_evaluation_cross_tenant_denied`).

---

### 2) Medium severity: permission docs accept arbitrary atom strings

**Summary.**  
`PermissionDocument` accepts any `Vec<String>` value from JSON and
`contains` performs raw membership check. There is no validation against
the canonical atom list `ALL_ATOMS`.

**Code locations.**

- Wire parsing accepts arbitrary strings:
  `philharmonic-policy/src/permission.rs` (lines 75-93).
- Membership check is raw string equality:
  `philharmonic-policy/src/permission.rs` (lines 66-68).

**Why this matters.**  
The design states atom vocabulary is deployment-visible and not intended
for tenant-level extension:
`docs/design/09-policy-and-tenancy.md` (lines 359-361).

Allowing arbitrary opaque strings can create upgrade-time privilege
surprises. Example: a currently-unknown atom string seeded today may
become a valid privileged atom in a later release without requiring role
edits.

**Risk class.**  
This is primarily forward-compatibility privilege drift, not immediate
remote code execution. Severity is medium due to policy integrity impact.

---

### 3) Low severity (supply-chain visibility): advisory in transitive graph

**Summary.**  
`./scripts/cargo-audit.sh` reports:

- `RUSTSEC-2023-0071` on `rsa` (`Marvin Attack` timing sidechannel),
  reachable through `sqlx-mysql`.
- `RUSTSEC-2024-0436` (`paste` unmaintained) warning.

**Scope detail.**  
For `philharmonic-policy`, this `rsa` path is via MySQL/sqlx usage (not a
direct cryptographic use in this crate’s own business logic). It is still
worth tracking at workspace dependency governance level.

**Relevant manifest area.**

- `philharmonic-policy/Cargo.toml` dev/runtime dependency layout (lines
  11-31).

## Areas reviewed with no direct vulnerability found

- `sck.rs` uses AES-256-GCM with authenticated AAD binding
  (`tenant_id`, `config_uuid`, `key_version`) and versioned wire format.
- `token.rs` token parsing enforces prefix and exact encoded/decoded
  lengths before hashing.
- Library code paths reviewed in this crate avoid panic-prone
  `.unwrap()`/`.expect()` on reachable paths (exceptions were in tests).

## Test and verification log

- `./scripts/rust-lint.sh philharmonic-policy`: pass.
- `./scripts/rust-test.sh philharmonic-policy`: pass.
- `./scripts/rust-test.sh --ignored philharmonic-policy`: pass
  (`permission_mysql.rs` integration tests all green).
- `./scripts/cargo-audit.sh`: completed and reported advisories above.

## Recommended follow-up changes

1. In `evaluate_permission`, enforce role tenant equality before checking
   role permissions:
   - Load role revision tenant entity slot.
   - Require `role.tenant == requested_tenant`.
   - Add regression tests for mismatched `RoleMembership.tenant` vs
     `RoleDefinition.tenant`.
2. Add strict permission atom validation:
   - Validate parsed permission docs against `ALL_ATOMS`.
   - Decide reject-on-unknown behavior at parse or at role create/update.
   - Add tests covering unknown atom rejection behavior.
3. Track workspace-level advisory policy for the `rsa` finding from
   `cargo-audit` and document accepted risk or mitigation path.

