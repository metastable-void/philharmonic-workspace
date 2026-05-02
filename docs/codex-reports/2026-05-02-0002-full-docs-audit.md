# Full Documentation Audit

**Date:** 2026-05-02
**Prompt:** Inline user request, 2026-05-02: "please do a full audit of any other docs (all, I'm not joking) for mistakes, uncertainties, bugs, etc."
**Related report:** `docs/codex-reports/2026-05-02-0001-embedding-datasets-design-audit.md`

## Scope

I audited the workspace-authored Markdown/documentation surfaces that are
available in this checkout:

- Top-level authoritative docs: `README.md`, `CONTRIBUTING.md`,
  `CLAUDE.md`, `AGENTS.md`, and `HUMANS.md` for context only.
- Main docs tree: `docs/README.md`, `docs/SUMMARY.md`,
  `docs/ROADMAP.md`, `docs/design/*.md`, `docs/guide/*.md`,
  `docs/crypto/**`, `docs/project-status-reports/**`,
  `docs/notes-to-humans/**`, `docs/codex-prompts/**`, and existing
  `docs/codex-reports/**`.
- Crate-level docs: `README.md` / `CHANGELOG.md` files under the
  workspace crates and fixture directories.
- Japanese docs submodule: `docs-jp/*.md`, enough to catch drift that
  duplicates the English workflow guide issues.

I did not treat historical prompts, notes-to-humans, crypto approvals, or
project-status snapshots as current design authority. They are point-in-time
records; many contain instructions that were correct when written and are
stale now by design. I still flagged broken links or stale path names when
they are likely to trip future readers.

I excluded generated/vendor/build trees such as `philharmonic/webui/node_modules`
and `target*`.

## Summary

The docs are broadly useful, but there are several live-doc problems worth
fixing before relying on them as current implementation guidance.

The highest-risk issue is a split-brain around `TenantEndpointConfig`:
some design docs say `implementation` is a plaintext content slot and the
encrypted blob is just connector config, while other design docs still say
`realm` and `impl` are inside the encrypted blob and the lowerer is a
byte-identical re-encryptor. Current code has `implementation` as a plaintext
content slot. This is not just wording: it changes substrate privacy,
lowerer behavior, and crypto-bound payload assembly.

The second major issue is user-facing workflow documentation. The English and
Japanese workflow authoring guides contain LLM examples that send an
incomplete `llm_generate` request and then read an OpenAI-native
`choices[0].message` response. The connector architecture specifies a
normalized `{output, stop_reason, usage}` response, and `output_schema` is
required. A user following the guide through the WebUI would likely produce a
workflow that fails or returns a shape they cannot consume.

The third major issue is status/version drift. Several design docs still
record early versions or phase status even though the local crate manifests
have moved on. `docs/design/06-execution-substrate.md` is the most severe:
it still says `mechanics-config` does not exist and extraction is pending.

## High Severity Findings

### 1. `TenantEndpointConfig` storage/encryption model contradicts itself

Affected docs:

- `docs/design/08-connector-architecture.md:29-36` says the
  implementation name is a separate plaintext content slot and is not inside
  the encrypted blob.
- `docs/design/10-api-layer.md:298-308` says `POST /v1/endpoints` accepts
  `{display_name, implementation, config}`, stores `implementation` as a
  plaintext content slot, and encrypts `config`.
- `docs/design/11-security-and-cryptography.md:52-61` and
  `docs/design/11-security-and-cryptography.md:231-268` also describe the
  current plaintext-implementation model.
- Current code matches that model:
  `philharmonic-policy/src/entity.rs:29-33` defines content slots
  `display_name`, `encrypted_config`, and `implementation`.

Contradicting docs:

- `docs/design/09-policy-and-tenancy.md:547-585` omits the
  `implementation` content slot and explicitly says there are no `impl_ref`
  or `realm_ref` cleartext slots because implementation and realm are inside
  the encrypted blob.
- `docs/design/09-policy-and-tenancy.md:587-610` shows the encrypted blob as
  `{realm, impl, config}`.
