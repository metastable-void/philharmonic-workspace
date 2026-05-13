# ROADMAP §3.J trim — 2026-05-13 (D23 + D25 done)

Verbatim preservation of the §3.J entries for **D25**
(`mechanics-http-client` hickory-resolver CVE bump) and
**D23** (in-tree minimal testcontainers replacement +
warm-container pivot + dockerlet atexit cleanup) before
they were trimmed out of the live `docs/ROADMAP.md` on
2026-05-13 when both landed.

Live commits / artefacts:

- **D25** — landed via Codex round 01 + Claude post-Codex
  polish on 2026-05-13. `mechanics-http-client` shipped
  `0.2.1` to crates.io. Codex prompt archive:
  [`docs/codex-prompts/2026-05-13-0003-d25-mhc-hickory-resolver-cve-fix-01.md`](../codex-prompts/2026-05-13-0003-d25-mhc-hickory-resolver-cve-fix-01.md).
- **D23** — landed via Codex round 01 + Claude post-Codex
  warm-container pivot + dockerlet atexit cleanup on
  2026-05-13. `dockerlet 0.1.0` shipped to crates.io.
  Six consumer crates patch-bumped (philharmonic-api
  0.1.9, philharmonic-policy 0.2.4, philharmonic-workflow
  0.1.5, philharmonic-store-sqlx-mysql 0.1.4,
  philharmonic-connector-impl-sql-mysql 0.1.1,
  philharmonic-connector-impl-sql-postgres 0.1.1). Codex
  prompt archive:
  [`docs/codex-prompts/2026-05-13-0004-d23-dockerlet-testcontainers-replacement-01.md`](../codex-prompts/2026-05-13-0004-d23-dockerlet-testcontainers-replacement-01.md).
- **deny.toml** after the pass — `rustls-native-certs`,
  `rustls-platform-verifier`, and `native-tls` all
  no-wrapper full bans; only `ring` retains a wrapper for
  the upstream `h3-quinn 0.0.10` default-features feature-
  unification bug (allowed via `wrappers = ["quinn-proto"]`).

---

## §3.J pre-trim content (verbatim)

### J. Production-security dep cleanup (3 dispatches) — TOP PRIORITY

**Sequencing directive (Yuka, 2026-05-13):** these
production-security cleanups land **before any remaining
Tier 2/3 connector work (D7 SMTP, D8 Anthropic, D9 Gemini,
D19 DNS) and before D18 (mechanics-core module refactor)**.
The framing is "serious production-ready security" — the
workspace's release-binary runtime trees being clean of
non-aws-lc-rs / non-webpki-roots TLS and the per-dep
feature surface being minimised are baseline requirements,
not optional polish.

In-section ordering: **D25 → D23 → D24** (active CVE bump
first, then testcontainers replacement to drop the bollard
`rustls-native-certs` wrapper, then the broad
default-features audit that benefits from D23 + D25 already
being landed).

**Retro / sequencing lesson** (Yuka, 2026-05-13): when
several §J-class cleanups queue together, weight
**test-speed-impact** alongside CVE urgency when ordering.
Dispatches that speed up `pre-landing.sh` (e.g. D23's
testcontainer-concurrency knob trimming the per-run cost
of the SQL-connector `--ignored` phase) compound across
**every** subsequent dispatch's verification pass, while a
CVE patch bump is a one-shot win. If a near-term cleanup
will materially shorten the validation loop for the
remaining queued cleanups, landing it first usually pays
back the delay on the urgent-but-isolated item. D25
landed first this round because the CVE was active and
the queue was already drafted; if a similar situation
recurs, surface the speed-vs-urgency tradeoff to the
human-developer explicitly at dispatch-archival time
instead of defaulting to CVE-first.

#### D25 — `mechanics-http-client` hickory-resolver CVE bump

Captured 2026-05-13 as an immediate follow-up to the D22
client push. GitHub Dependabot surfaced two `hickory-proto`
advisories on the parent repo within hours of the D22
client landing:

- **HIGH** — `RUSTSEC-2026-0118` /
  `GHSA-3v94-mw7p-v465` — NSEC3 closest-encloser
  proof unbounded loop. Only triggerable with
  `dnssec-ring` / `dnssec-aws-lc-rs` features on;
  **not applicable to mhc's config** (mhc uses
  `default-features = false, features = ["system-config",
  "tokio"]`, no dnssec).
- **MEDIUM** — `RUSTSEC-2026-0119` /
  `GHSA-q2qq-hmj6-3wpp` — O(n²) name compression in
  `BinEncoder` during DNS message encoding. **Triggerable
  in mhc's config** — any attacker-influenced
  authoritative server response during HTTPS RR
  resolution can amplify CPU exhaustion.

Bump `hickory-resolver` 0.25.2 → 0.26.1 in mhc; pick up
the upstream rename of `hickory-proto` → `hickory-net`
(0.26.0 release) and any API drift it forced. mhc ships
as `0.2.1` patch release (public API unchanged; the
hickory types are internal to `src/https_rr.rs` and
`src/tests.rs`).

Claude drafts the Codex prompt at
[`docs/codex-prompts/2026-05-13-0003-d25-mhc-hickory-resolver-cve-fix-01.md`](../codex-prompts/2026-05-13-0003-d25-mhc-hickory-resolver-cve-fix-01.md);
Codex implements + tests. **No crypto-review gate** —
transport-layer dep bump only. Patch-bump mhc to 0.2.1
(0.3.0 only if upstream API drift forced a public-
surface change, which is not expected).

