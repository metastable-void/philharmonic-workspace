# Remaining TODOs — 2026-05-01

**Date**: 2026-05-01 (end of Phase 9 session)
**Author**: Claude Code

Zero TODOs remain in Rust source code. All deployment blockers
(key generators, example configs) are resolved. This note
consolidates what's left.

---

## Unpublished local changes

1. **All 17 publishable crates need a patch release.** Changes
   since last publish: doc comments added (missing_docs gate),
   `pub(crate)` visibility in bins, crypto-vector paths moved
   into crate-local `tests/vectors/`, `philharmonic` meta-crate
   gained `mechanics` feature-gate + `bootstrap` CLI + `whoami`
   endpoint. Bump all versions by patch (`.+1`), then publish
   in dependency order. The 3 placeholder crates (email-smtp,
   llm-anthropic, llm-gemini) stay at `0.0.0` — their only
   change is a one-line crate-level doc comment.

---

## Documentation polish (can be done anytime)

2. ~~**doc 08 (connector architecture)**~~: **Done** (2026-05-01).
   Corrected connector-client/service descriptions.

3. ~~**doc 15 (v1 scope)**~~: **Done** (2026-05-01). "Needs
   implementation work" → "Implemented and published".

4. ~~**Workspace README scripts section**~~: **Done**
   (2026-05-01). All scripts and xtask bins listed.

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

17. ~~**Visibility + docs audit for bin targets**~~: **Done**
    (2026-05-01). All `pub` → `pub(crate)` in bin targets, doc
    comments added across all crates, `RUSTDOCFLAGS="-D
    missing_docs"` gated in `rust-lint.sh`.

18. **Feature-gate mechanics/boa in meta-crate**: `boa_engine`
    (the JS runtime from `mechanics-core`) leaks into all three
    bins via the unconditional `mechanics`/`mechanics_core`
    re-exports in the `philharmonic` meta-crate. Only
    `mechanics-worker` needs it. Add a `mechanics` Cargo feature
    to the meta-crate (default off), enable it only in
    `mechanics-worker`. Same pattern as the embed-weight split.

19. **Restructure `docs/` for mdBook**: Per HUMANS.md — move
    `ROADMAP.md` and `POSIX_CHECKLIST.md` under `docs/`, split
    crypto proposals/vectors out of `docs/design/` into
    `docs/crypto/`, demolish `docs/instructions/README.md`
    (absorbed into CONTRIBUTING.md + agent MDs), make `docs/`
    the home for mdBook-based GitHub Pages docs. Deployed at
    `https://metastable-void.github.io/philharmonic-workspace/`
    — set `[output.html] site-url = "/philharmonic-workspace/"`
    in `book.toml`. Add `cargo install mdbook` to `setup.sh`.
    Update all internal references to new paths.

20. **WebUI branding via API config**: Allow operators to replace
    the "Philharmonic" display text in the WebUI with a
    config-supplied string (e.g. `webui_brand_name = "Acme
    Workflows"`). The `[P]` monogram icon should derive from the
    first character of the brand name (e.g. `[A]` for "Acme").
    Injected at serve time via a template variable or a
    `/v1/_meta/branding` endpoint that the React app fetches on
    load. Custom icon/logo upload is out of scope for now.

---

## Post-Golden-Week tasks (on or after 2026-05-07)

21. **Phase 7 Tier 2 — SMTP** (`email-smtp`): Single connector
    impl, `lettre`-based. Placeholder 0.0.0 is published.
    Discrete scope.

22. **Phase 7 Tier 3 — Anthropic** (`llm-anthropic`): Native
    Anthropic Messages API. Placeholder 0.0.0 published.
    Scheduled on or after 2026-05-07.

23. **Phase 7 Tier 3 — Gemini** (`llm-gemini`): Native Google
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