- `docs/design/09-policy-and-tenancy.md:640-674` says the lowerer decrypts
  `encrypted_config` and re-encrypts the byte-identical blob with no field
  extraction, substitution, synthesis, or reshaping.
- `docs/design/15-v1-scope.md:122-125`, `docs/design/15-v1-scope.md:157-161`,
  and `docs/design/15-v1-scope.md:250-253` repeat the older "whole blob
  includes realm and impl" model.
- `docs/design/14-open-questions.md:257-265` repeats "pure byte forwarding"
  and says only `realm` is inspected.

Why this matters:

- Substrate privacy changes: plaintext `implementation` leaks the kind of
  external service even when credentials remain encrypted.
- Crypto-bound payload semantics change: current lowerer behavior is payload
  assembly from slots and decrypted config, not byte-identical re-encryption.
- The embedding-datasets design in the previous report builds on lowerer
  changes, so this contradiction makes future crypto review harder.

Recommendation:

Pick one model and update all design docs around it. If the current code is
the intended model, update docs 09, 14, and 15 to say:

- `implementation` is plaintext metadata.
- `realm` is selected from deployment/lowerer routing configuration, not
  necessarily admin-submitted encrypted JSON.
- `encrypted_config` is connector-specific config only.
- The lowerer assembles `{realm, impl, config}` before COSE_Encrypt0.

This should be treated as crypto/privacy-adjacent documentation because it
defines what is protected by SCK and by realm KEM.

### 2. Workflow authoring guides have broken LLM examples

Affected docs:

- `docs/guide/workflow-authoring.md:213-223`
- `docs/guide/workflow-authoring.md:450-466`
- `docs-jp/ワークフロー作成ガイド.md:185-198`
- `docs-jp/ワークフロー作成ガイド.md:338-354`

Problems:

- The examples call `llm_openai_compat` with only `model` and `messages`.
  `docs/design/08-connector-architecture.md:840-870` says
  `output_schema` is required and unknown knobs are rejected.
- The chat example reads `response.body.choices[0].message`, which is the
  upstream OpenAI Chat Completions shape. Doc 08 says `llm_generate` returns
  normalized `{output, stop_reason, usage}` at
  `docs/design/08-connector-architecture.md:875-895`.
- The examples risk teaching users to couple workflow code to provider-native
  responses, which the connector abstraction is explicitly meant to hide.

WebUI usability impact:

The WebUI exposes free-form JSON/script inputs. If a user follows the guide,
they will likely paste a script that either fails connector validation or
throws at runtime when `choices` is absent. The WebUI will appear broken even
though the backend is enforcing the documented normalized protocol.

Recommendation:

Replace the examples with a schema-first call:

```javascript
const response = await endpoint("llm", {
  body: {
    model: "default",
    messages,
    output_schema: {
      type: "object",
      properties: { reply: { type: "string" } },
      required: ["reply"],
      additionalProperties: false
    }
  }
});

const reply = response.body.output.reply;
```

If `mechanics:endpoint` wraps the connector response in a `{body, headers,
status, ok}` envelope, state that clearly and keep the connector body shape
separate from the transport wrapper.

### 3. Workflow guide HTTP example is invalid as written

Affected docs:

- `docs/guide/workflow-authoring.md:65-79`
- `docs-jp/ワークフロー作成ガイド.md:75-91`

The example uses `url_template:
"https://api.example.com/v1/{resource}"` but does not include a
`url_param_specs` entry for `resource`. Current validation rejects templates
with missing URL slot specs; see
`mechanics-config/src/endpoint/validate.rs:49-53`.

Recommendation:

Either add:

```json
"url_param_specs": { "resource": {} }
```

or remove the `{resource}` placeholder from the example URL.

### 4. `docs/design/06-execution-substrate.md` is obsolete

Affected lines:

- `docs/design/06-execution-substrate.md:3-8` says
  `mechanics-config` does not exist and extraction is pending.
- `docs/design/06-execution-substrate.md:228-240` still presents the
  extraction as a migration plan.
- `docs/design/06-execution-substrate.md:242-248` says extraction is pending
  and blocks connector-client implementation.

