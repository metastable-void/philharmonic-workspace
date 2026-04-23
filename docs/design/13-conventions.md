# Workspace conventions — moved

**Moved to [`/CONTRIBUTING.md`](../../CONTRIBUTING.md).**

Development conventions belong in the top-level `CONTRIBUTING.md`
rather than the architecture design doc set. `docs/design/`
describes what Philharmonic *is* (storage substrate, policy
model, connector architecture, security / cryptography, etc.);
contributor conventions describe how to develop on it. Keeping
them separate lets the design docs stay reader-focused.

If you followed a link to an anchor like `§Git workflow`,
`§Panics and undefined behavior`, or `§Library crate
boundaries`, the same section exists in `CONTRIBUTING.md` under
matching or near-matching headings. The rough map:

- `§Development environment` → `§2` Development environment.
- `§Licensing` → `§13` Licensing.
- `§Naming`, `§Crate name claims`, `§Versioning` → `§12`
  Versioning and releases (and `§14.1` for the crate-naming
  pattern).
- `§Crate version lookup` → `§5.1` Crate version lookup.
- `§Git workflow` (including `§No history modification`,
  `§Tracked Git hooks`) → `§4` Git workflow.
- `§Shell scripts` → `§6` Shell script rules.
- `§ROADMAP maintenance` → `§16` ROADMAP maintenance.
- `§Journal-like files` → `§15` Journal-like files.
- `§Notes to humans` → `§15.1`.
- `§Codex prompt archive` → `§15.2`.
- `§Codex reports` → `§15.3`.
- `§Edition and MSRV`, `§Build targets`, `§Testing`, `§Miri`,
  `§Async runtime`, `§Re-export discipline`, `§Error types`,
  `§Trait crate vs. implementation crate split`, `§Library
  crate boundaries`, `§Panics and undefined behavior` → `§10`
  Rust code conventions.
- `§Pre-landing checks`, `§CI` → `§11` Pre-landing checks.
- `§Script wrappers`, `§External tool wrappers`, `§In-tree
  workspace tooling (xtask/)`, `§KIND UUID generation` → `§5`,
  `§7`, `§8`, `§9`.
- `§API breakage detection`, `§Release tagging` → `§12.3`,
  `§12.4`.
- `§Naming and terminology` → `§14` Naming and terminology.
- `§Workspace inspirations`, `§When conventions should change`
  → `§17` Conventions-about-conventions.
- `§Repository structure`, `§Documentation` → `§3` Repository
  structure (and the per-crate README convention is part of
  §15 / §1).

Historical journal entries (`docs/codex-prompts/*`,
`docs/codex-reports/*`, `docs/notes-to-humans/*`) that still
reference `docs/design/13-conventions.md §X` are intentionally
left as-is — they're archive records of what was true at the
time of writing, not live pointers.
