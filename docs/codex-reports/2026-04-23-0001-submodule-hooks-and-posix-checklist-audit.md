# Submodule hooks and POSIX checklist audit

**Date:** 2026-04-23
**Prompt:** *(direct in-session request; no archived `docs/codex-prompts/*` file for this report)*

## Scope

This report captures two review tracks requested during this session:

1. Review of the new hook-enforcement behavior introduced in `scripts/setup.sh` for submodules.
2. Factual audit of `POSIX_CHECKLIST.md` against POSIX.1-2024 primary references, with web verification where needed.

## 1) Submodule hook enforcement review

### Files reviewed

- `scripts/setup.sh`
- `scripts/lib/relpath.sh`
- `.githooks/pre-commit`

### Result

The revised mitigation in `scripts/setup.sh` is correct for the previously identified risk.

- Previous risk: sourcing `scripts/lib/relpath.sh` from each submodule's immediate superproject could fail for nested submodules.
- Mitigation now in place: `REPO_ROOT` is captured once at workspace root, exported, and then used inside `git submodule foreach --recursive` to source `"$REPO_ROOT/scripts/lib/relpath.sh"` and compute a relative `core.hooksPath`.
- This removes dependency on the nested submodule's immediate parent layout and makes lookup stable across recursive traversal.

### Validation performed

- POSIX parse checks:
  - `dash -n scripts/setup.sh scripts/lib/relpath.sh .githooks/pre-commit`
  - `./scripts/test-scripts.sh`
- Runtime sanity over current submodules:
  - Computed hook path for each submodule resolves to the workspace `.githooks` directory.
- No syntax or behavior regressions were found in the current workspace topology.

### Residual gap

There are no nested submodules in this workspace today, so the nested case was validated by path-logic inspection rather than an in-tree end-to-end nested fixture.

## 2) POSIX_CHECKLIST factual audit

### Method

- Audited `POSIX_CHECKLIST.md` line-by-line.
- Verified claims against Issue 8 pages from `pubs.opengroup.org` (fetched via `scripts/web-fetch.sh`).
- Used additional web verification for GNU/BSD `date -r` behavior where the checklist statement mixed implementation families.

### Inaccuracies found

1. `POSIX_CHECKLIST.md:128` (`find -iname`) is incorrect.
   - `find -iname` is standardized in POSIX.1-2024 (Issue 8).

2. `POSIX_CHECKLIST.md:170` (`xargs -r`) is incorrect.
   - `xargs -r` and `xargs -0` are standardized in Issue 8.

3. `POSIX_CHECKLIST.md:236` (`timeout`) is incorrect.
   - `timeout` is a POSIX utility in Issue 8.

4. `POSIX_CHECKLIST.md:206` (`tail -r`) is incorrect.
   - `tail -r` is standardized in Issue 8.

5. `POSIX_CHECKLIST.md:141` and `POSIX_CHECKLIST.md:319` (`grep -o`) are incorrect.
   - POSIX Issue 8 `grep` does not include a `-o` option.

6. `POSIX_CHECKLIST.md:225` (`uuencode`/`uudecode` removed) is incorrect.
   - Both utilities are present in Issue 8.

7. `POSIX_CHECKLIST.md:314` (`select` is POSIX shell) is incorrect.
   - In POSIX shell language, `select` may be recognized by some implementations with unspecified results; it is not a required shell construct.

8. `POSIX_CHECKLIST.md:262` (`date -r N` described as GNU epoch form) is incorrect.
   - GNU `date -r` is `--reference=FILE`.
   - BSD-family `date -r seconds` is the epoch-seconds form.

### Source set used

- POSIX Issue 8 pages:
  - `utilities/V3_chap02.html` (shell command language)
  - `utilities/find.html`
  - `utilities/xargs.html`
  - `utilities/timeout.html`
  - `utilities/tail.html`
  - `utilities/grep.html`
  - `utilities/uuencode.html`
  - `utilities/uudecode.html`
  - `utilities/date.html`
  - `functions/strftime.html` (for `%s` conversion context)
- Additional implementation references:
  - GNU Coreutils `date` options manual
  - OpenBSD `date(1)` manual

## Recommended follow-up

1. Patch `POSIX_CHECKLIST.md` to correct the eight statements above.
2. Keep the checklist explicit about Issue 8 additions to reduce drift against older assumptions.
3. Optionally add one short note in the checklist that flags GNU/BSD behavioral splits separately from POSIX status, to avoid category mixing.