Contradicting current sources:

- `docs/design/15-v1-scope.md:19-21` says `mechanics-config` is published.
- `./scripts/crate-version.sh --all` reports `mechanics-config 0.1.1`,
  `mechanics-core 0.3.1`, and `mechanics 0.4.1`.

Recommendation:

Rewrite doc 06 as current-state documentation. Keep a short historical note if
needed, but remove "doesn't exist yet", "pending", and "migration plan" as live
guidance.

### 5. `docs/design/03-crates-and-ownership.md` is badly stale

Affected lines:

- `docs/design/03-crates-and-ownership.md:3-43` pins many old versions.
  Examples: `philharmonic-types v0.3.4`, `philharmonic-store v0.1.0`,
  `mechanics-config v0.1.0`, `mechanics-core v0.3.0`,
  `philharmonic-policy v0.1.0`, and `philharmonic-workflow v0.1.0`.
- Local versions from `./scripts/crate-version.sh --all` include
  `philharmonic-types 0.3.6`, `philharmonic-store 0.1.2`,
  `mechanics-config 0.1.1`, `mechanics-core 0.3.1`,
  `mechanics 0.4.1`, `philharmonic-policy 0.2.1`, and
  `philharmonic-workflow 0.1.2`.
- `docs/design/03-crates-and-ownership.md:44-66` says connector-client is
  "the lowerer" and connector-service hosts the implementation trait and
  dispatch. Current doc 08 says connector-client is crypto primitives only,
  connector-service does not host the implementation registry, and dispatch
  lives in the deployment binary.
- `docs/design/03-crates-and-ownership.md:125-139` says `philharmonic`,
  `philharmonic-api`, `email-smtp`, `llm-anthropic`, and `llm-gemini` have no
  crates.io presence. Current local versions show `philharmonic 0.2.0`,
  `philharmonic-api 0.1.3`, and the three deferred impl crates at `0.0.1`.
  `./scripts/xtask.sh crates-io-versions -- <crate>` also reported
  `0.0.0` and `0.0.1` for the deferred impl crates.
- `docs/design/03-crates-and-ownership.md:150-223` still labels the
  dependency graph as current/planned in a way that no longer matches the
  workspace.

Recommendation:

Either make this doc a versionless architectural crate-boundary reference, or
update it from the manifest/script output. Avoid exact patch versions in
design docs unless they are part of a historical release note.

## Medium Severity Findings

### 6. MSRV docs say every crate is 1.88, but two manifests require 1.89

Docs:

- `CONTRIBUTING.md:1190-1203` says MSRV is 1.88 and shows
  `rust-version = "1.88"`.
- `README.md:1185-1190` says MSRV is 1.88 and mirrored in each manifest.
- `docs/design/03-crates-and-ownership.md:330-335` says each crate documents
  `rust-version = "1.88"`.
- `docs/ROADMAP.md:233-234` says every crate uses 1.88.

Current manifests:

- `inline-blob/Cargo.toml:5` has `rust-version = "1.89"`.
- `philharmonic-connector-impl-embed/Cargo.toml:5` has
  `rust-version = "1.89"`.

This is either a docs bug or an implementation-policy bug. Given the large
model bundling work, the two 1.89 manifests may be intentional, but the
workspace-wide docs currently make them look non-compliant.

Recommendation:

Record the exception explicitly, or coordinate a workspace MSRV bump in the
authoritative docs.

### 7. Phase/status language conflicts with placeholder releases

Affected docs:

- `README.md:166-168` says unpublished `philharmonic-connector-impl-*`
  crates have no crates.io presence and no `0.0.0` placeholders were
  reserved.
- `README.md:181-184` says all 25 crates are published, while remaining
  Phase 7 Tier 2/3 connector implementations are deferred.
- `docs/ROADMAP.md:2021-2028` defines v1 as all 25 crates published at
  `0.1.0` or higher, but current local versions have
  `philharmonic-connector-impl-llm-anthropic`,
  `philharmonic-connector-impl-llm-gemini`, and
  `philharmonic-connector-impl-email-smtp` at `0.0.1`.
