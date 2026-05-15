# Philharmonic Implementation Roadmap

**Audience**: Claude Code, working in the `philharmonic-workspace`
repo.

**Purpose**: The authoritative implementation plan for Philharmonic.
V1 is complete through the first working end-to-end deployment;
active work now lives in the post-v1 dispatch plan (§3 below).

**Current state** (2026-05-15):

- Design: complete. v1 implementation path: **complete
  through Phase 9.** Reference deployment operational since
  2026-05-02.
- **Dispatches done** (post-v1): D1–D6 (embedding-datasets
  feature), D10–D15 (WebUI infrastructure + guides), D12/D16
  (connector enhancements), D17 (mechanics-core tail-promise
  polling), D18 (mechanics-core 0.6 module-surface refactor),
  D20 (workspace-wide webpki-roots TLS via
  `mechanics-http-client`), D21 (`pre-landing.sh` dep-aware
  test filtering), D22 (HTTP/3 client + server-lib +
  server-integration + streaming), D23 (`dockerlet` replaces
  testcontainers), D24 (workspace-wide
  `default-features = false` audit), D25 (mhc hickory CVE
  bump). Plus the `mechanics-h3-quinn` in-tree vendored fork,
  generic `vendor-upstream` xtask, `check-no-registry`
  workspace-hardening guard, and dev-profile
  incremental-build disable. Per-crate version state and
  release notes live in each `CHANGELOG.md`.
- **Post-D22 H3 client stability follow-on**: landed
  2026-05-15 in mhc + mhs (unpublished locally; api-server
  picks up via workspace path-overrides). Pre-wire H3 stream
  failures transparently fall back to HTTP/2 + retry-once on
  fresh QUIC connection; cached `SendRequest` mutex scoped
  to `send_request` only (streams multiplex naturally
  thereafter); 3 s connect/setup timeout; client+server
  QUIC keep-alive + 120 s max-idle. Detail at
  [`docs/codex-reports/2026-05-15-0001-h3-client-stability.md`](codex-reports/2026-05-15-0001-h3-client-stability.md).
  Connector-router 0.1.5 shipped same window to strip
  body-framing + hop-by-hop headers across the mhc-based
  forwarder.
- **Pending**: D7 / D8 / D9 / D19 (Tier 2/3 connectors —
  SMTP, Anthropic, Gemini, DNS). **Gated** on the Audit &
  refactor sweep (§3.K below) reaching its done-state;
  Claude Code resumes Tier-2 dispatches only after Yuka
  signals the sweep complete.
