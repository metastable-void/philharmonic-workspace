# 2026-05-12 ROADMAP rewrite — D17 done, D7 spec-locked, D18 added

Pre-rewrite verbatim text of the `docs/ROADMAP.md` §3
preamble, §3.B (Phase 7 Tier 2/3 connector implementations
when D7 was still a stub), §3.E (D17 in the design-landed
state, before Codex implemented), and the Suggested
sequencing footer.

Rewritten on 2026-05-12 because:

1. **D17 landed** — `mechanics-core` 0.4.0 → 0.4.1 with
   tail-promise polling. Status update in §3.E and the
   done/remaining counts in §3 preamble.
2. **D7 unblocked** — HUMANS.md surfaced a complete spec
   for `email_send` (connection policy, TLS strictness
   enum, request shape, MIME validation rules). That spec
   moved into `docs/design/08-connector-architecture.md`
   §SMTP, and §3.B D7 was rewritten to point at it as the
   authoritative spec.
3. **D18 added** — HUMANS.md also surfaced a substantial
   `mechanics-core` module-surface refactor (feature
   gating + new `mime` / `url` / `console` / `html`
   modules). Added as the new §3.F dispatch.

Prior trim archive: [`2026-05-11-roadmap-completed-arc-trim.md`](2026-05-11-roadmap-completed-arc-trim.md).

---

## §3 preamble (pre-rewrite)

```
Total: **17 Codex dispatches plus 1 Gate-1 proposal.**
**D1, D2, D3, D4, D5, D6, D10, D11, D12, D13, D14, D15, D16
are done** (13 of 17). Gate 1 and Gate 2 both approved.
Remaining: D7, D8, D9 (Tier 2/3 connectors), D17
(execution-substrate response-detached background-poll).
```

## §3.B (pre-rewrite) — D7 stub state

```
### B. Phase 7 Tier 2/3 connector implementations (3 dispatches)

Each is one substantive crate going from `0.0.x` placeholder to
`0.1.0` substantive implementation. None of these touch the
crypto path; the connector-service framework already validates
tokens and decrypts payloads — implementations only need to
implement the `Implementation` trait.

- **D7** `philharmonic-connector-impl-email-smtp` (Tier 2).
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

Independent of one another and of section A; safe to run in
parallel.
```

(D7 was a one-line stub because `email_send` wire-shape was
flagged "pending detailed wire-protocol shaping" in
[`docs/design/08-connector-architecture.md`](../design/08-connector-architecture.md)
§Open questions. 2026-05-12: HUMANS.md surfaced a complete
spec; design/08 §SMTP was expanded to lock it in and the
open-question note was removed. The post-rewrite ROADMAP §3.B
D7 entry summarises the connection policy + TLS strictness +
request shape and points at design/08 §SMTP as the
authoritative spec.)

## §3.E (pre-rewrite) — D17 in design-landed-but-not-dispatched state

