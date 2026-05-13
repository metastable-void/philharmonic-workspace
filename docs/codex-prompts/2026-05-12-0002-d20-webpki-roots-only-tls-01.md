# D20 — workspace-wide webpki-roots-only TLS trust posture (initial dispatch)

**Date:** 2026-05-12
**Slug:** `d20-webpki-roots-only-tls`
**Round:** 01 (initial dispatch — D20, ROADMAP §3.G)
**Subagent:** `codex:codex-rescue`

## Motivation

After the 2026-05-12 ring-removal work (parent commit
`7723e1c`), the workspace's TLS trust-store posture is
inconsistent across subsystems:

- **sqlx** (Postgres and MySQL connectors,
  `philharmonic-api`'s MySQL substrate store): now uses
  `tls-rustls-aws-lc-rs` which pulls in `webpki-roots`
  (bundled Mozilla CA bundle).
- **reqwest** (every outbound HTTP path — `mechanics-core`'s
  endpoint client, `http_forward`, `llm_openai_compat`, and
  the upcoming Tier 3 LLM connectors): uses
  `rustls-platform-verifier 0.7.0` + `rustls-native-certs
  0.8.3` (host-OS native trust store; on Linux that's
  `openssl-probe`-discovered `/etc/ssl/certs`).

Operational consequences of the split:

- Tenant-installed corporate / internal CAs get picked up for
  HTTP outbound but not for SQL outbound.
- Air-gapped or fully-self-signed environments need different
  mitigations for each path.
- The HTTP trust set drifts on OS package updates; SQL trust
  is frozen at compile time. Different rotation cadences.

Locked design choice (2026-05-12, Yuka):

> the best practice is to use the valid CA everywhere; no
> native-roots. please force webpki-roots.

D20 unifies the workspace on `webpki-roots` for every outbound
TLS path. The bundled Mozilla bundle becomes the single source
of CA truth across SQL and HTTP. No native-roots fallback.
`aws-lc-rs` stays as the sole crypto provider (no
re-introduction of `ring`).

## References

- [`docs/ROADMAP.md` §3.G](../ROADMAP.md#g-tls-trust-posture-1-dispatch)
  — D20 entry with the locked design.
- [`CLAUDE.md` §"HTTP client split is strict"](../../CLAUDE.md)
  — "rustls for both; no native-tls, no OpenSSL." D20
  tightens this to explicitly choose webpki-roots over
  native-roots.
- Ring-removal commit (`7723e1c`) — the immediate prior
  work that made sqlx aws-lc-rs+webpki-roots. D20 finishes
  the consistency story for the HTTP side.
- reqwest 0.13.3 feature definitions in
  `~/.cargo/registry/.../reqwest-0.13.3/Cargo.toml`:
  - `rustls = ["__rustls-aws-lc-rs", "dep:rustls-platform-verifier", "__rustls"]`
  - `rustls-no-provider = ["dep:rustls-platform-verifier", "__rustls"]`
  Neither variant offers a webpki-roots-only path; both
  unconditionally pull `rustls-platform-verifier`. The
  workaround is runtime config via
  `ClientBuilder::use_preconfigured_tls()`.

## Context files pointed at

Four production reqwest::Client construction sites that need
to migrate, plus four test sites:

- [`mechanics-core/src/internal/pool/api.rs:119`](../../mechanics-core/src/internal/pool/api.rs#L119)
  — production, mechanics's HTTP endpoint client for script
  `endpoint(...)` calls.
- [`bins/philharmonic-api-server/src/executor.rs:24`](../../bins/philharmonic-api-server/src/executor.rs#L24)
  — production, api-server's StepExecutor client.
- [`philharmonic-connector-impl-http-forward/src/client.rs:18`](../../philharmonic-connector-impl-http-forward/src/client.rs#L18)
  — production, the generic-HTTP connector.
- [`philharmonic-connector-impl-llm-openai-compat/src/client.rs:15`](../../philharmonic-connector-impl-llm-openai-compat/src/client.rs#L15)
  — production, the OpenAI-compatible LLM connector.
- `mechanics-core/src/internal/pool/tests/{mod,queue,lifecycle}.rs`
  — 4 test-only sites that should also migrate for
  consistency (they use `reqwest::Client::new()` for
  testcontainer-backed integration tests).

Per-crate `Cargo.toml`s that gain direct `rustls` +
`webpki-roots` dep additions:

- `mechanics-core/Cargo.toml`
- `bins/philharmonic-api-server/Cargo.toml`
- `philharmonic-connector-impl-http-forward/Cargo.toml`
- `philharmonic-connector-impl-llm-openai-compat/Cargo.toml`

## Outcome

**Superseded 2026-05-13 — not dispatched.** The runtime-bypass
approach described below was the original D20 plan: keep
reqwest and call `ClientBuilder::use_preconfigured_tls()` at
every construction site, leaving `rustls-platform-verifier` as
unused dead weight in the dep tree.

Yuka's 2026-05-13 redirection pivoted D20 to a structural
solution: build a new `mechanics-http-client` crate wrapping
`hyper-rustls` + `webpki-roots`, migrate the four reqwest call
sites to it, and drop reqwest entirely. The dead crates
(`rustls-platform-verifier`, `rustls-native-certs`,
`openssl-probe`) exit the runtime dep tree as a natural
consequence rather than persisting as unused compiled weight.

This file is preserved as the historical record of the
runtime-bypass approach (a reasonable middle path that future
maintainers may want to reach for if the new-crate path runs
into upstream-API instability). The authoritative current spec
lives in
[`docs/ROADMAP.md` §3.G](../ROADMAP.md#g-http-client-transport--tls-trust-posture-1-dispatch).
No Codex dispatch was created for either shape; D20 lands via
Claude-direct implementation per same-session user override.

---

## STRUCTURED-OUTPUT-CONTRACT — READ THIS FIRST

Emit a six-section structured report before `task_complete`,
including the verbatim `RUN STATUS: COMPLETE` or `RUN STATUS:
PARTIAL — <reason>` token. The streak broke at D17 due to a
helper-subagent detachment mid-pre-landing; the contract
itself was not violated by Codex. Maintain it here.

The contract is repeated at the end of the prompt.

---

## Shape (locked decisions)

### What changes

Replace every `reqwest::Client::new()` and
`reqwest::Client::builder()…build()` call in the four
production crates with a route through a small TLS helper
that:

1. Constructs a `rustls::RootCertStore` and populates it
   from `webpki_roots::TLS_SERVER_ROOTS`.
2. Builds a `rustls::ClientConfig` with that root store and
   `with_no_client_auth()`.
3. Calls
   `reqwest::Client::builder().use_preconfigured_tls(config)`
   and returns the builder so call sites can chain their
   existing `.timeout(...)`, `.user_agent(...)`, etc.

Sketch:

```rust
use rustls::ClientConfig;
use rustls::RootCertStore;

fn webpki_roots_client_builder() -> reqwest::ClientBuilder {
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let tls_config = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    reqwest::Client::builder().use_preconfigured_tls(tls_config)
}
```

(The `ClientConfig::builder()` default provider is whatever
crate-feature unification picks — across the workspace,
that's `aws-lc-rs` post-ring-removal. The helper does not
need to opt into a specific provider explicitly.)

### Helper placement (your call)

Two reasonable sub-shapes:

- **Sub-shape A — per-crate inline helper** (preferred
  default): each of the four production crates gets its own
  ~6-line helper function in the file where the call site
  lives, or in a small `tls.rs` module. Duplicates the
  helper four times but keeps each crate self-contained.
  No new shared dep coupling.
- **Sub-shape B — shared helper crate or module**:
  `philharmonic-connector-common` is one candidate home for
  the three connector-side sites (it's already shared by
  the connector-impl crates) but has no current HTTP-client
  responsibilities — adding one is a layering question.
  `mechanics-core` is on the wrong side of the
  Mechanics-Philharmonic independence boundary to host a
  workspace-wide helper for philharmonic-side crates.
  `bins/philharmonic-api-server` is unpublished so it can
  freely depend on whichever shared crate ships the helper.

If sub-shape B feels cleaner in your final layout, go with
it; otherwise default to A. **Document the choice in
residual risks.**

### Per-crate `Cargo.toml` additions

For every production crate listed above:

```toml
rustls = "0.23"           # already at 0.23.40 transitively
webpki-roots = "1"        # already at 1.0.7 transitively
```

No new feature toggles. The transitive `aws-lc-rs` provider
feature on `rustls` is already enabled across the workspace
and stays that way.

`bins/philharmonic-api-server` is `publish = false`, so no
version-bump impact. The three published crates take patch
bumps:

- `mechanics-core` 0.4.1 → 0.4.2
- `philharmonic-connector-impl-http-forward` 0.1.0 → 0.1.1
- `philharmonic-connector-impl-llm-openai-compat` 0.1.2 → 0.1.3

Add corresponding `CHANGELOG.md` entries on each. Keep the
entries self-contained per the Mechanics-Philharmonic
independence rule (mechanics-core's CHANGELOG must not
reference Philharmonic-workspace-internal labels like
"D20" or relative paths into the parent like
`../docs/ROADMAP.md` — describe the user-visible change
directly).

### Tests

- Each production crate's existing test suite must remain
  green byte-for-byte.
- Add at least one new unit test per helper that confirms
  the resulting `reqwest::Client` builds successfully (a
  no-op build proves the type plumbing works). A live TLS
  handshake against a real server is **not** required —
  webpki-roots correctness is upstream's invariant, not
  this crate's.
- The four mechanics-core test sites (`pool/tests/{mod,queue,lifecycle}.rs`)
  should migrate to the helper for consistency. Their
  network-using paths are gated on Docker; on-host
  unit-test runs of those files will still see only the
  helper-built client.

### Verification

```sh
./scripts/pre-landing.sh
```

Auto-detects the modified crates and runs fmt + check +
clippy (-D warnings) + rustdoc + test workspace-wide,
including the `--ignored` Docker phase for the modified
connector + sql crates.

```sh
./scripts/check-api-breakage.sh mechanics-core 0.4.1
./scripts/check-api-breakage.sh philharmonic-connector-impl-http-forward 0.1.0
./scripts/check-api-breakage.sh philharmonic-connector-impl-llm-openai-compat 0.1.2
```

`cargo-semver-checks` per published crate at its current
baseline. The change is internal-behavior; no public API
signature changes expected. Surface any flagged items in
residual risks.

After the change lands and pre-landing is green, verify the
end-state by running:

```sh
for p in mechanics-worker philharmonic-api-server philharmonic-connector-bin; do
    cargo tree -p "$p" --features https -e all 2>&1 \
        | grep -oE "(^| )(rustls-platform-verifier|rustls-native-certs|webpki-roots) v[0-9.]+" \
        | sort -u
done
```

Acceptance: `rustls-platform-verifier` and
`rustls-native-certs` may remain in the dep tree as
transitive deps of reqwest (we can't actually drop them
without an upstream feature change in reqwest 0.13), but
`webpki-roots` should appear for every binary. Surface the
exact dep-tree state in the residuals so we can confirm
the runtime-touched path no longer goes through the
native verifier.

## Prompt (verbatim)

<task>
Ship D20: switch every `reqwest::Client` construction in the
workspace from reqwest's default `rustls` feature path (which
uses `rustls-platform-verifier` + `rustls-native-certs` =
OS-native trust store) to an explicit
`ClientBuilder::use_preconfigured_tls()` call with a
`rustls::ClientConfig` whose `RootCertStore` is populated from
`webpki_roots::TLS_SERVER_ROOTS` (bundled Mozilla CA bundle).
Workspace-wide single source of CA truth; aws-lc-rs stays as
the sole crypto provider.

Single coherent change. No crypto-review gate (trust store
config only; no AAD/AEAD/SCK/COSE changes).

Deliverables (in order):

1. **Add deps** to each of these four `Cargo.toml`s:
   - `mechanics-core/Cargo.toml`
   - `bins/philharmonic-api-server/Cargo.toml`
   - `philharmonic-connector-impl-http-forward/Cargo.toml`
   - `philharmonic-connector-impl-llm-openai-compat/Cargo.toml`

   Adding `rustls = "0.23"` and `webpki-roots = "1"`. Both
   versions are already in the workspace's resolved dep tree
   (0.23.40 and 1.0.7 respectively) — these direct deps
   adopt them.

2. **Add a TLS helper function** per the "Helper placement"
   section in this prompt. Default to sub-shape A (per-crate
   inline). Each helper returns a
   `reqwest::ClientBuilder` already wired with
   `use_preconfigured_tls(webpki_roots_config)`.

3. **Migrate call sites**:
   - `mechanics-core/src/internal/pool/api.rs:119` —
     production.
   - `mechanics-core/src/internal/pool/tests/{mod,queue,lifecycle}.rs`
     — 4 test sites.
   - `bins/philharmonic-api-server/src/executor.rs:24` —
     production.
   - `philharmonic-connector-impl-http-forward/src/client.rs:18`
     — production.
   - `philharmonic-connector-impl-llm-openai-compat/src/client.rs:15`
     — production.

4. **Version bumps + changelog entries** on the three
   published crates (patch bumps; `bins/philharmonic-api-server`
   is `publish = false`, no bump):
   - `mechanics-core` 0.4.1 → 0.4.2
   - `philharmonic-connector-impl-http-forward` 0.1.0 → 0.1.1
   - `philharmonic-connector-impl-llm-openai-compat` 0.1.2 →
     0.1.3

   Each CHANGELOG entry must be self-contained — describe
   the user-visible trust-store change directly. Do **not**
   reference Philharmonic-workspace-internal labels
   (no "D20", no relative paths into the parent like
   `../docs/ROADMAP.md`). This is the
   Mechanics-Philharmonic-independence rule — even though
   only `mechanics-core` is strictly subject to it, applying
   the same discipline to the other connector crates keeps
   the standalone-consumer experience clean.

5. **Tests**: keep existing tests green byte-for-byte; add
   one unit test per crate confirming the helper builds a
   `reqwest::Client` successfully (no live TLS handshake
   needed). The four mechanics-core test files use the
   helper for consistency.

6. **Verification**:
   - `./scripts/pre-landing.sh` (clean across all modified
     crates including --ignored Docker phase).
   - `./scripts/check-api-breakage.sh mechanics-core 0.4.1`
   - `./scripts/check-api-breakage.sh philharmonic-connector-impl-http-forward 0.1.0`
   - `./scripts/check-api-breakage.sh philharmonic-connector-impl-llm-openai-compat 0.1.2`
   - End-state cargo-tree check (see the prompt's
     "Verification" section) showing webpki-roots is present
     and reqwest no longer routes through
     rustls-platform-verifier at runtime construction
     points. `rustls-platform-verifier` may remain as a
     transitive dep — that's expected; what matters is the
     ClientBuilder.use_preconfigured_tls() path bypasses it.

7. **No publish**. Claude reviews and decides post-Codex.

## Hard constraints

- **No native-roots fallback.** Removing
  `rustls-native-certs` from the build entirely is not
  possible without upstream reqwest changes — it'll remain
  as a transitive dep of `rustls-platform-verifier`. What
  matters is the runtime control flow: every reqwest
  client construction routes through the helper and
  therefore through webpki-roots. Confirm via the cargo-tree
  end-state check.
- **`aws-lc-rs` stays as the sole crypto provider.** No
  re-introduction of `ring` (transitively or otherwise).
  Verify the dep tree after the change.
- **No HTTP wire format change.** No public API surface
  change. No new feature flags exposed.
- **No `unsafe` blocks.** No panicking in lib `src/` paths
  (no `.unwrap()` / `.expect()` on `Result`/`Option`, no
  `panic!` / `unreachable!` / `todo!` /
  `unimplemented!` on reachable paths). Tests are exempt.
- **Mechanics-Philharmonic-independence rule** —
  `mechanics-core` is a standalone crate that ships
  independently of Philharmonic. Its CHANGELOG and
  `lib.rs`-visible names must not embed
  Philharmonic-workspace internal identifiers (D20,
  ROADMAP path, etc.). Describe behavior directly.

<structured_output_contract>
**Critical: emit this report before `task_complete`.**

Six sections, in this order:

1. **Summary** — one paragraph: what landed, sub-shape
   chosen (A per-crate inline / B shared crate or module),
   version bumps applied, semver-checks outcome. Include
   the verbatim string `RUN STATUS: COMPLETE` or `RUN
   STATUS: PARTIAL — <reason>` for grep.

2. **Touched files** — exhaustive list with
   `(new|edited|deleted) <path> — <one-line note>`.

3. **Verification results** — exact commands + outcomes:
   - `./scripts/pre-landing.sh` — pass/fail/exit code.
   - `./scripts/check-api-breakage.sh <crate> <baseline>`
     for each of the three published crates — pass/fail
     plus output excerpt for any flagged items.
   - End-state cargo-tree check showing webpki-roots
     present and reqwest's runtime construction paths no
     longer invoking rustls-platform-verifier.

4. **Residual risks / known issues** — including:
   - Sub-shape choice (A vs. B) and why.
   - Whether `rustls-platform-verifier` and
     `rustls-native-certs` remain as transitive deps after
     the change (they probably do, via reqwest's `rustls`
     feature — that's expected; flag with the binary-weight
     cost in mind).
   - Any pre-existing test that started failing and how
     you addressed it.
   - Any reqwest API quirk (e.g., `use_preconfigured_tls`
     being marked unstable / behind a feature) that
     required a feature-flag adjustment.

5. **Git state** — current `HEAD` SHA in the parent
   workspace repo plus each touched submodule. Confirm no
   commits made.

6. **Open questions** — questions for Yuka or Claude.
</structured_output_contract>

<default_follow_through_policy>
- Implement in the order listed: deps → helper → call-site
  migration → version + changelog → verification.
- Run `cargo check --workspace` once early to catch
  feature-unification surprises.
- Prefer sub-shape A (per-crate inline) unless the
  duplicate-helper count makes a shared module obviously
  cleaner — document the choice either way.
- If `use_preconfigured_tls` is gated behind a reqwest
  feature you don't already have, **stop** and report in
  residuals — don't enable a new top-level reqwest feature
  without surfacing it. (Per reqwest 0.13.3's source the
  method is on `ClientBuilder` and available in the
  default builder; verify before changing course.)
- No edits to `docs/`, `docs-jp/`, `HUMANS.md`,
  `CLAUDE.md`, `AGENTS.md`, `CONTRIBUTING.md`,
  `scripts/`, `.claude/` content. Claude reconciles those
  post-merge.
</default_follow_through_policy>

<completeness_contract>
"Done" means:

- All 4 production reqwest::Client construction sites
  routed through the webpki-roots helper.
- All 4 test sites in `mechanics-core/src/internal/pool/tests/`
  routed through the helper.
- Helper function present per chosen sub-shape, ~6 lines
  each, returns `reqwest::ClientBuilder`.
- `rustls = "0.23"` + `webpki-roots = "1"` added as direct
  deps to all four production crates' Cargo.toml.
- Patch version bumps + CHANGELOG entries on the three
  published crates.
- Existing tests green byte-for-byte; one new
  helper-builds-cleanly unit test per crate.
- `./scripts/pre-landing.sh` clean.
- `./scripts/check-api-breakage.sh` run for the three
  published crates with output surfaced.
- End-state cargo-tree shows webpki-roots in every release
  bin's runtime tree; rustls-platform-verifier may remain
  transitively but is not invoked by any
  `reqwest::Client::*` call site (every site routes
  through `use_preconfigured_tls`).
- Six-section structured output report emitted before
  `task_complete`.

Partial completion is acceptable only with `RUN STATUS:
PARTIAL — <reason>`. A half-migrated state where some
clients use webpki-roots and others use platform-verifier
is **worse** than the pre-D20 status quo (introduces
inconsistency where there was at least uniformity on the
HTTP side); if you can't finish, revert to the original
reqwest::Client::* calls and leave the workspace in its
pre-D20 shape.
</completeness_contract>

<verification_loop>
1. Add deps to all four Cargo.tomls.
2. Write the helper(s) per sub-shape choice.
3. Migrate all 4 production + 4 test call sites.
4. `cargo check --workspace` — catches feature-unification
   issues fast.
5. Per-crate `cargo test -p <crate>` for each modified
   crate.
6. Run `./scripts/pre-landing.sh` once.
7. Run `./scripts/check-api-breakage.sh` for each of the
   three published crates.
8. Run the end-state cargo-tree check from the prompt's
   "Verification" section.
9. Emit structured output report.
10. `task_complete`.
</verification_loop>

<missing_context_gating>
If you find yourself needing information not in this prompt
or the cited authoritative sources (ROADMAP §3.G,
CLAUDE.md "HTTP client split"), **stop** and report what's
missing in the structured output's "Open questions"
section.

Specifically: do **not**:

- Touch sqlx feature flags (D20 is reqwest-side; sqlx is
  already on webpki-roots after the ring-removal commit).
- Switch the workspace's crypto provider (aws-lc-rs stays).
- Add a new top-level reqwest feature without surfacing it.
- Add a non-default reqwest feature (e.g.,
  `rustls-tls-native-roots`) to "fix" the platform-verifier
  path that way — the directive is webpki-roots-only via
  runtime config.
- Edit any docs files: `.claude/`, `docs/`, `docs-jp/`,
  `HUMANS.md`, `CLAUDE.md`, `AGENTS.md`, `CONTRIBUTING.md`,
  `scripts/`.
- Publish to crates.io. No `cargo publish` even
  `--dry-run`. Claude reviews and decides post-Codex.
</missing_context_gating>

<action_safety>
Files allowed to be created or modified:

- `mechanics-core/Cargo.toml` (deps + version bump)
- `mechanics-core/CHANGELOG.md` (entry)
- `mechanics-core/src/internal/pool/api.rs` (call site)
- `mechanics-core/src/internal/pool/tests/mod.rs` (call site)
- `mechanics-core/src/internal/pool/tests/queue.rs` (call site)
- `mechanics-core/src/internal/pool/tests/lifecycle.rs` (call site)
- `mechanics-core/src/internal/...` (new helper file if
  sub-shape A; or wherever your chosen organisation
  decides)
- `bins/philharmonic-api-server/Cargo.toml` (deps)
- `bins/philharmonic-api-server/src/executor.rs` (call site)
- `bins/philharmonic-api-server/src/...` (helper if needed)
- `philharmonic-connector-impl-http-forward/Cargo.toml`
  (deps + version bump)
- `philharmonic-connector-impl-http-forward/CHANGELOG.md`
- `philharmonic-connector-impl-http-forward/src/client.rs`
  (call site)
- `philharmonic-connector-impl-http-forward/src/...`
  (helper if needed)
- `philharmonic-connector-impl-llm-openai-compat/Cargo.toml`
  (deps + version bump)
- `philharmonic-connector-impl-llm-openai-compat/CHANGELOG.md`
- `philharmonic-connector-impl-llm-openai-compat/src/client.rs`
  (call site)
- `philharmonic-connector-impl-llm-openai-compat/src/...`
  (helper if needed)
- `Cargo.lock` (regenerates when cargo runs)

Files NOT to touch (flag if you find a reason to):

- Any file under `philharmonic/`, `philharmonic-api/`,
  `philharmonic-policy/`, `philharmonic-workflow/`,
  `philharmonic-store*/`, `philharmonic-connector-common/`,
  `philharmonic-connector-client/`,
  `philharmonic-connector-router/`,
  `philharmonic-connector-service/`,
  `philharmonic-connector-impl-api/`,
  `philharmonic-connector-impl-sql-*/`,
  `philharmonic-connector-impl-embed/`,
  `philharmonic-connector-impl-vector-search/`,
  `philharmonic-connector-impl-llm-anthropic/`,
  `philharmonic-connector-impl-llm-gemini/`,
  `philharmonic-connector-impl-email-smtp/`,
  `philharmonic-connector-impl-dns/`,
  `mechanics/`, `mechanics-config/`, `inline-blob/`,
  `philharmonic/webui/`, the workspace `Cargo.toml`
  `[patch.crates-io]` block.
- `.claude/`, `docs/`, `docs-jp/`, `HUMANS.md`,
  `CLAUDE.md`, `AGENTS.md`, `CONTRIBUTING.md`, `scripts/`.

Git rules: signed-off + signed commits via
`scripts/commit-all.sh` only. **Codex never runs
`commit-all.sh`** — Claude commits after reviewing your
work. No `cargo publish`. No raw `git commit` /
`git push` / `git add` / `git reset` / `git rebase` /
`git revert`. Read-only `git log` / `git diff` /
`git show` is fine.
</action_safety>
</task>