- **Priority: Audit & refactor** (in flight via Yuka's
  direct Codex dispatch, per
  [`HUMANS.md` §Priority: Audit & refactor](../HUMANS.md)):
  workspace-wide pass for maintainability issues, dirty /
  spaghetti code, memory leaks / deadlocks / races, and
  enforcement of the new "bins are thin" principle
  ([design/02 §Bins are thin](design/02-design-principles.md#bins-are-thin),
  operationalised at
  [CONTRIBUTING.md §10.14](../CONTRIBUTING.md#1014-unpublished-bin-crates-minimal-cli-logic-in-libraries)).
  Bug-fixes encountered mid-run are landed; no other
  behaviour change. Tracked under §3.K.
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
Done / pending breakdown lives in the
**Current state** block at the top of this file. Done arcs
trim to one-line done-pointers below (§3.A, C, D, E, F, G,
H, I, J); the still-pending §3.B carries the full dispatch
specs. §3.K is the in-flight Audit & refactor priority that
gates §3.B.

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
  - **Resolv.conf fallback**: when `/etc/resolv.conf` is
    `ENOENT` (typical in minimal-base / distroless /
    scratch container images), fall back to a hardcoded
    Cloudflare resolver set:
    `2606:4700:4700::1111`,
    `2606:4700:4700::1001`,
    `1.1.1.1`,
    `1.0.0.1`. Any other read error
    (permission denied, malformed file, I/O) surfaces as
    a startup error — fallback fires only on ENOENT.
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

### F. Mechanics module surface (1 dispatch) — DONE

DONE 2026-05-14. D18 — `mechanics-core 0.6.0` module-surface
refactor across four rounds. R01 added `[features]` gating
(`rand`, `encoding`, `html`, `console`, `url` default-on;
`mime` opt-in), the `mechanics:console` no-op module, and the
`mechanics:html` htmlize wrapper. R02 added `mechanics:url`
(WHATWG `URL` + `URLSearchParams`). R03 added `mechanics:mime`
(pure format-only `compose` / `parse`; q-p preferred over
base64 for non-ASCII text). R04 refreshed the workflow-authoring
guide en+jp under §"Built-in modules" and added a "JavaScript
runtime notes" subsection covering the setTimeout-removal +
Promise-based replacement patterns. The setTimeout-removal
sub-piece (parent `796f83e`, mechanics-core `cf4f9c6`) closed
the design-06 §"Realm surface (no non-ES globals)" hard-rule
violation that D17 had inadvertently introduced. Codex prompt
archives at
[`docs/codex-prompts/`](codex-prompts/) (search for
`d18-mechanics-module-surface`); R03 backing-crate trade-off
notes at
[`docs/codex-reports/2026-05-14-0001-d18-mechanics-mime.md`](codex-reports/2026-05-14-0001-d18-mechanics-mime.md).

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

### I. HTTP/3 client + server (D22) — DONE

DONE. Client (mhc) speaks opportunistic HTTP/3 via HTTPS RR
+ Alt-Svc discovery; server-lib (mhs) provides
`Http3Server` + `AltSvcLayer` + 0-RTT replay-safety + the
`H3RequestBody` streaming body type; all three release bins
wired with `bind_h3` + Alt-Svc. Codex prompt archives under
[`docs/codex-prompts/`](codex-prompts/) (`d22-*`).

**2026-05-15 stability follow-on** (Yuka-direct Codex
dispatch — not a numbered D-dispatch): pre-wire H3 stream
failures transparently retry once on a fresh QUIC connection
then fall back to HTTPS; `SendRequest` mutex scoped to
`send_request` so concurrent H3 streams to the same origin
multiplex naturally; 3 s connect/setup timeout; client+server
QUIC keep-alive (15 s) + 120 s max-idle to survive
NAT / firewall idle eviction. Detail at
[`docs/codex-reports/2026-05-15-0001-h3-client-stability.md`](codex-reports/2026-05-15-0001-h3-client-stability.md);
ships in unpublished mhc 0.2.4 / mhs 0.1.4 (workspace
path-overrides pick up via rebuild). Same window:
connector-router 0.1.5 strips body-framing + hop-by-hop
headers across the mhc-based forwarder.

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

### K. Audit & refactor (in flight, Yuka direct Codex dispatch)

Workspace-wide audit + refactor sweep. Per
[`HUMANS.md` §Priority: Audit & refactor](../HUMANS.md):

- **Maintainability sweep.** Sub-agent / subdirectory pass
  for spaghetti code, dirty bits, memory leaks, deadlocks,
  races, and other quality issues. Refactor for structured /
  small / deduplicated units. Bug-fixes encountered mid-run
  are landed; no other behaviour change is permitted in the
  same pass.
- **Clean separation of concerns.** Unpublished bin crates
  under `bins/` are reduced to Clap CLI + `main()` glue;
  substantive logic moves into library crates (existing or
  new). Discipline rule:
  [`CONTRIBUTING.md §10.14`](../CONTRIBUTING.md#1014-unpublished-bin-crates-minimal-cli-logic-in-libraries).
  Design principle:
  [`docs/design/02-design-principles.md` §Bins are thin](design/02-design-principles.md#bins-are-thin).
  Current extraction candidates flagged in
  [`docs/design/03-crates-and-ownership.md`](design/03-crates-and-ownership.md):
  `bins/philharmonic-api-server/src/lowerer.rs`,
  `embed_job.rs`, `executor.rs`, `scope.rs`.
- **Chat UI relocation (deferred, decided
  2026-05-15).** Chats are workflow knowledge; the
  framework in principle should not know anything about
  workflows. The current `philharmonic/webui/` Chat UI
  (bundled into the `philharmonic` meta-crate via
  `rust-embed` behind the `webui` feature) is structurally
  in the wrong place by this rule, but is retained because
  it is useful for testing the end-to-end stack. The
  decided future home is **either** an in-tree
  `philharmonic-chat-app` bin (frontend + backend unified)
  **or** a separate project — Yuka picks at relocation time.
  **No removal this pass**: the old Chat UI stays in place
  during the Audit & refactor sweep; the relocation is its
  own follow-on. Recorded in
  [`docs/design/14-open-questions.md` §Questions already
  answered](design/14-open-questions.md#questions-already-answered).

**Dispatch model.** This arc is not driven by Claude's
prompt-and-dispatch flow (no `docs/codex-prompts/` archives).
Yuka spawns Codex directly, scoping each pass herself; Claude
Code maintains the docs in step (this file, CONTRIBUTING.md,
design docs, CLAUDE.md / AGENTS.md cross-refs) as scope
shifts. Done-state is whatever Yuka signals.

**Gate on §3.B.** Tier-2 connectors (D7 SMTP, D8 Anthropic,
D9 Gemini, D19 DNS) are dispatchable only after Yuka signals
this sweep complete. Until then, Claude Code's connector
work is limited to docs / design / prompt-drafting; no
substantive coding dispatch lands.

### Suggested sequencing

**Currently in flight**: §3.K Audit & refactor (Yuka direct
Codex dispatch). Gates §3.B.

**Next dispatchable (post-sweep)**: D7 / D8 / D9 / D19
(Tier 2/3 connectors; all four independent + parallel-safe).

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
