# What next, after Phase 1 shipped

**Date:** 2026-04-21
**Context:** Phase 1 (mechanics-config extraction) closed today
with `mechanics-config 0.1.0`, `mechanics-core 0.3.0`, and
`mechanics 0.3.0` all on crates.io with signed release tags.
See `ROADMAP.md` §"Current state" + §Phase 1 for the full
wrap-up. This note records the next-step orientation so the
following session doesn't have to re-derive it.

## TL;DR

**Next up: Phase 2 — `philharmonic-policy`.** Before any code is
written, Yuka's **design-stage review** is required for the two
crypto-sensitive sub-items (SCK encrypt/decrypt and `pht_` API
token generation). The recommended ordering is to knock out the
non-crypto work first — entity kinds, permission evaluation,
substrate integration — so Claude has useful work to carry
forward while the crypto design review is in flight.

## Phase 2 scope (abridged — authoritative version in `ROADMAP.md`)

Crate: `philharmonic-policy`.

Five work-items:

1. **Seven entity kinds** — `Tenant`, `Principal` (with unused
   `epoch` reserved), `TenantEndpointConfig` (minimal:
   `display_name`, `encrypted_config`, `key_version`,
   `is_retired`, `tenant`), `RoleDefinition`, `RoleMembership`,
   `MintingAuthority` (with `epoch`), `AuditEvent`. Each gets a
   stable `KIND: Uuid` constant — generate once, commit, treat
   as wire-format.
2. **SCK-based encryption** for `TenantEndpointConfig`
   (AES-256-GCM on submit, decrypt on read). **Crypto-review
   gate applies.**
3. **`pht_` API token format** — 32 random bytes → 43-char
   base64url → `pht_` prefix, with SHA-256 for storage.
   **Crypto-review gate applies.**
4. **Permission evaluation** —
   `evaluate_permission(principal, tenant, required_atom)` walks
   `RoleMembership` → `RoleDefinition` → permission array.
5. **Unit tests** covering entity CRUD (testcontainers against
   sqlx-mysql backend), SCK round-trip with known vectors,
   pht_ generate/parse round-trip, and permission evaluation
   across nested memberships.

Acceptance criteria in full: `ROADMAP.md` §Phase 2.

## Crypto-review gates — non-waivable, design-first

Two sub-items trigger `.claude/skills/crypto-review-protocol`:

| Sub-item | Design gate | Code gate |
|---|---|---|
| SCK AES-256-GCM over `TenantEndpointConfig.encrypted_config` | Required | Required |
| `pht_` API token generation + SHA-256 storage hash | Required | Required |

Design-gate form: before any crypto code is written, Yuka
approves the approach (construction choice, key-derivation
story, IV/nonce strategy, test-vector set). Code-gate form:
before `philharmonic-policy 0.1.0` publishes, Yuka reviews the
implementation.

**Claude does not skip either gate, even with explicit
authorization at the code-gate stage only.** The rule is "both
gates, always." See
`.claude/skills/crypto-review-protocol/SKILL.md` +
`ROADMAP.md` §5 "Crypto review protocol" for the full text.

## Suggested intra-Phase-2 ordering

Three waves, chosen so Claude can make progress without blocking
on crypto review:

1. **Wave 1 — non-crypto foundation (Claude can drive solo):**
   - Six of the seven entity kinds (everything except
     `TenantEndpointConfig`, which has crypto-touching fields).
     Their `KIND: Uuid` constants.
   - Substrate wire-up via `philharmonic-store` traits
     (no implementation, just the kind registration).
   - Permission evaluation skeleton + unit tests.
   - Test harness plumbing for testcontainers integration
     against the sqlx-mysql backend.

2. **Wave 2 — gated crypto (Yuka design-review then code):**
   - SCK approach design doc — construction choice,
     key-storage path, nonce discipline, test-vector plan.
     Yuka approves before any crypto code lands.
   - `TenantEndpointConfig` entity kind with AES-256-GCM
     encrypt/decrypt.
   - `pht_` token approach design — random source, encoding,
     storage-hash discipline. Yuka approves before any crypto
     code lands.
   - `pht_` generate + parse implementation.
   - Yuka code-reviews both before Phase 2 closes.

3. **Wave 3 — publish readiness:**
   - Run `./scripts/check-api-breakage.sh philharmonic-policy`
     (no crates.io baseline yet — this is a fresh publish, so
     the tool won't have a baseline to compare against; it'll
     short-circuit as "no prior version").
   - `./scripts/publish-crate.sh --dry-run philharmonic-policy`
     → real publish → `./scripts/push-all.sh`.
   - Mark Phase 2 done in `ROADMAP.md` in the same commit as
     whatever final change lands (per CLAUDE.md "ROADMAP is
     living").

This split is a suggestion, not an assignment. If Yuka prefers
front-loading the crypto design discussion, invert — but don't
write crypto code before design approval either way.

## Housekeeping that could go in parallel

None of these block Phase 2, but if there's an idle moment:

- **`rsa 0.9.10` Marvin Attack (RUSTSEC-2023-0071)** still shows
  in `cargo-audit`. Transitive through
  `sqlx-mysql → philharmonic-store-sqlx-mysql`; no upstream fix
  available. Track upstream (`rustcrypto/RSA`) rather than act.
  Not a library-publish concern (we don't ship Cargo.lock).
- **`time 0.3.45` stack-exhaustion (RUSTSEC-2026-0009)** was
  flagged during the pre-release audit; the advisory has
  resolved on its own since then (no longer appears in
  `cargo-audit` output as of 2026-04-21 late session). No
  action needed.
- **`paste 1.0.15` unmaintained warning (RUSTSEC-2024-0436)** —
  transitive through `boa_engine`. Warning, not a vulnerability.
  Upstream boa-rs's problem; track rather than act.
- **`[profile.release]` warnings** from workspace crates
  declaring their own release profiles — cargo ignores them in
  workspace members. Cleanup opportunity (delete those blocks
  from non-root Cargo.tomls). Not a publish concern.

## Authoritative references

- `ROADMAP.md` §Phase 2 — the full scope + acceptance criteria.
- `docs/design/09-policy-and-tenancy.md` — entity kinds,
  SCK and `pht_` specs, permission evaluation semantics.
- `docs/design/11-security-and-cryptography.md` — threat model,
  construction specifics, and the test-vector discipline that
  crypto-gate submissions must satisfy.
- `.claude/skills/crypto-review-protocol/SKILL.md` — the
  two-gate rule and when it triggers.
- `ROADMAP.md` §5 "Crypto review protocol" — the
  authoritative version of the same rule.
