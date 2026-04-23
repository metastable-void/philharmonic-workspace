# Phase 6 `http_forward` scoping — blocker before Codex dispatch

**Author**: Claude Code
**Date**: 2026-04-23 (木, JST 19:30-ish — inside extended hours)
**Audience**: Yuka (decision required before Codex dispatch can happen)
**Status**: **BLOCKED** — three decisions needed below. I stopped
before writing any code or archiving a Codex prompt.

## TL;DR

I went to scope the first Phase 6 implementation crate
(`philharmonic-connector-impl-http-forward`, recommended earlier
as the no-crypto no-external-SaaS canary) and discovered the
`Implementation` trait that doc 08 says impl crates should
implement **does not exist in any connector crate**. Phase 5
shipped the verify/decrypt/registry primitives but stopped short
of the runtime trait. Phase 6 cannot proceed until that gap is
closed, and the decision of *where* and *how* it's closed is
large enough that it's yours to make, not mine.

I also noted three smaller scope decisions below that are worth
your sign-off before I draft a Codex prompt.

## Context (what I checked)

1. **ROADMAP.md §"Phase 6 — First implementations"** — lists
   `http_forward` + `llm_openai_compat` as the two crates, with
   the `http_forward` task saying "Config shape reuses
   `mechanics_config::HttpEndpoint`… integration tests against
   `httpbin.org` or a local test server, use `reqwest` with
   `rustls-tls`."
2. **docs/design/08-connector-architecture.md** — §"Implementation
   trait" (line 444-467) specifies:
   ```rust
   #[async_trait]
   pub trait Implementation: Send + Sync {
       fn name(&self) -> &str;
       async fn execute(
           &self,
           config: &JsonValue,
           request: &JsonValue,
           ctx: &ConnectorCallContext,
       ) -> Result<JsonValue, ImplementationError>;
   }
   ```
   and §"Per-implementation crates" (line 253-262) says: "Each
   implements the `Implementation` trait **from
   `connector-service`**. Implementation crates depend only on
   `connector-service` and `connector-common`."
3. **philharmonic-connector-service/src/lib.rs** (just published
   as v0.1.0 this afternoon) — exports `decrypt_payload`,
   `verify_token*`, `verify_and_decrypt*`,
   `VerifiedDecryptedPayload`, `RealmPrivateKeyRegistry`,
   `MintingKeyRegistry`. **No `Implementation` trait.**
4. **Full-workspace grep** for `trait Implementation`,
   `trait Connector`, `pub trait` across connector-common,
   connector-service, connector-client, connector-router —
   the only trait found is `Forwarder` in router/dispatch.rs.
   The Implementation trait was never implemented.
5. **mechanics-config::HttpEndpoint** (endpoint/mod.rs, line
   149-516) — complete: `build_url()`, `build_headers()`,
   `prepare_runtime()` returning a `PreparedHttpEndpoint`. Good
   news — the http_forward impl can lean on this verbatim. Note:
   doc 08 refers to a `validate_config` method that doesn't
   exist; the actual load-time validation hook is
   `prepare_runtime()`. Minor doc drift, worth a separate
   fix-forward.

## Blocker 1 (primary) — where does the `Implementation` trait live?

Doc 08 says "from `connector-service`", but we just published
`connector-service = 0.1.0` four hours ago without it. Adding the
trait now is a real API change to a freshly-published crypto
crate, and there are three plausible paths. Each has a different
shape, and none of them is obviously right.

### Option A — add `Implementation` to `connector-service` 0.2.0

Stick to doc 08 as written. Bump `connector-service` to 0.2.0
with the new trait. Client/router can stay at 0.1.0 since they
don't depend on the trait.

**Pros**: matches the design doc verbatim; impl crates only
depend on `connector-service` + `connector-common` per doc 08.

**Cons**:
- Breaking minor-version bump of a Gate-2-reviewed crypto crate
  four hours after publish. Would need a separate Gate 1 / Gate 2
  cycle? Strictly speaking the trait addition itself is *not*
  crypto-sensitive (no new key handling, no construction choice),
  but the policy in `.claude/skills/crypto-review-protocol.md`
  reads "crypto-sensitive paths are gated" — the whole crate is
  one such path, so I default to asking.
- Adds deps to connector-service: `async_trait` (or a stable-async
  alternative, see Blocker 2), `serde_json` (currently only a
  dev-dep), maybe `tokio` transitively through reqwest-based
  impls. None of these are crypto-relevant but they expand the
  crate's dep surface.

### Option B — new crate `philharmonic-connector-impl-api`

Define the trait in a new non-crypto crate that impl crates
depend on, and which `connector-service` consumes at runtime
when dispatching. Keeps `connector-service`'s dep surface narrow
and its Gate-2 snapshot stable. Costs: diverges from doc 08; adds
a 25th crate to the workspace; needs a naming/publish dance.

### Option C — define the trait locally in each impl crate, wire it up later