- `docs/design/00-index.md:128-131` says remaining Tier 3 impls are designed,
  not yet implemented; this is broadly true in the substantive sense, but not
  true in the "no crates.io presence" sense used elsewhere.

Recommendation:

Use two distinct terms:

- "Published placeholder" for `0.0.x` crates with no substantive connector.
- "Published substantive implementation" for `0.1.0+` connector crates.

Then update v1/Phase 9 status to avoid "all 25 crates published at 0.1.0 or
higher" unless the three deferred impl crates are actually promoted.

### 8. `docs/design/15-v1-scope.md` mixes current completion status with an old critical path

Affected lines:

- `docs/design/15-v1-scope.md:5-9` says Phase 9 is complete.
- `docs/design/15-v1-scope.md:431-506` then gives an imperative "Critical v1
  path" that starts with closing remaining design questions, claiming crate
  names, extracting `mechanics-config`, implementing policy, and so on.

This is confusing because the document is both current scope and historical
plan. Some entries in the critical path are now wrong, not just done:

- It says SQL and vector-search wire details are still open at
  `docs/design/15-v1-scope.md:437-441`, but doc 14 says they are settled.
- It says claim remaining crate names as `0.0.0` stubs at
  `docs/design/15-v1-scope.md:454-466`, but that work already happened for
  deferred impls and most listed crates are substantive.
- It says extract `mechanics-config` at `docs/design/15-v1-scope.md:467`.

Recommendation:

Rename the section to "Historical v1 implementation path" and move it below a
current-state summary, or delete it from the live scope doc and rely on
`docs/ROADMAP.md` for history.

### 9. Per-minting-authority rate limiting is internally inconsistent

Docs saying deferred:

- `docs/design/09-policy-and-tenancy.md:523-525`
- `docs/design/14-open-questions.md:194-195`
- `docs/design/14-open-questions.md:335`
- `docs/design/15-v1-scope.md:279-280`

Docs/code saying implemented or intended:

- `docs/design/10-api-layer.md:472-474` says minting endpoint rate limits
  apply per minting authority.
- `philharmonic-api/src/middleware/rate_limit.rs:133-149` includes
  `minting_authority` in the rate-limit key.
- `philharmonic-api/src/middleware/rate_limit.rs:250-263` populates that key
  for minting requests.

Recommendation:

Update docs 09, 14, and 15 if the code is intended. If not intended, the code
needs review. From a docs-only perspective, the current live documentation
cannot tell an operator what the v1 abuse guardrail actually is.

### 10. API docs and permission mapping omit live routes

Current route table includes:

- `/v1/whoami` at `philharmonic-api/src/routes/mod.rs:38-42` and
  `philharmonic-api/src/routes/whoami.rs:8-23`.
- Operator tenant routes at
  `philharmonic-api/src/routes/operator.rs:42-57`.

Docs:

- `docs/design/10-api-layer.md:222-242` lists meta endpoints but not
  `/v1/whoami`.
- `docs/design/10-api-layer.md:443-453` says operator tenant management lives
  wherever the deployment designates, but does not list
  `/v1/operator/tenants`, `/v1/operator/tenants/{id}/suspend`, or
  `/v1/operator/tenants/{id}/unsuspend`.
- `docs/design/09-policy-and-tenancy.md:325-365` has a permission-to-endpoint
  table but omits `/v1/whoami`, operator routes, and
  `PATCH /v1/workflows/templates/{id}` from
  `docs/design/10-api-layer.md:263-268`.

Recommendation:

Update doc 10's endpoint surface and doc 09's mapping table. For public/meta
or identity routes, explicitly mark "no permission, authenticated only" or
"public" rather than omitting them.

### 11. Link rot from wrong relative paths

Representative broken or suspicious links:

- `README.md:29-30` says to start with `docs/01-project-overview.md`, but the
  file is `docs/design/01-project-overview.md`.
- `AGENTS.md:232` links `POSIX_CHECKLIST.md`, but the file is
  `docs/POSIX_CHECKLIST.md`.
