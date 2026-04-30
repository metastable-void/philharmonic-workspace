# Remaining gaps before v1

**Date**: 2026-04-30
**Author**: Claude Code

## Context

Phase 9 integration is complete: all library crates published,
three bin targets split into separate crates (embed weight
isolation verified), WebUI embedded, e2e tests passing, Docker
compose ready, `install` subcommand working. This note
catalogues what remains before declaring v1.

---

## Documentation gaps

### Design docs needing reconciliation

1. **doc 08 (connector architecture)**: Should note the Phase 9
   deployment shape where the connector router is embedded in
   the API binary, not deployed as a separate process. The
   framework supports both — doc 08 should say so explicitly.

2. **doc 15 (v1 scope)**: Could add a "Phase 9 completion" note
   confirming all v1-scoped items shipped. Currently reads as a
   forward-looking scope doc.

### README gaps

3. **Workspace README `scripts/` section**: Missing entries for
   `build-status.sh`, `release-build.sh` (if not already added).
   Also missing `audit-log.sh` entry.

4. **Meta-crate README**: Just updated, but should mention the
   bin split (bins are now in `bins/` at the workspace root, not
   in the meta-crate).

### CHANGELOG gaps

5. Several crates were bumped (store 0.1.1, mechanics 0.4.0,
   connector-service 0.2.0, etc.) but their CHANGELOGs may not
   have `[Unreleased]` → version entries for the new releases.
   Check each before re-publishing.

---

## Missing tests

### High priority

6. **`install` subcommand**: No test coverage. The subcommand
   requires root, so a unit test of the `InstallPlan` struct
   and systemd unit template generation (without actually
   writing files) would be valuable.

7. **`security_headers` middleware**: No test verifying the
   headers are actually present in responses. A simple axum
   test-request → assert headers.

8. **`webui` module**: No test that `webui_fallback` returns
   `index.html` for unknown paths and the correct MIME type
   for known extensions.

9. **`ConnectorConfigLowerer::lower()`**: No unit test. The
   full-pipeline e2e test covers the round-trip, but a unit
   test with known inputs → known output would catch regressions
   faster.

10. **`HttpStepExecutor::execute()`**: No unit test. A
    `wiremock`-backed test that verifies the correct headers
    (`Authorization`, `X-Encrypted-Payload`) are sent would be
    valuable.

### Medium priority

11. **SIGHUP reload in bins**: No test that sending SIGHUP
    actually reloads config. The `ReloadHandle` has a unit test,
    but the bin-level reload loop (re-read TOML, rebuild
    registries, hot-swap router) is untested.

12. **TOML config drop-in overlay ordering**: The `load_config`
    function has unit tests, but no test with >2 drop-in files
    verifying lexicographic merge order.

13. **`SinglePool::connect()` error path**: No test that a bad
    URL returns a meaningful `StoreError`.

### Low priority (nice-to-have)

14. **WebUI TypeScript tests**: No Jest/RTL tests for the React
    components. The WebUI is a test/demo artifact, so this is
    lowest priority.

15. **Docker compose smoke test**: No CI step that runs
    `docker compose config` to validate YAML. Cheap to add.

16. **musl release build CI**: No CI step that runs
    `release-build.sh`. The debug musl build is verified
    manually; a CI check would catch toolchain drift.

---

## Deployment blockers (for Yuka's reference deployment)

17. ~~**Ed25519 keypair generation**~~ **Resolved 2026-04-30.**
    All three bins: `gen-signing-key -o <dir>`.

18. ~~**Realm KEM keypair generation**~~ **Resolved 2026-04-30.**
    All three bins: `gen-realm-key -r <realm> -o <dir>`.

19. ~~**SCK generation**~~ **Resolved 2026-04-30.**
    All three bins: `gen-sck -o <dir>`.

20. ~~**Example TOML configs**~~ **Resolved 2026-04-30.** The
    `deploy/*.toml` files now have all fields documented. Was:
    minimal. A more complete example showing signing key paths,
    verifying key entries, realm public keys, and SCK config
    would help first-time deployment.

---

## Phase 7 remaining work (post-v1 scope per ROADMAP)

21. **Tier 2 — SMTP** (`email-smtp`): Single connector, discrete
    scope, `lettre`-based. The 0.0.0 placeholder is published.

22. **Tier 3 — Anthropic + Gemini** (`llm-anthropic`,
    `llm-gemini`): Deferred until on or after 2026-05-07
    (post-Golden-Week). The 0.0.0 placeholders are published.

---

## Summary

The system is functionally complete for v1. The gaps above are
polish items (tests, docs, tooling) that improve the developer
and operator experience but don't block the reference deployment.
Items 17–20 (key generation tooling) are the most practically
useful for Yuka's immediate deployment.
