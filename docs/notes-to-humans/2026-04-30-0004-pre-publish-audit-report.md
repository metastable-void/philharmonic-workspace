# Pre-publish audit report

**Date**: 2026-04-30
**Author**: Claude Code (5 parallel subagent audits)
**Scope**: All library crates, bin targets, docs, Cargo.toml
files, security/crypto paths

---

## CRITICAL

| # | Finding | Location | Action |
|---|---|---|---|
| C1 | `.env` contains a real OpenAI API key in git history | `.env` (gitignored but committed once) | **Rotate the key immediately.** History can't be rewritten (append-only rule). `.gitignore` prevents further commits. |

## HIGH

| # | Finding | Location | Action |
|---|---|---|---|
| H1 | `philharmonic-workflow` depends on `philharmonic-policy = "0.1"` but policy is at 0.2.0 | `philharmonic-workflow/Cargo.toml` | Bump to `"0.2"`. Publishing will fail without this. |
| H2 | `philharmonic-connector-router` missing publishing metadata (`description`, `license`, `repository`, `readme`) | `philharmonic-connector-router/Cargo.toml` | Add fields. Publishing blocker. |
| H3 | Docker deploy configs have hardcoded passwords (`philharmonic`) | `docker-compose.yml`, `deploy/api.toml` | Acceptable for dev/example configs — add comments warning these are examples. |
| H4 | Design doc 10 (API layer) still says "Not yet implemented" | `docs/design/10-api-layer.md` ~L538 | Update to reflect Phase 8 completion (published 0.1.0, 2026-04-28). |

## MEDIUM

| # | Finding | Location | Action |
|---|---|---|---|
| M1 | Version constraint inconsistencies: some crates use `"0.3.5"` (strict), others `"0.3"` (loose) for `philharmonic-types` | Multiple Cargo.toml files | Standardize — either all strict or all loose. Not a blocker but drift risk. |
| M2 | X25519 `StaticSecret` in `RealmPrivateKeyEntry` not wrapped in `Zeroizing<>` | `philharmonic-connector-service/src/realm_keys.rs:14` | Wrap in `Zeroizing<StaticSecret>`. `StaticSecret` implements `Zeroize` but not `ZeroizeOnDrop`. |
| M3 | No security headers middleware (CORS, CSP, X-Content-Type-Options, HSTS) | `philharmonic-api/src/` | Add a tower layer, or document that it's handled at reverse proxy. |
| M4 | `mechanics/src/lib.rs` has 3 `eprintln!()` calls for connection errors | `mechanics/src/lib.rs:254,295,313` | Replace with proper error return or logging trait. Library crates should not print. |
| M5 | Missing `#[must_use]` on crypto Result-returning functions (`encrypt_payload`, etc.) | `philharmonic-connector-client/src/encrypt.rs` | Add `#[must_use]`. Prevents silently ignoring encryption failures. |
| M6 | MSRV inconsistency: `inline-blob` and `philharmonic-connector-impl-embed` use 1.89, all others 1.88 | Two Cargo.toml files | Document or align. |
| M7 | Gap note says e2e tests "still open" but ROADMAP marks task 6 done | `docs/notes-to-humans/2026-04-30-0003` | Resolve — both `e2e_mysql.rs` and `e2e_full_pipeline.rs` exist. |

## LOW / INFO

| # | Finding | Location | Action |
|---|---|---|---|
| L1 | README missing `inline-blob` from crate list | `README.md` crate inventory | Add entry. |
| L2 | README missing `musl-build.sh` and `new-submodule.sh` from script list | `README.md` script section | Add entries. |
| L3 | 2 pub fns missing doc comments: `RevisionRef::new`, `SinglePool::new` | `philharmonic-store`, `philharmonic-store-sqlx-mysql` | Add `///` doc comments. |
| L4 | 1 TODO comment: "TODO(sub-phase D)" | `philharmonic-api/src/auth.rs` | Verify if resolved; remove or track. |
| L5 | CONTRIBUTING.md §10.9 doesn't explicitly state "no system OpenSSL" | `CONTRIBUTING.md` | Verify the addition from this session landed. |
| L6 | Design doc 08 doesn't mention router-in-API-binary topology | `docs/design/08-connector-architecture.md` | Add note about Phase 9 embedded-router shape. |

## PASSED (no issues)

- **14-step token validation**: All steps implemented, correctly ordered.
- **SQL injection**: All queries parameterized via `sqlx::query().bind()`.
- **Unsafe code**: Zero `unsafe` blocks in any library crate.
- **Error info leakage**: Auth/authz return generic 401/403 only.
- **Box\<dyn Error\>/anyhow**: Not used in any library crate.
- **Panics in library code**: All `.unwrap()`/`.expect()` in test modules only.
- **Feature flag consistency**: Meta-crate features match impl crate names.
- **Dev-dep leakage**: None found.
- **License consistency**: All crates `Apache-2.0 OR MPL-2.0`.
- **Edition consistency**: All crates edition 2024.
- **Patch section**: All submodule crates in `[patch.crates-io]`.

## Publishing blockers (must fix before `cargo publish`)

1. **H1**: `philharmonic-workflow` dep on `philharmonic-policy = "0.1"`
   → bump to `"0.2"`.
2. **H2**: `philharmonic-connector-router` missing Cargo.toml metadata.
3. Three unshipped connector impls (`llm-anthropic`, `llm-gemini`,
   `email-smtp`) need 0.0.0 placeholder publishes before the
   meta-crate can publish (optional deps must resolve on crates.io).

## Recommended before publish but not blocking

- **C1**: Rotate OpenAI key (security, not a publish gate).
- **M1**: Standardize version constraints.
- **L1–L2**: README completeness.
- **H4**: Update design doc 10.