```
### E. Execution-substrate runtime semantics (1 dispatch)

- **D17** `mechanics-core` response-detached background-poll
  runtime. Today
  [`mechanics-core/src/internal/runtime.rs:268-275`](../mechanics-core/src/internal/runtime.rs#L268-L275)
  unconditionally drives `ctx.run_jobs()` after invoking the
  module's default export, and the executor loop at
  [`mechanics-core/src/internal/executor.rs:209-302`](../mechanics-core/src/internal/executor.rs#L209-L302)
  drains async-job / promise / timeout / generic queues to
  full quiescence before returning. Net effect: an unawaited
  `mechanics:endpoint(...)`, a live `setTimeout`, or any
  fire-and-forget `Promise.then(...)` chain pins the worker
  response open until the per-job `max_execution_time`
  deadline trips. The script's `return` is therefore not the
  step-completion fence — quiescence is.

  Target shape: as soon as the top-level resolves (sync
  return or awaited promise fulfilled), the worker
  serialises the result and returns the run-job response.
  Pending promises continue to be polled in the background
  until they settle or the per-job deadline expires — their
  side effects (real HTTP calls to connectors, real timer
  callbacks) still complete, but no longer hold the
  response open.

  Design choices (settled 2026-05-12; authoritative subsection
  at [`docs/design/06-execution-substrate.md` §Tail-promise
  polling](design/06-execution-substrate.md#tail-promise-polling)):

  - **Background-poll lifetime**: tail-poll shares the per-job
    `max_execution_time` budget with main; runs on the same
    worker tokio task that started the job; no separate pool.
  - **Audit-trail handling**: fire-and-forget. Tail-promise
    outcomes are not recorded in the workflow step record and do
    not surface in the audit log.
  - **Unhandled-rejection handling**: increment whatever
    external rejection telemetry exists; today nothing external
    is wired, so the internal `pending_unhandled_rejections`
    counter (see `RuntimeHostHooks`) is the only sink. No
    response failure on tail rejection — mirrors the 0.4.0
    "trust the script's try/catch" stance.
  - **Deadline mid-tail-poll**: drop realm + in-flight futures,
    emit one `tracing::warn!` naming the job ID and the
    in-flight + queued counts at abort time.
  - **Realm + context lifetime**: realm stays alive on the
    per-job tokio task until tail-poll exits (quiescence or
    deadline); dropped at exit.
  - **Backpressure / quota**: no new cap. The existing
    worker-pool slot limits self-regulate, since tail-poll
    occupies the worker's tokio slot it ran on.
  - **Opt-in vs. default**: hardcoded default. No per-job knob.

  Claude drafts the Codex prompt; Codex implements + tests. No
  crypto-review gate — runtime control flow only.
```

(D17 landed 2026-05-12: `mechanics-core` 0.4.0 → 0.4.1,
mechanics-core submodule `0e6c3e7` + parent `743e091`.
Sub-shape B chosen: `RunJobsExit { Complete,
DeadlineExceeded(QueueSnapshot) }` enum +
`run_jobs_until(predicate)` helper. `tracing = "0.1"` dep
added; `setTimeout` global builtin added inside the realm
to support the deadline-mid-tail-poll test. Claude
post-processing closed two housekeeping items Codex
would've caught — the pre-existing
`execution_timeout_stops_slow_async_job` test asserting
the old error message and the CHANGELOG entry's
`../docs/ROADMAP.md` + "D17" label leak across the
mechanics-core/Philharmonic boundary. Codex prompt archive
+ outcome detail at
[`docs/codex-prompts/2026-05-12-0001-d17-mechanics-core-tail-promise-polling-01.md`](../codex-prompts/2026-05-12-0001-d17-mechanics-core-tail-promise-polling-01.md).)

## Suggested-sequencing footer (pre-rewrite)

```
### Suggested sequencing

**Completed work (2026-05-02 through 2026-05-11):** D1 +
D2 + D10 → Gate 1 → embedding-datasets feature end-to-end
(D3 r01 → r02 → D4+D5+caps+409 → Gate 2 → D6 WebUI) → D12
→ D13 → D11 (+ JP mirror) → D16 → D14 + D15. Per-step
commit SHAs and per-dispatch shape detail preserved at
[`docs/archive/2026-05-11-roadmap-completed-arc-trim.md`](archive/2026-05-11-roadmap-completed-arc-trim.md).
Additional 2026-05-11 deployment-time polish (not
numbered Codex dispatches — mechanics-core 0.4.0,
philharmonic-api 0.1.8, WebUI permission-aware UI,
assistant `name` field surfacing, connector wire-shape
guide expansion, audit-log producer gap closed)
summarised in the Current state preamble at the top of
this file with the same archive pointer.

**Next dispatchable**: D7 / D8 / D9 (Tier 2/3 connector
implementations — SMTP, Anthropic, Gemini). D9 carries
the dual-mode AI Studio + Vertex AI requirement. All
three are independent and parallel-safe. **D17** (execution-
substrate response-detached background-poll) design has
landed in [`docs/design/06-execution-substrate.md` §Tail-promise
polling](design/06-execution-substrate.md#tail-promise-polling)
(2026-05-12); Codex prompt drafting is the only step left
before dispatch. Sequence independent of D7–D9.
```