Impl crates define their `execute(config, request, ctx) -> …`
as a free function with the documented signature, without
blessing a formal trait yet. When Phase 9 integration wires up a
realm binary, *that* binary defines the trait and implements it
as thin adapters around each impl crate's free function.

Pros: unblocks Phase 6 today without touching any published
crate. The "impl crate is just a library of domain functions"
framing is actually closer to how most Rust ecosystem code works.
Cons: diverges from doc 08; risks drift if the free-function
signatures across impl crates aren't kept uniform.

**My recommendation**: **Option A** if you're comfortable with a
breaking `connector-service` 0.1.0 → 0.2.0 bump in under a day
and a focused Gate-1 scope for the trait definition only
(no crypto changes); **Option C** if you want Phase 6 to move
tonight without any publish-track friction, with Option A
deferred to Phase 8/9 when we actually wire a realm binary.
I lean Option C for *tonight's* session and think A is the right
durable answer once the trait is actually exercised by a
service binary.

## Blocker 2 — async-in-traits mechanism

The trait is documented as `#[async_trait]`. Three ways to spell
it in 2026:

- **`async_trait` crate** — macro, works everywhere, allocates
  `Box<dyn Future>` per call. Established, battle-tested.
- **Native async fn in traits** (Rust 1.75+) — no macro, zero
  allocation, but `dyn Implementation` becomes hard: callers
  have to bound the return future as `Send`, and object-safety
  needs `async-trait-bounds` or manual `Pin<Box<…>>` gymnastics.
- **Return-position `impl Future`** — `fn execute(…) -> impl
  Future<Output = …> + Send`. Clean syntactically but has the
  same `dyn`-compat issues as native async.

The service framework will want to hold impls in a registry
(`HashMap<String, Box<dyn Implementation>>`), which means
**dyn-compat is the relevant constraint**. That strongly favors
`async_trait`. Doc 08's `#[async_trait]` annotation is probably
intentional.

**My recommendation**: go with `async_trait` crate. Unless you
want a specific 2026-era alternative.

## Blocker 3 — sync vs. async HTTP stack for `http_forward`

ROADMAP says "use `reqwest` with `rustls-tls`". reqwest is
async-by-default (tokio). The existing workspace HTTP client
code (xtask/openai-chat.rs, xtask/web-fetch.rs) uses **ureq 3
+ rustls**, which is sync. If Phase 6 introduces reqwest it's
the first tokio surface in the runtime crates (xtask is dev-only
and doesn't count).

Options:
- **reqwest + tokio** per ROADMAP. Idiomatic for async-trait
  impls, brings in tokio + hyper + tower transitively. Adds real
  weight to the workspace dep graph.
- **ureq 3 (sync)**, wrap in `tokio::task::spawn_blocking` at
  the trait boundary. Keeps the workspace single-HTTP-stack,
  quirky at the Impl boundary.
- **`reqwest` with `blocking` feature** — ugly for an
  async-trait crate.

**My recommendation**: reqwest + rustls-tls, async all the way.
ROADMAP already chose this; Phase 7 impls (sql-postgres with
sqlx, email-smtp with lettre) will be async anyway, so the
workspace is committing to tokio at some point regardless.
Better to pick it now than retrofit.

## Smaller decisions (ok to batch-approve or override)

- **Integration tests**: ROADMAP says "httpbin.org **or** a
  local test server". httpbin.org is flaky in CI. I'd lean
  `wiremock-rs` for deterministic in-process mocks + a single
  optional `httpbin.org` smoke test gated on an env flag.
- **`exposed_response_headers` normalization**: `HttpEndpoint`'s
  `prepare_runtime` already normalizes the allowlist; the impl
  just has to normalize incoming response-header names to match.
  Doc 08 spells this out. No ambiguity.
- **Doc-drift**: `validate_config` in doc 08 line 609 doesn't
  exist on `HttpEndpoint`. The actual hook is `prepare_runtime`.
  Should update doc 08 to match the code in a separate commit.

## What I need from you

1. **Blocker 1 verdict**: A, B, or C?
2. **Blocker 2 verdict**: `async_trait` crate? (or another call?)
3. **Blocker 3 verdict**: reqwest + tokio is ok for Phase 6?
4. **Ok to switch to wiremock-rs** for integration tests?
5. **Ok to fix doc-08's `validate_config` reference** in the
   same commit as the Phase 6 scoping note (cheap, under 10 lines
   of docs edit)?

If you want to minimize tonight's work: **C + async_trait +
reqwest + wiremock + yes to the doc fix** gets a Codex prompt
ready to archive tomorrow morning. If you want to do it
"properly" end-to-end: **A + async_trait + reqwest + wiremock**
is a longer path that blocks on another Gate-1/Gate-2 cycle on
connector-service.

I'll wait for your direction. Current JST 19:32 木 — still
inside the extended-hours window, ~1 hour until 20:30, but I'd
rather not dispatch Codex after I've left the loop for the
night.
