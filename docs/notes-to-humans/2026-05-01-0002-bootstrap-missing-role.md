# Bootstrap missing role + membership — 403 on all tenant routes

**Date**: 2026-05-01
**Author**: Claude Code
**Severity**: Deployment blocker (first deploy)

## Bug

The `philharmonic-api bootstrap` subcommand created a Tenant and
a Principal but no RoleDefinition or RoleMembership. The
permission evaluation system (`evaluate_permission` in
`philharmonic-policy/src/evaluation.rs`) resolves permissions
exclusively through role memberships — a bare principal with no
role memberships has zero permissions. Every tenant-scoped route
is protected by a `RequiredPermission` atom, so the authz
middleware returned 403 for all requests.

## Root cause

The bootstrap implementation (commit `9768985`) followed the
`create_principal` pattern from `philharmonic-api/src/routes/
principals.rs`, which only creates the Principal entity. In normal
operation, an already-authenticated admin would then create a role
and membership via the API. But at bootstrap time there is no
authenticated admin — the bootstrap principal *is* the first
admin and needs permissions from the start.

## Fix

The bootstrap function now also creates:

1. A **RoleDefinition** ("Bootstrap Admin") containing all 22
   permission atoms from `ALL_ATOMS`.
2. A **RoleMembership** linking the bootstrap principal to the
   admin role within the bootstrap tenant.

This gives the bootstrap principal full permissions on its tenant,
matching what an operator would expect from a first-run setup.

## Impact

Any deployment that ran `bootstrap` before this fix has a
principal with no permissions. To recover:

- Drop the database and re-run `bootstrap`, or
- Manually insert the role + membership entities via SQL (not
  recommended — use the bootstrap flow).

## Lesson

Bootstrap/seed commands must create the full entity graph needed
for the seeded principal to actually function, not just the
minimum entities. Test bootstrap by exercising a protected route
immediately after seeding.
