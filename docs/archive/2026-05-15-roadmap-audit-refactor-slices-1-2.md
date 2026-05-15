# 2026-05-15 ROADMAP §3.K trim — Audit & refactor slices 1 + 2 landed

Pre-trim verbatim "Slices landed" entries under ROADMAP §3.K
as they stood after slice 2 landed. Trimmed out of the live
ROADMAP because the slices are historical: their authoritative
detail lives in the per-slice `docs/codex-reports/` files.

The live §3.K now carries only forward-looking content
(in-flight status, sub-directives, pending extraction
candidates, the gate on §3.B) plus a one-line pointer to
this archive.

Prior trim archives:
[`2026-05-14-roadmap-d24-done.md`](2026-05-14-roadmap-d24-done.md).

---

## Verbatim §3.K — "Slices landed (2026-05-15)"

- **Slice 1** — server-side bin-thinning: default-serve
  command, missing-config-defaults handling, and raw-or-hex
  key-material parsing extracted to `philharmonic::server`
  (`cli` / `config` / new `key_material` module).
  [Detail](../codex-reports/2026-05-15-0003-audit-refactor-server-helpers.md).
- **Slice 2** — HTTPS+HTTP-3 axum accept loop extracted to
  new `philharmonic::server::https`. `philharmonic::server`
  now feature-gated by `server` / `server-key-material` /
  `server-https`; `server-https` is separate from the
  mechanics-runtime `https` feature.
  [Detail](../codex-reports/2026-05-15-0004-audit-refactor-https-helper.md).

## Per-slice cross-references

- Slice 1 codex-report:
  [`docs/codex-reports/2026-05-15-0003-audit-refactor-server-helpers.md`](../codex-reports/2026-05-15-0003-audit-refactor-server-helpers.md)
- Slice 2 codex-report:
  [`docs/codex-reports/2026-05-15-0004-audit-refactor-https-helper.md`](../codex-reports/2026-05-15-0004-audit-refactor-https-helper.md)
- Combined CHANGELOG entry: `philharmonic` 0.3.4 (unpublished
  at trim time) — "Audit & refactor sweep — bin-thinning,
  slices 1 + 2".

Net bin diff across the two slices: ~−556 / +114 lines
across the three deployment bins (`mechanics-worker`,
`philharmonic-api-server`, `philharmonic-connector`).
Behaviour unchanged.