Lands **before D23 and D24** per the §J in-section
ordering; lands **before any other workspace work** per
the §J top-priority directive.

#### D23 — in-tree minimal testcontainers replacement

Captured 2026-05-13 as the cleanup-residual of the
`ring` / `rustls-platform-verifier` / `rustls-native-certs`
bans-tightening pass. After that pass (philharmonic-api
reqwest → mhc dev-dep migration, rcgen aws_lc_rs feature,
ureq `rustls-no-provider` + manual aws-lc-rs provider
install, testcontainers/bollard switched to `aws-lc-rs`
feature), exactly one wrapper remains in `deny.toml`:
`rustls-native-certs` allowed when its direct parent is
`bollard`. The path is `bollard`'s `ssl_providerless`
feature (which both `aws-lc-rs` and `ssl` build on)
unconditionally pulling `rustls-native-certs` — bollard
uses it to validate the Docker daemon's registry-side TLS
when pulling images.

Replace `testcontainers` / `testcontainers-modules`
  with a minimal in-tree dev-tooling crate
  (working name `xtask-testcontainers` or
  `mechanics-testcontainers`; bikeshed at prompt-drafting
  time) that uses `bollard` with **only the features the
  workspace's integration tests actually need**, dropping
  `ssl_previderless` / `home` / `rustls-native-certs`
  entirely.

  **What workspace tests use today** (audit before drafting
  the prompt; this is the baseline):
  - Start a MySQL container (`mysql:8` or similar public
    image) with a healthcheck-style wait.
  - Start a Postgres container with the same pattern.
  - Inspect host + assigned port for the connection string.
  - Tear down on `Drop`.
  - All over a local Docker daemon Unix socket (the
    workspace doesn't currently use remote-daemon test
    setups).

  **What bollard needs for that subset:**
  - `unix-socket` feature (talk to local Docker daemon).
  - Image-pull / container-start / container-inspect /
    container-stop API calls.
  - **No TLS**: registry-side TLS is the daemon's concern,
    not bollard's, when the workspace talks to the daemon
    over a Unix socket.
  - **No `home`**: workspace tests use public images that
    don't need Docker Hub auth from `~/.docker/config.json`.

  **Hard requirements:**
  - Public surface mirrors what the workspace's test code
    actually uses today (a `Container<Image>` handle with
    `.start().await`, `.get_host_port_ipv4(...)`,
    `.get_host()`, `.with_startup_timeout(...)`, Drop-based
    teardown). Migration sites are mostly mechanical.
  - The MySQL and Postgres "image" types from
    `testcontainers-modules` get re-implemented as
    minimal wrappers (image name + env vars + ready
    probe).
  - Dev-tooling crate, never published. `publish = false`.
  - In-tree (non-submodule) member, mirroring the existing
    `xtask/` placement convention.
  - No `unsafe`, no panics on reachable paths in lib code
    (test fixtures themselves may `.expect()` per the
    workspace's test conventions).

  **Migration scope** (~6 consumer crates):
  - `philharmonic-api/tests/{e2e_mysql.rs, e2e_full_pipeline.rs}`
  - `philharmonic-policy/tests/...`
  - `philharmonic-workflow/tests/...`
  - `philharmonic-store-sqlx-mysql/tests/...`
  - `philharmonic-connector-impl-sql-mysql/tests/...`
  - `philharmonic-connector-impl-sql-postgres/tests/...`

  Each migrates from `testcontainers` / `testcontainers-modules`
  imports to the in-tree replacement. Existing test logic
  unchanged.

  **Concurrency-limit knob** (Yuka, 2026-05-13): today the
  workspace's testcontainer tests are file-lock-serialized
  via `serial_test` (see comments in
  `philharmonic-connector-impl-sql-{mysql,postgres}/Cargo.toml`)
  because spinning up many containers concurrently used to
  OOM the dev box. The current host is more capable —
  the new default is **`min(4, available_parallelism / 4)`**
  concurrent testcontainer tests (computed via
  `std::thread::available_parallelism()`). On a 16-core
  box that's 4; on an 8-core box, 2; on a 4-core box, 1;
  on resource-tight CI runners, the floor of 1 falls back
  to the current serial behaviour automatically. Implement
  via either a semaphore-style file-based limiter (so the
  limit applies across cargo's per-test-binary process
  fan-out) or a concurrency knob on the replacement
  crate's fixture initialiser. Wire the formula in one
  place in the replacement crate (a `pub const` or a
  `LazyLock<usize>`) so future tuning is one-edit. Verify
  by running the SQL connector test suites under a load
  monitor (`./scripts/build-status.sh` or the
  resource-pressure xtask) at the computed concurrency
  and confirming the dev box stays healthy; the formula
  errs on the conservative side.

  **Acceptance:** after D23, the `rustls-native-certs`
  entry in `deny.toml` becomes a no-wrapper full ban
  (matching `native-tls`, `rustls-platform-verifier`).
  `cargo tree --workspace --invert rustls-native-certs`
  prints nothing.

  Claude drafts the Codex prompt; Codex implements + tests
  + migrates consumers. No crypto-review gate — dev-tooling
  only, no AAD / AEAD / SCK / COSE touches.

  **Why not fork bollard's features instead?** Bollard's
  Cargo.toml entangles `home` / `ssl_providerless` /
  `rustls-native-certs` such that disabling them from a
  consumer's feature flags isn't possible without a fork.
  The in-tree wrapper approach is structurally cleaner —
  one small crate that depends on bollard with the precise
  minimum features the workspace needs, instead of a
  bollard fork that has to be kept in sync.