- `docs/ROADMAP.md` contains many links written as if the file lived at repo
  root:
  - `docs/ROADMAP.md:167` links `CONTRIBUTING.md` instead of
    `../CONTRIBUTING.md`.
  - `docs/ROADMAP.md:1191`, `1320`, `1322`, `1408`, `1506`, `1514`,
    `1616`, and `1618` link `docs/...` from inside `docs/`, which resolves
    as `docs/docs/...`.
  - `docs/ROADMAP.md:1340` links `inline-blob/README.md` from inside
    `docs/`, which resolves as `docs/inline-blob/README.md`.
  - `docs/ROADMAP.md:1511` links `.claude/skills/...` from inside `docs/`,
    which resolves as `docs/.claude/...`.
- `docs/crypto/proposals/2026-04-28-phase-8-ephemeral-api-token-primitives.md`
  links `../09-policy-and-tenancy.md` and `../11-security-and-cryptography.md`
  at lines 107, 262, and 360. From `docs/crypto/proposals`, those resolve
  under `docs/crypto/`, not `docs/design/`.
- `tests/fixtures/upstream/vllm/README.md:8` and
  `tests/fixtures/upstream/openai-chat/README.md:7` link
  `../../../ROADMAP.md`, which resolves to `tests/ROADMAP.md`, not
  `docs/ROADMAP.md`.

Recommendation:

Run or add a lightweight local Markdown link checker that ignores archival
external URLs but validates repository-relative links. This would catch a
large fraction of the navigational drift.

### 12. Fixture documentation still points to `docs/upstream-fixtures`

Current fixture location is `tests/fixtures/upstream/...`.

Stale references:

- `tests/fixtures/upstream/openai-chat/README.md:150-160` re-capture commands
  use `docs/upstream-fixtures/openai-chat/...`.
- `docs/ROADMAP.md:1191` and `docs/ROADMAP.md:1248` reference
  `docs/upstream-fixtures/...`.
- Historical prompts and notes under `docs/codex-prompts/` and
  `docs/notes-to-humans/` contain many `docs/upstream-fixtures/...`
  references. Those may be archival and should not necessarily be rewritten,
  but they are a trap if copied into new work.
- `philharmonic-connector-impl-llm-openai-compat/tests/fixtures/README.md`
  also says provenance fixtures live under `<workspace-root>/docs/upstream-fixtures/`.

Recommendation:

Update live fixture READMEs and any current roadmap references. Leave archived
prompts alone unless they are being used as templates for new tasks.

### 13. `docs/design/12-deferred-decisions.md` names a nonexistent entity

`docs/design/12-deferred-decisions.md:351-354` says the `key_version` scalar
on `TenantCredential` generalizes to tenant-specific keys. There is no
current `TenantCredential` entity; the relevant entity is
`TenantEndpointConfig`.

Recommendation:

Replace `TenantCredential` with `TenantEndpointConfig`.

## Lower Severity / Cleanup Findings

### 14. `docs/design/00-index.md` has stale status phrasing

Examples:

- `docs/design/00-index.md:35-38` calls `connector-client` the lowerer.
  Current doc 08 says the full lowering orchestration lives in the API server
  binary and connector-client is crypto/minting primitives.
- `docs/design/00-index.md:110-120` says "Implemented and published (as of
  2026-04-24)" for a connector subset, then a later block covers Phase 8-9.
  This is not wrong if read as history, but it is easy to misread as current
  status.
- `docs/design/00-index.md:125-131` says "Phase 7 Tier 1-2 connector impls
  shipped" but lists `sql-postgres`, `sql-mysql`, `embed`, and
  `vector-search`; elsewhere email SMTP is Tier 2 and still deferred.

Recommendation:

Make the index mostly versionless and point to `README.md` / `ROADMAP.md` for
status, or refresh all status blocks in one pass.

### 15. `docs/design/04-cornerstone-vocabulary.md` and `docs/design/07-workflow-orchestration.md` pin old versions

Examples:

