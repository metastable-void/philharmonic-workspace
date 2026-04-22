# Documentation gap audit

**Date:** 2026-04-22
**Prompt:** *(direct in-session request; no archived `docs/codex-prompts/*` file for this report)*

## Scope

Reviewed documentation for cross-file consistency, status drift, and actionable correctness gaps:

- `README.md`, `ROADMAP.md`
- `docs/design/*.md`
- `docs/design/crypto-proposals/*`, `docs/design/crypto-approvals/*`
- `docs/crypto-vectors/wave-a/README.md`, `docs/crypto-vectors/wave-b/README.md`
- `docs/codex-reports/*`, `docs/codex-prompts/*` (metadata hygiene)

No documentation changes were applied in this pass (flag-only audit).

## Findings

### 1) High: canonical design docs drift from approved Wave B security decisions

Several core docs still encode pre-approval or weaker semantics than the approved Wave B proposal.

Evidence:

- `docs/design/08-connector-architecture.md` says router has "No rate limiting (deferred)" (`:228`), while approved Wave B proposal makes service-bin and router rate limits a **normative** mitigation for replay-driven CPU exhaustion (`docs/design/crypto-proposals/2026-04-22-phase-5-wave-b-hybrid-kem-cose-encrypt0.md:171-188`).
- `docs/design/08-connector-architecture.md` token-claim list omits `iat` (`:285-300`), while settled docs include `iat` as part of connector claims (`docs/design/11-security-and-cryptography.md:130-143`; Wave B proposal also references the 0.2.0 `iat` bump at `:87-94`).
- `docs/design/08-connector-architecture.md` request flow is still high-level (`:423-436`) and does not reflect approved strict Step 12a validation and explicit `Enc_structure` AEAD AAD handling (`docs/design/crypto-proposals/2026-04-22-phase-5-wave-b-hybrid-kem-cose-encrypt0.md:461-463`, `:468-492`).
- `docs/design/11-security-and-cryptography.md` still states "hybrid algorithm identifier" for COSE_Encrypt0 (`:213-216`), but the approved decision is `alg = 3` (A256GCM) plus custom text-keyed headers (`docs/design/crypto-proposals/2026-04-22-phase-5-wave-b-hybrid-kem-cose-encrypt0.md:41-42`).

Why this matters: implementers reading canonical design docs can apply an outdated or weakened security contract.

### 2) Medium: top-level design index has status and blocker drift

`docs/design/00-index.md` is inconsistent with newer design-status docs.

Evidence:

- It still marks connector client/router/service and per-impl crates as "Designed, not yet implemented" (`docs/design/00-index.md:107-113`), while both ROADMAP and crate-ownership docs record Wave A functionality as landed in-tree (`ROADMAP.md:60-64`, `docs/design/03-crates-and-ownership.md:54-56`, `:63-64`).
- It lists `http_forward` among unresolved wire-protocol blockers (`docs/design/00-index.md:118-121`), while open-questions doc marks `http_forward` as already settled (`docs/design/14-open-questions.md:49-53`).

Why this matters: `00-index.md` is the first navigation entry-point; stale status there causes planning and onboarding errors.

### 3) Medium: Gate-1-approved Wave B proposal still contains pre-approval sections and internal inconsistency

The proposal status says Gate-1 approved with all eight questions answered, but stale approval-request content remains.

Evidence:

- Status/revision section says all open questions are answered (`docs/design/crypto-proposals/2026-04-22-phase-5-wave-b-hybrid-kem-cose-encrypt0.md:31-43`).
- Same file still includes an "Open questions" section as unresolved (`:815-883`) and a "Requesting Gate-1 approval" section (`:955-980`).
- Internal step-count mismatch remains: "12-step ... 12th ... 13th" in scope (`:81-83`) vs detailed table that runs through step 15 (`:461-463`).

Why this matters: this document is now both historical and normative; mixed state increases review ambiguity.

### 4) Medium: crypto-vector READMEs are stale relative to generated artifacts

Wave A/Wave B vector docs don’t consistently reflect currently committed composition artifacts.

Evidence:

- Wave B README says Wave A×Wave B composition vector is "not yet generated" and still outstanding (`docs/crypto-vectors/wave-b/README.md:103-109`, `:152-160`).
- Wave A generator already emits composition artifacts when Wave B payload hash exists (`docs/crypto-vectors/wave-a/gen_wave_a_vectors.py:236-265`).
- Composition artifacts are present in-tree (`docs/crypto-vectors/wave-a/`: `wave_a_composition_*.hex`).
- Wave A README file table omits the composition files (`docs/crypto-vectors/wave-a/README.md:45-57`).

Why this matters: vector reproducibility and provenance are security-sensitive; stale vector docs reduce trust in test artifacts.

### 5) Low: README onboarding path points to a non-existent location

`README.md` says to start with `docs/01-project-overview.md` (`README.md:24`), but the design docs are under `docs/design/`.

Why this matters: first-time readers hit a dead path immediately.

### 6) Low: documentation metadata hygiene nits

Evidence:

- Malformed markdown link (extra closing parenthesis) in `docs/codex-prompts/2026-04-22-0001-philharmonic-policy-auth-boundary-hardening.md:12`.
- Two Codex reports claim their prompt file was "not present in workspace" even though the referenced prompt files now exist (`docs/codex-reports/2026-04-22-0003-phase-5-wave-a-cose-sign1-tokens-security-review.md:4`, `docs/codex-reports/2026-04-22-0005-phase-5-wave-b-hybrid-kem-cose-encrypt0-security-review.md:4`).

Why this matters: low risk, but it degrades confidence in doc provenance over time.

## Suggested triage order

1. Reconcile canonical security docs (`08`, `11`, and related sections in `14`) with approved Wave B semantics.
2. Refresh `00-index.md` status/blocker sections to match current roadmap and settled decisions.
3. Clean Wave B proposal post-approval sections (or explicitly split historical appendix vs active spec).
4. Update vector READMEs to match current generated artifact set.
5. Fix onboarding/metadata hygiene nits.
