# Philharmonic Implementation Roadmap

**Audience**: Claude Code, working in the `philharmonic-workspace`
repo.

**Purpose**: The authoritative implementation plan for Philharmonic.
V1 is complete through the first working end-to-end deployment;
active work now lives in the post-v1 dispatch plan (§3 below).

**Current state** (2026-05-14):

- Design: complete. v1 implementation path: **complete
  through Phase 9.** Reference deployment operational since
  2026-05-02.
- **Dispatches done** (post-v1): D1, D2, D3, D4, D5, D6
  (embedding-datasets feature), D10, D11, D12, D13, D14, D15,
  D16 (WebUI + connector enhancements), D17 (mechanics-core
  tail-promise polling), D20 (workspace-wide webpki-roots TLS
  via `mechanics-http-client`), D21 (`pre-landing.sh`
  dep-aware test filtering), D22 (HTTP/3 client + server-lib +
  server-integration + streaming; `mechanics-http-server
  0.1.3` published), D23 (`dockerlet` replaces testcontainers),
  D24 (workspace-wide `default-features = false` audit), D25
  (`mhc` hickory CVE bump). Plus the **setTimeout-removal
  sub-piece of D18** + the `mechanics-h3-quinn` in-tree
  vendored fork + generic `vendor-upstream` xtask +
  `check-no-registry` workspace-hardening guard + dev-profile
  incremental-build disable (2026-05-14 batch).
- **In progress**: D18 (mechanics-core module-surface refactor)
  — R01 landed (feature-gating + `console` no-op + `html`
  htmlize wrapper); R02 dispatched (`url` module); R03
  (`mime`) + R04 (workflow-authoring guide refresh en+jp)
  queued.
- **Pending**: D7 / D8 / D9 / D19 (Tier 2/3 connectors —
  SMTP, Anthropic, Gemini, DNS).
- **§3.J production-security cleanup arc closed 2026-05-14**:
  D23 + D24 + D25 all done; combined with the
  mechanics-h3-quinn vendor + the `deny.toml` Linux-x86_64
  target restriction, the workspace's `ring` ban is now a
  clean no-wrapper full ban.

Per-dispatch detail, daily-log entries, and per-arc
done-state snapshots are preserved at
[`docs/archive/`](archive/) — per-day archives 2026-05-10
through 2026-05-14. The live ROADMAP carries only the
current-state summary + the pending plans (§3.B, §3.F).

Authoritative sources for things this file used to restate but
now cross-references:

- **Conventions / dev environment / git workflow / pre-landing
  / scripts / publishing**: [`CONTRIBUTING.md`](../CONTRIBUTING.md)
- **Architecture / cross-cutting design** (observability, error
  envelope, permission atoms, API token format, canonical JSON,
  statelessness, etc.): [`docs/design/`](design/)
- **Operating principles for Claude / Codex**:
  [`CLAUDE.md`](../CLAUDE.md), [`AGENTS.md`](../AGENTS.md)
- **Two-gate crypto review protocol**:
  [`.claude/skills/crypto-review-protocol/SKILL.md`](../.claude/skills/crypto-review-protocol/SKILL.md)

If a design doc is wrong or incomplete, update the doc first,
then implement — **do not invent architectural decisions**.

---

## 1. Completed v1 milestone archive

