# Remaining TODOs — 2026-05-01

**Date**: 2026-05-01 (end of Phase 9 session)
**Author**: Claude Code

Zero TODOs remain in Rust source code. All deployment blockers
(key generators, example configs) are resolved. This note
consolidates what's left.

---

## Unpublished local changes

1. **`philharmonic` meta-crate**: local is 0.1.1 on crates.io
   but has 2 new commits (keygen module + README update). Needs
   a 0.1.2 publish when ready.

---

## Documentation polish (can be done anytime)

2. **doc 08 (connector architecture)**: Should note the Phase 9
   deployment shape where the router is embedded in the API bin.

3. **doc 15 (v1 scope)**: Could add a completion note.

4. **Workspace README scripts section**: Missing entries for
   `build-status.sh`, `release-build.sh`, `audit-log.sh`.

5. **CHANGELOG entries**: Several crates bumped (store 0.1.1,
   mechanics 0.4.0, connector-service 0.2.0, api 0.1.2,
   philharmonic 0.1.1) but some may not have `[Unreleased]` →
   version entries in their CHANGELOGs.

---

## Missing tests (no deployment blocker)

6. **`install` subcommand**: Unit test of `InstallPlan` +
   systemd unit template generation (without writing files).

7. **`security_headers` middleware**: Assert headers present in
   axum test response.

8. **`webui` module**: Assert `webui_fallback` returns
   `index.html` for unknown paths, correct MIME for known.

9. **`ConnectorConfigLowerer::lower()`**: Unit test with known
   inputs → known output.

10. **`HttpStepExecutor::execute()`**: `wiremock`-backed test
    verifying correct headers.

11. **SIGHUP reload loop**: Test that sending SIGHUP reloads
    config in the bin.

12. **TOML drop-in merge with >2 files**: Verify lexicographic
    ordering.

13. **`SinglePool::connect()` error path**: Bad URL → meaningful
    `StoreError`.

14. **WebUI TypeScript tests** (Jest/RTL): Lowest priority.

15. **Docker compose smoke test in CI**: `docker compose config`.

16. **musl release build in CI**: `release-build.sh` CI step.

---

## Code hygiene

17. **Visibility + docs audit for bin targets**: `pub` items in
    `bins/*/src/` should be `pub(crate)` (nothing outside the bin
    imports them) or, where genuinely public, have doc comments.
    Affects `mechanics-worker`, `philharmonic-api-server`,
    `philharmonic-connector`. Crate-by-crate: grep for bare `pub`
    in each bin's `src/`, downgrade to `pub(crate)`, add docs to
    any that stay `pub`. Then enable `#![warn(missing_docs)]` in
    lib crates one at a time (gated by `RUSTDOCFLAGS="-D
    missing_docs" cargo doc --no-deps -p <crate>` in
    `rust-lint.sh`).

---

## Post-Golden-Week tasks (on or after 2026-05-07)

18. **Phase 7 Tier 2 — SMTP** (`email-smtp`): Single connector
    impl, `lettre`-based. Placeholder 0.0.0 is published.
    Discrete scope.

19. **Phase 7 Tier 3 — Anthropic** (`llm-anthropic`): Native
    Anthropic Messages API. Placeholder 0.0.0 published.
    Scheduled on or after 2026-05-07.

20. **Phase 7 Tier 3 — Gemini** (`llm-gemini`): Native Google
    Gemini API. Placeholder 0.0.0 published. Scheduled on or
    after 2026-05-07.

---

## Post-v1 (deferred by design, per doc 12)

Not scheduled; documented in `docs/design/12-deferred-decisions.md`:

- Hierarchical tenancy
- Streaming step execution
- Multi-region / geo-distributed deployment
- Workflow versioning / migration
- Plugin system for custom implementations
- Observability stack (metrics, distributed tracing)
- Admin UI beyond the current test/demo WebUI

---

## Summary

**Deployment-ready now.** All bins compile, key generators
work, configs documented, release builds produce correct
sizes (13 MB / 11 MB / 2.2 GB). The items above are polish
(tests, docs) or scheduled future work (Tiers 2–3). Nothing
blocks Yuka's reference deployment.