- `docs/design/04-cornerstone-vocabulary.md:3-4` says
  `philharmonic-types` is currently v0.3.4; local version is 0.3.6.
- `docs/design/07-workflow-orchestration.md:3-5` and
  `docs/design/07-workflow-orchestration.md:472-475` say
  `philharmonic-workflow` 0.1.0; local version is 0.1.2.

Recommendation:

Avoid exact versions in component design docs unless the exact version is the
subject. Use "published" and leave version lookup to `./scripts/crate-version.sh
--all`, changelogs, or release notes.

### 16. Roadmap "six layers" section lists seven layers

`docs/ROADMAP.md:108-123` says "Six layers" and then enumerates 1 through 7,
including API as the seventh layer.

Recommendation:

Change "Six layers" to "Seven layers" or fold API into the workflow/API tier
if that was the intended wording.

### 17. `docs/README.md` says "25 Rust crates, three deployment binaries" without explaining placeholders

`docs/README.md:3-5` is concise, but given the current placeholder/substantive
split it may imply all 25 crates are production-ready. This is lower priority
because the public docs intro is intentionally short.

Recommendation:

Consider "25 crate names, including deferred placeholder connector crates" if
the book is meant for technical readers. For marketing readers, leave it alone
and keep the nuance in `README.md`.

### 18. Japanese workflow guide mirrors the English guide bugs

`docs-jp/ワークフロー作成ガイド.md` repeats the invalid HTTP placeholder
example and the OpenAI-native LLM response consumption described above.

This matters because `docs-jp/2026-05-02-開発サマリー.md:40-49` says the WebUI
has been expanded and is usable for operations. A Japanese reader following
the guide into the WebUI would hit the same bad examples.

Recommendation:

Update the Japanese guide in the same pass as the English guide. Because
`docs-jp` is a submodule and generated by Claude Code per its README, Codex
should not silently rewrite it unless explicitly assigned that docs-jp update.

## Historical / Archival Areas

### Codex prompts and notes-to-humans

The prompt archive and notes contain many references that are obsolete by
current implementation state. That is expected. Examples include old
`docs/upstream-fixtures/...` fixture locations and implementation instructions
that were later revised.

Recommendation:

Do not bulk-edit historical prompts/notes. Instead:

- Fix links only in current docs and current fixture READMEs.
- When an old prompt is used as a template, copy it into a new prompt and
  update paths there.
- Add a short warning to any index that these are historical records, not live
  implementation instructions, if future agents keep copying stale passages.

### Project status reports

`docs/project-status-reports/README.md:27-40` correctly says committed reports
should not be edited and are not authoritative. I did not flag stale claims
inside individual status snapshots for that reason.

## Suggested Fix Order

1. Resolve the `TenantEndpointConfig` model across docs 08, 09, 10, 11, 14,
   and 15. Treat this as crypto/privacy-adjacent.
2. Fix the workflow authoring guides, especially the LLM examples and invalid
   HTTP placeholder example. This directly affects WebUI users.
3. Rewrite `docs/design/06-execution-substrate.md` from pending-plan to
   current-state doc.
4. Refresh or simplify `docs/design/03-crates-and-ownership.md`.
5. Decide and document the MSRV 1.89 exceptions or bump policy.
6. Add a link-checking maintenance pass for current docs.
7. Update API endpoint and permission tables for `/v1/whoami`, operator
   tenant routes, and `PATCH /v1/workflows/templates/{id}`.
8. Clean up version pins in component design docs.

## Verification Notes

Commands used during the audit included:

- `uname -s` -> `Linux`.
- `./scripts/xtask.sh calendar-jp` -> JST 2026-05-02 (Sat), out-of-hours
  weekend context.
- Markdown inventory via `find`, excluding generated/vendor/build trees.
- Text scans with `rg` for status, TODO-like wording, path/link patterns,
  permission atoms, endpoint routes, and version pins.
- `./scripts/crate-version.sh --all` for local crate versions.
- `./scripts/xtask.sh crates-io-versions -- ...` for the deferred connector
  impl crate names and `philharmonic-api`.

No Rust files were edited for this report.
