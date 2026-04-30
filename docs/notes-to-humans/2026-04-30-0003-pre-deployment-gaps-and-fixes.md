# Pre-deployment gap analysis and fixes

**Date**: 2026-04-30
**Author**: Claude Code

## Context

With all three bin targets landed (mechanics-worker,
philharmonic-connector, philharmonic-api), we investigated
what's still missing for real e2e tests and real deployments.

## Gaps found

### Fixed in this session

1. **Schema migration not called at API startup.**
   The `philharmonic-api` bin connected to MySQL but never
   ran `migrate()`, so starting against a fresh database
   would fail on missing tables. Fixed: added
   `migrate(pool.pool()).await` to `serve()` immediately
   after pool creation, before building the API router.
   Migration is idempotent (CREATE TABLE IF NOT EXISTS +
   duplicate-index suppression), so it's safe to run on
   every startup.

### Still open

2. ~~**No WebUI source tree.**~~ **Resolved 2026-04-30.**
   WebUI landed: React 19 + Redux Toolkit + TypeScript SPA
   (8 pages), built via `webui-build.sh`, embedded in the
   API binary via `rust-embed` + SPA fallback routing.

3. ~~**No e2e testcontainers for the full API stack.**~~
   **Resolved 2026-04-30.** Two e2e test suites landed:
   `e2e_mysql.rs` (7 tests: CRUD + auth against real MySQL)
   and `e2e_full_pipeline.rs` (full crypto round-trip: API
   lowerer → in-process connector service → VectorSearch
   impl). Both use testcontainers MySQL + `serial_test`.

4. **No Docker images or docker-compose.** No Dockerfile
   for any of the three bins. No orchestration for
   multi-container scenarios. ROADMAP Phase 9 task 10
   (optional, post-5/2).

5. **`migrate` takes `&MySqlPool` directly.** The bin
   accesses it via `SinglePool::pool()` (added yesterday).
   Works, but means the migration path is coupled to the
   MySQL backend. If a second backend is ever added, the
   migration path would need to be abstracted. Not a
   problem today — MySQL is the only backend.

## Deployment readiness assessment

With the schema-migration fix, the `philharmonic-api` bin
can start against a fresh MySQL instance and serve API
requests. A minimal deployment needs:

- MySQL 8 (or MariaDB 10.5+, or TiDB) accessible at the
  configured URL
- An Ed25519 signing key (32-byte seed file)
- At least one verifying key entry matching the signing key
- Optionally: SCK file for endpoint-config encryption
- Optionally: TLS cert + key (with `--features https`)
- Optionally: connector dispatch table pointing at
  philharmonic-connector instances

The `mechanics-worker` and `philharmonic-connector` bins are
deployment-ready as-is (they don't need a database).
