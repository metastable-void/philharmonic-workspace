# Entity list sort: optional `?sort=modified_desc` mode, WebUI opts in (round 02)

**Date:** 2026-05-20 (JST)
**Slug:** `entity-list-sort-modified-opt-in`
**Round:** 02 — re-dispatch after round 01 died without doing
any work. Round 01's archive is at
[`2026-05-20-0001-entity-list-sort-modified-opt-in-01.md`](2026-05-20-0001-entity-list-sort-modified-opt-in-01.md);
its full preamble, references, hard constraints, per-file
scope, tests, verification, hand-off rules, and `<task>` block
are **all authoritative for this round**. Read round 01 in
full before starting; this file does not repeat the content.
**Subagent:** `codex:rescue`

## What's different in round 02

Nothing on the implementation contract. The prior dispatch
returned with the working tree clean (no edits, no
`task_complete` event in the rollout). This round is a clean
re-attempt of the same scope, against the same starting tree.

If you ran into something at runtime that prevented edits
last time, surface it in this round's session summary (or
codex-report) so Claude knows what to thread back to Yuka.

## Starting state

`./scripts/status.sh` should print `(clean)` for the parent
repo and every submodule. The only changes to land between
rounds 01 and 02 on origin are:

- `7ad827a` — `archive codex prompt: entity-list-sort-modified-opt-in round 01` (parent)
- This round-02 archive file + the round-01 outcome update,
  to be committed in a follow-up parent-only commit before
  re-dispatch.

If the starting tree is dirty for any other reason, **STOP
and report**.

## Outcome

Implemented the optional `sort=modified_desc` entity-list mode while
leaving API defaults at `created_desc`. Touched
`philharmonic-api/src/{pagination.rs,store.rs,lib.rs,routes/{audit,authorities,embed_datasets,endpoints,identity,memberships,principals,roles,workflows}.rs}`
plus `philharmonic-api/tests/{common/mod.rs,endpoint_config.rs,workflow_endpoints.rs}`;
`philharmonic-store/src/entity.rs`;
`philharmonic-store-sqlx-mysql/src/{entity.rs,schema.rs}` and
`philharmonic-store-sqlx-mysql/tests/integration.rs`; and
`philharmonic/webui/src/api/client.ts`,
`philharmonic/webui/src/pages/{Authorities,AuthorityDetail,EmbedDatasets,Endpoints,Instances,Memberships,Principals,RoleDetail,Roles,Templates}.tsx`,
with `philharmonic/webui/dist/{main.js,main.js.map}` regenerated.
`CursorKey` uses the preferred `sort_key_value` rename plus
`sort_mode`; `CursorWire` keeps the JSON `created_at` field and adds
defaulted `sort`. The three route-local `paginate_items` copies were
consolidated into the canonical helper in
`philharmonic-api/src/pagination.rs`; audit remains event-time sorted
and pins `SortMode::CreatedDesc`. The sqlx-mysql store now implements
one-round-trip `latest_revision_timestamps` via an
`INNER JOIN (SELECT entity_id, MAX(revision_seq) ...)` query, and
`INDEX_MIGRATIONS` now includes
`ALTER TABLE entity_revision ADD INDEX ix_entity_revision_entity_created (entity_id, created_at)`.
Verification passed: `./scripts/pre-landing.sh --verbose` ended with
`=== pre-landing: all checks passed ===` after the default quiet run
hit an existing wrapper argument-order issue in the ignored-test phase;
`./scripts/webui-build.sh --production` exited 0 with the pre-existing
webpack size warnings. Hand-off SHAs: parent `abde64b`,
`philharmonic` `5329489`, `philharmonic-api` `243932b`,
`philharmonic-store` `d426627`,
`philharmonic-store-sqlx-mysql` `e05f0b2`. Residual: parent status
also shows `.github/workflows/ci.yml` and several `scripts/*.sh`
changes plus `scripts/cargo-install.sh` that were not part of the
requested implementation; Claude should review/attribute those before
committing.

**Post-Codex follow-ups bundled into the same batch commit:**

- `scripts/pre-landing.sh` — Claude-side fix for the wrapper
  argument-order issue Codex's outcome calls out. The default
  quiet mode was appending `$quiet_flag` *after* the crate
  positional in the narrowed `rust-test` and `--ignored`
  loops, which `rust-test.sh` rejects (it parses one
  positional and errors on extra args). Moved `$quiet_flag`
  to before the crate name in both spots; the default quiet
  path now works end-to-end without the `--verbose` workaround
  Codex used.
- `.github/workflows/ci.yml`, `scripts/{cargo-audit,cargo-deny,
  check-api-breakage,setup,tokei}.sh`, and the new
  `scripts/cargo-install.sh` — Yuka's parallel work enabling
  `cargo-binstall` workspace-wide. Attributed and bundled
  into the same commit per Yuka's direction.

---

<task>
Re-dispatch of round 01. Read the authoritative prompt in
full:

`/home/ubuntu/philharmonic-workspace/docs/codex-prompts/2026-05-20-0001-entity-list-sort-modified-opt-in-01.md`

The entire preamble (Motivation, References, Hard constraints,
Per-file scope, Shape (locked), Tests, Verification, Hand-off
shape, Codex report) plus the round-01 `<task>` block apply
verbatim to this round. Nothing is changed in the scope or
constraints.

If you encountered a runtime issue last round that prevented
any edits, mention it in your session summary so Claude can
diagnose. Otherwise proceed exactly per round 01's
specification.

Verification gate before declaring done (unchanged from
round 01):

- `./scripts/pre-landing.sh` clean (`=== pre-landing: all
  checks passed ===`).
- `./scripts/webui-build.sh --production` exit 0 (pre-
  existing webpack size warnings OK; no new TS errors).

Update the `## Outcome` section of **this** file (round 02)
before declaring done — not round 01's outcome, which is
already filled in with the "died without doing anything"
record.

Return the structured-output contract from round 01's
`<structured_output_contract>` block. Pay particular
attention to (3) cursor-shape diff, (5) the
latest_revision_timestamps SQL, (7) the paginate_items
consolidation outcome, (10) test coverage by name, (12)
`dist/` regeneration confirmation, and (17) the
`## Outcome` paragraph ready for Claude to paste into
this round-02 archive.
</task>