Phases 0–9 (workspace setup → reference deployment) all landed.
The detailed per-phase plans, definition-of-done, completed-
crate inventory, and pre-Phase-9 cross-cutting notes were
trimmed from this roadmap on 2026-05-10. The full pre-trim text
is preserved verbatim at
[`docs/archive/2026-05-10-readme-roadmap-trim.md`](archive/2026-05-10-readme-roadmap-trim.md)
(under "Pre-trim `docs/ROADMAP.md`" → §4 "Completed v1 Milestone
Archive" and §8 "Definition of done for v1").

One-line summary: **Phase 0** workspace setup, **Phase 1**
`mechanics-config` extraction, **Phase 2** `philharmonic-policy`,
**Phase 3** `philharmonic-connector-common`, **Phase 4**
`philharmonic-workflow`, **Phase 5** connector triangle (client +
service + router) under Yuka's two-gate crypto review, **Phase 6**
first connector implementations (`http_forward`,
`llm_openai_compat`), **Phase 7 Tier 1** SQL Postgres / SQL MySQL /
stateless vector search / local embedding (with `inline-blob`),
**Phase 8** `philharmonic-api 0.1.0`, **Phase 9** integration +
reference deployment (2026-05-02).

Historical implementation detail also lives in dated
`docs/codex-prompts/`, `docs/codex-reports/`,
`docs/notes-to-humans/`, and
`docs/crypto/{proposals,approvals}/` files.

---

## 2. Crypto review protocol (pointer)

The two-gate cryptographic-review protocol (Gate 1 = approach
pre-approval; Gate 2 = post-implementation code review before
publish) lives in
[`.claude/skills/crypto-review-protocol/SKILL.md`](../.claude/skills/crypto-review-protocol/SKILL.md).
That file is the authoritative spec for what triggers the gates,
what each gate requires, and the test-vector discipline.

The Phase-5 / Phase-9 / 2026-05-04 Gate-1 records and approvals
are committed under `docs/crypto/{proposals,approvals}/`.

---

## 3. Post-v1 dispatch plan

Phase 9 is complete (2026-05-02) and the reference deployment is
operational. The work below is post-v1 / post-GW: it does not
block deployment and is sequenced for the next development
cycle. Each numbered item is one Codex dispatch with its own
archived prompt under `docs/codex-prompts/` (see
[`.claude/skills/codex-prompt-archive/SKILL.md`](../.claude/skills/codex-prompt-archive/SKILL.md)).
The single `(Gate 1)` item is **not** a Codex dispatch — Claude
drafts the proposal, Yuka reviews per the two-gate crypto-review
protocol (§2).

Total: **26 Codex dispatches plus 1 Gate-1 proposal.**
Done / in-progress / pending breakdown lives in the
**Current state** block at the top of this file. Done arcs
trim to one-line done-pointers below (§3.A, C, D, E, G,
H, I, J); the still-pending sections (§3.B, §3.F) carry
the full dispatch specs.

### A. Embedding datasets (6 dispatches + 1 Gate-1) — DONE

DONE 2026-05-10. D1 / D2 / D3 / D4 / D5 / D6 + both crypto
gates. Authoritative design:
[`docs/design/16-embedding-datasets.md`](design/16-embedding-datasets.md).
Per-dispatch detail at
[`docs/archive/2026-05-11-roadmap-completed-arc-trim.md`](archive/2026-05-11-roadmap-completed-arc-trim.md).

### B. Phase 7 Tier 2/3 connector implementations (4 dispatches)

Three of these (D7 / D8 / D9) take an existing `0.0.x`
placeholder to a `0.1.0` substantive implementation; D19 is
a **new crate** (no placeholder yet on crates.io, no
submodule wired into the workspace). None touch the crypto
path; the connector-service framework already validates
tokens and decrypts payloads — implementations only need
to implement the `Implementation` trait.

- **D7** `philharmonic-connector-impl-email-smtp` (Tier 2).
  Implement per
  [`docs/design/08-connector-architecture.md` §SMTP](design/08-connector-architecture.md#smtp).
  Hard requirements (locked 2026-05-12 via HUMANS.md):
  - **Port 25 rejected** unconditionally.
  - **Username + password required**; anonymous submission
    refused at config validation.
  - **Explicit `connection_mode` knob** in the endpoint
    config: `starttls` / `smtps` / `auto` (default). When
    set, wins over the port-driven inference. The full
    SMTP server config — host, port, credentials, mode,
    strictness — lives in the endpoint config and is
    never visible to workflow code.
  - **Port-driven defaults** (when `connection_mode` is
    `auto`): 587 → STARTTLS, 465 → SMTPS, otherwise
    STARTTLS. **Auto-discovery** (`auto` + no port):
    try 587/STARTTLS, then 465/SMTPS.
  - **Four-valued `tls_strictness` enum**: `strict`
    (default), `sloppy`, `opportunistic`,
    `opportunistic_sloppy`. Independent of
    `connection_mode`.
  - **Request shape** `{mail_from, recipients[], body}`
    with minimal MIME envelope fixing (insert
    `MIME-Version` / `Date` / `Message-Id` / default
    `Content-Type` only when the submission server would
    reject otherwise; CRLF-normalise line endings; never
    inject security-relevant headers).
  - Transport: `lettre` over `rustls`.
- **D8** `philharmonic-connector-impl-llm-anthropic` (Tier 3).
- **D9** `philharmonic-connector-impl-llm-gemini` (Tier 3) —
  must support **both** Google API surfaces for Gemini:
  - **Google AI Studio**
    (`https://generativelanguage.googleapis.com/`): API-key
    auth, simplest single-tenant deployment shape, free-
    tier-friendly.
  - **Vertex AI on GCP**: Service Account JSON key auth.
    The SA JSON lives **inside** the SCK-encrypted endpoint
    config alongside the API-Studio mode's API key —
    consistent with how `llm-openai-compat` carries its
    `api_key` field. Encryption-at-rest is handled by the
    existing SCK boundary; per-tenant credential rotation
    happens via the existing endpoint-config rotation flow.
    Endpoint shape under
    `<region>-aiplatform.googleapis.com/v1/projects/<project>/`.

  The runtime endpoint config carries a discriminator
  selecting which mode is active; the impl handles auth
  + endpoint construction accordingly per mode. Detailed
  shape (discriminator field name, exact field names for
  the Vertex mode's project / region / SA JSON,
  OAuth2 access-token caching for Vertex AI) defers to
  D9's prompt-drafting time.

- **D19** `philharmonic-connector-impl-dns` (Tier 2,
  **new crate**). Implement per
  [`docs/design/08-connector-architecture.md` §DNS](design/08-connector-architecture.md#dns).
  Hard requirements (locked 2026-05-12 via HUMANS.md):
  - Arbitrary DNS querying via the system's stub
    resolver (consults `/etc/resolv.conf`). No custom
    recursive resolver, no caching layer beyond the OS.
  - `IN` class only.
  - Endpoint config carries optional `allowed_types`
    (RR type allowlist), `allowlist_zones`, and
    `blocklist_zones` (domain-suffix gates). When both
    lists exist: query passes if its zone is
    allowlisted AND not blocklisted. Blocklist is a
    strict overlay-deny.
  - Resolver library: `hickory-resolver` (formerly
    `trust-dns-resolver`) in system-config mode —
    pure-Rust, async, no `unsafe`.
  - Capability name `dns_query`; realm `dns`. Request
    `{name, type, timeout_ms?}`; response
    `{records: [{type, name, ttl, data}, ...]}`. v1
    emits rdata as presentation-format strings;
    per-type structured objects are a sub-shape
    decision at prompt-drafting time.

  **Pre-D19 setup** (Yuka, one-time):
  1. Create the
     `philharmonic-connector-impl-dns` GitHub repo
     under `metastable-void` (mirror the layout of
     the existing connector submodules).
  2. Reserve the crate name on crates.io by publishing a
     `0.0.1` placeholder.
  3. Add a submodule entry to `.gitmodules` + the
     workspace root.
  4. Add the crate to the workspace `Cargo.toml`
     `[workspace] members` list.

  After setup, Claude drafts the D19 prompt; Codex
  implements + tests inside the new submodule. No
  crypto gate — DNS connector is HTTP-realm style
  (token + encrypted payload over the connector
  framework); just one more `Implementation` trait
  impl.

Independent of one another and of section A; safe to run in
parallel (modulo the D19 setup prerequisite).

### C. Connector enhancements (2 dispatches) — DONE

DONE 2026-05-10 / 2026-05-11. D12 (`llm_openai_compat`
`custom_headers` knob) + D16 (`tool_call_fallback_auto`
dialect variant). Per-dispatch detail at
[`docs/archive/2026-05-11-roadmap-completed-arc-trim.md`](archive/2026-05-11-roadmap-completed-arc-trim.md).

### D. WebUI infrastructure, features, and docs (5 dispatches) — DONE

DONE 2026-05-02 / 2026-05-10 / 2026-05-11. D10 (CodeMirror 6),
D11 (workflow-authoring guide rewrite en+jp), D13 (chat-style
testing UI), D14 (markdown rendering with DOMPurify), D15
(`abstract_config` structured editor). Per-dispatch detail
at
[`docs/archive/2026-05-11-roadmap-completed-arc-trim.md`](archive/2026-05-11-roadmap-completed-arc-trim.md).

### E. Execution-substrate runtime semantics (1 dispatch) — DONE

DONE 2026-05-12. D17 — `mechanics-core` response-detached
background-poll runtime: the worker's run-job response returns
when the script's top-level settles; unawaited promises and
endpoint calls continue polling on the worker tokio task until
quiescence or `max_execution_time`. The D17-added `setTimeout`
realm global was reverted 2026-05-14 under D18 per HUMANS.md's
"no non-ES globals" hard rule (see §3.F). Authoritative
behaviour spec at
[`docs/design/06-execution-substrate.md` §Tail-promise polling](design/06-execution-substrate.md#tail-promise-polling).
Per-dispatch detail at
[`docs/archive/2026-05-12-roadmap-d17-done-d7-spec-d18-added.md`](archive/2026-05-12-roadmap-d17-done-d7-spec-d18-added.md).

### F. Mechanics module surface (1 dispatch)

Surfaced via HUMANS.md §"MIME module at `mechanics-core`" and
the surrounding directive on feature-gating non-endpoint
modules.

- **D18** `mechanics-core` module-surface refactor. Make every
  non-endpoint built-in module feature-gated and ship four
  new modules (one non-default, three default):

  - **Refactor**: every existing non-endpoint module
    (`mechanics:rand`, `mechanics:uuid`, `mechanics:encoding`)
    moves behind a Cargo feature flag. Pre-existing modules
    keep their previous availability by being members of
    default features.
  - **Feature `rand`** (default) — `mechanics:rand` +
    `mechanics:uuid`. Without it, `Math.random()` is seeded
    with zero (per HUMANS.md).
  - **Feature `encoding`** (default) — form-urlencoded,
    base64, base32, hex. Existing surface; gets gated.
  - **Feature `html`** (default, **new**) — wraps the
    `htmlize` crate: `htmlize::escape_text` → `escapeText`,
    `htmlize::escape_all_quotes` → `escapeAttribute`,
    `htmlize::unescape` → `unescapeText`,
    `htmlize::unescape_attribute` → `unescapeAttribute`.
  - **Feature `url`** (default, **new**) — WHATWG-compliant
    `mechanics:url`. Default export `URL`; named export
    `URLSearchParams`. Backed by the `url` crate.
  - **Feature `console`** (default, **new**) — minimal
    WHATWG-compliant `mechanics:console`. Levels: `log`,
    `info`, `warn`, `error`, `debug`. **No I/O of any
    kind** — no stdout, no stderr, no host-side
    `tracing` emission. Per Yuka 2026-05-14: workflows
    run in a sandboxed realm where any direct I/O would
    violate the stateless-per-job contract and leak host
    information. Initial implementation is a complete
    no-op: the level methods exist with the expected
    WHATWG signatures (variadic args, format-spec
    handling) and silently return `undefined`. **Future
    work** (separate dispatch, possibly breaking): capture
    `console.*` invocations made before the script's
    `return` into a structured field on the worker
    response (e.g. `RunJobResponse.logs: Vec<ConsoleEntry>`).
    The capture window is *pre-return*; D17's tail-promise
    polling phase is post-return and won't capture
    further `console.*` calls (they continue to no-op).
    This means workflow authors can use `console.log` for
    structured debugging during development without
    needing a worker-config knob, and operators don't
    have to worry about runaway log emission from
    malicious workflows.
  - **Feature `mime`** (non-default, **new**) —
    structured MIME composer + parser at `mechanics:mime`.
    `import { compose, parse } from 'mechanics:mime'`.
    Handles Base64 and multipart cleanly; emits
    standards-compliant MIME messages. Format-only;
    does **not** know about HTML, headers semantics, or
    SMTP. Useful both standalone and as a workflow-author
    helper for the D7 `email_smtp` connector
    (workflows can keep hand-writing the `body` string
    when `mime` isn't enabled).

  **setTimeout-removal sub-piece: DONE 2026-05-14** (parent
  commit `796f83e`; mechanics-core submodule `cf4f9c6`). The
  non-ES `setTimeout` global that D17 had inadvertently added
  to the Mechanics realm is now gone — closes the design-06
  §"Realm surface (no non-ES globals)" hard-rule violation.
  The full D18 module-surface refactor below is the remaining
  scope.

  Original captured scope: **remove the non-ES `setTimeout`
  global** that D17 inadvertently added to the Mechanics
  realm. Per HUMANS.md the rule is "no non-ES globals" (hard
  rule, just reiterated 2026-05-13); `setTimeout` and
  `setInterval` are WHATWG/Web Platform globals, not
  ECMAScript spec, so neither belongs in the realm's global
  object or in any `mechanics:*` module export. Engine-level
  timer plumbing (Boa's `TimeoutJob` queue + the D17
  tail-promise polling loop) stays — it's needed for
  spec-conformant Promise microtask handling and shouldn't
  be torn out. The fix landed verbatim as captured:
  - Drop `set_timeout` + `install_timer_builtins` from
    `mechanics-core/src/internal/runtime.rs` (D17's
    additions; ~25 lines).
  - Rewrite the two `setTimeout`-using fixtures in
    `mechanics-core/src/internal/pool/tests/runtime_behavior.rs`
    (the D17 tail-poll behavior tests) to exercise the same
    invariants via Promise-based async patterns (e.g.
    `new Promise(resolve => endpoint(...).then(resolve))`)
    so D17's tail-poll behavior remains tested without the
    `setTimeout` surface.
  - Update `docs/design/06-execution-substrate.md`
    §Tail-promise polling to drop the `setTimeout`
    callback bullet (Promise chains + endpoint calls remain
    as the only legitimate sources of tail work).
  - Update `docs/ROADMAP.md` §2 daily-log (2026-05-12 D17
    entry) and §3.E to note the `setTimeout` addition was
    reversed under D18.
  - `mechanics-core/CHANGELOG.md`: new `[0.6.0]` entry
    naming the global-removal as a breaking change
    (alongside the rest of D18's surface).
  - Add an `Outcome` addendum to the D17 prompt archive +
    the 2026-05-12 ROADMAP trim archive linking forward
    here, so future readers tracing the D17 reversal don't
    chase a phantom feature.

  Hard constraints:

  - `jsdom` won't work with Mechanics — the runtime has no
    non-ES globals on purpose. Modules expose ES-style
    `import`s only; no implicit globals. This is a **hard
    rule** (reiterated by Yuka 2026-05-13). `setTimeout`,
    `setInterval`, `requestAnimationFrame`, `queueMicrotask`,
    and anything else from the Web Platform / WHATWG global
    surface MUST NOT appear as Mechanics realm globals or
    as `mechanics:*` module exports. Engine-internal job
    queue plumbing is fine — the constraint is the user-
    facing surface, not the runtime's internals.
  - No new public-API breakage on the Rust side beyond the
    feature gates themselves and the `setTimeout`-global
    removal (existing Rust consumers stay green with default
    features on; JS workflows that called `setTimeout()`
    explicitly will fail — that's the intended behavior).
  - All modules respect Mechanics's per-job stateless
    contract — no cross-job state, no globalThis
    mutations that persist.
  - Workflow-authoring guide (en + jp) re-synced as part of
    the dispatch per HUMANS.md §"Keep the workflow authoring
    guide up-to-date" — the new modules need recipe-shaped
    documentation alongside the existing connector
    walkthroughs.

  Claude drafts the Codex prompt; Codex implements + tests.
  No crypto-review gate — runtime module surface only.
  Independent of D7-D9.

### G. HTTP-client transport + TLS trust posture (1 dispatch) — DONE

DONE 2026-05-13. D20 — built `mechanics-http-client`
(hyper-rustls + webpki-roots + aws-lc-rs; published 0.1.0)
and migrated every workspace reqwest call site to it. All
three release binaries have runtime dep trees free of
`reqwest`, `rustls-platform-verifier`, `rustls-native-certs`,
and `ring`. Cascade-bumps documented at
[`docs/archive/2026-05-13-roadmap-d20-done.md`](archive/2026-05-13-roadmap-d20-done.md).

### H. Workspace tooling (1 dispatch) — DONE

DONE 2026-05-13. D21 — `scripts/pre-landing.sh` dep-aware
test filtering: default test phase narrows to
{dirty crates} ∪ {transitive reverse-dep closure of dirty},
with workspace-wide fallback on `Cargo.toml` / `Cargo.lock` /
`scripts/` dirty. `--full` flag forces the pre-D21
behaviour. Archive at
[`docs/archive/2026-05-13-roadmap-d21-done.md`](archive/2026-05-13-roadmap-d21-done.md).

### I. HTTP/3 client + server (D22 — 4 rounds across 2 sessions) — DONE

DONE 2026-05-13 / 2026-05-14. D22 client (mhc 0.2.0
opportunistic HTTP/3 via HTTPS RR + Alt-Svc discovery) +
D22 server-lib (mhs 0.1.0 with Http3Server / AltSvcLayer /
0-RTT replay-safety) + D22 server-integration round 01
(three release bins wired with `bind_h3` + Alt-Svc;
mechanics 0.5.2, mhs 0.1.2) + D22 server-integration round
02 (mhs 0.1.3 with proper `http_body::Body` streaming via
the new `H3RequestBody` type; the 16 MiB axum-adapter buffer
cap from round 01 is gone). mhs 0.1.3 published to
crates.io 2026-05-14 13:55 JST. Codex prompt archives under
[`docs/codex-prompts/`](codex-prompts/) (search for
`d22-`).

### J. Production-security dep cleanup (D23 + D24 + D25 — all done; arc closed) — DONE

DONE 2026-05-13 / 2026-05-14. D23 (`dockerlet 0.1.0`
replaces testcontainers; warm-container + atexit cleanup +
`auto_remove: true`) + D24 (workspace-wide
`default-features = false` audit; 24 patch-bumps; banned-
dep posture: `pyo3` / `maturin` / `openssl-sys` /
`native-tls` / `rustls-platform-verifier` /
`rustls-native-certs` all no-wrapper full bans) + D25 (`mhc
0.2.1` clearing hickory-resolver CVEs `RUSTSEC-2026-0118`
+ `RUSTSEC-2026-0119`). 2026-05-14 follow-on closing the
last `ring` wrapper exception: `mechanics-h3-quinn 0.0.10`
in-tree vendored fork (published) + `deny.toml`
`[graph] targets` restricted to
`x86_64-unknown-linux-{gnu,musl}` — `ring` is now a clean
no-wrapper full ban. Per-arc archives at
[`docs/archive/2026-05-13-roadmap-d23-d25-done.md`](archive/2026-05-13-roadmap-d23-d25-done.md)
and
[`docs/archive/2026-05-14-roadmap-d24-done.md`](archive/2026-05-14-roadmap-d24-done.md).

### Suggested sequencing

**Next dispatchable**: D18 (in flight; R02 dispatched, R03
+ R04 queued) → then D7 / D8 / D9 / D19 (Tier 2/3
connectors; all four independent + parallel-safe).

- **D18** (`mechanics-core` module-surface refactor) is
  fully spec'd from §3.F above. Rounds 01 + setTimeout-
  removal sub-piece landed 2026-05-14; R02 (url) dispatched;
  R03 (mime) + R04 (workflow-authoring guide refresh)
  queued.
- **D7** (`philharmonic-connector-impl-email-smtp`,
  Tier 2) — `email_send` wire shape locked in
  [`docs/design/08-connector-architecture.md` §SMTP](design/08-connector-architecture.md#smtp);
  prompt draft ready.
- **D8** (`philharmonic-connector-impl-llm-anthropic`,
  Tier 2) — fully spec'd from
  [`docs/design/08-connector-architecture.md` §llm_anthropic](design/08-connector-architecture.md#llm_anthropic--config);
  prompt draft ready.
- **D9** (`philharmonic-connector-impl-llm-gemini`, Tier 2)
  carries the dual-mode AI Studio + Vertex AI requirement;
  Claude proposes the discriminator field, Vertex-mode
  field names, and OAuth2 access-token caching strategy in
  the prompt; Yuka overrides at prompt-review time if she
  has a preference.
- **D19** (DNS connector) — fully spec'd from
  [`docs/design/08-connector-architecture.md` §DNS](design/08-connector-architecture.md#dns),
  setup-unblocked 2026-05-12 (submodule wired, crates.io
  `0.0.0` placeholder published). Prompt draft ready.

### Dispatch discipline reminder

Per [`.claude/skills/codex-prompt-archive/SKILL.md`](../.claude/skills/codex-prompt-archive/SKILL.md)
and [`CONTRIBUTING.md §15.2`](../CONTRIBUTING.md#152-codex-prompt-archive):
every Codex prompt is archived to
`docs/codex-prompts/YYYY-MM-DD-NNNN-<slug>.md` and committed
**before** the spawn. Each dispatch above corresponds to one
archived prompt; multi-round work shares the daily-sequence
`NNNN` and increments the trailing `-NN`.
