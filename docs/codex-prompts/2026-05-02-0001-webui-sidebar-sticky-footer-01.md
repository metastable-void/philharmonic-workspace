# WebUI sidebar sticky footer (initial dispatch)

**Date:** 2026-05-02
**Slug:** `webui-sidebar-sticky-footer`
**Round:** 01 (initial dispatch)
**Subagent:** `codex:codex-rescue`

## Motivation

The sidebar's language switcher, token preview, and logout button
scroll out of view when the nav list is long or the viewport is
short. These controls should stay visible at the bottom of the
sidebar viewport regardless of scroll position.

## References

- `philharmonic/webui/src/components/Layout.tsx` — sidebar markup
- `philharmonic/webui/src/app.css` — sidebar styles

## Context files pointed at

- `philharmonic/webui/src/components/Layout.tsx`
- `philharmonic/webui/src/app.css`

## Outcome

Completed cleanly. Codex modified Layout.tsx (wrapped
language-switcher + session-box in `sidebar-footer` div) and
app.css (added `.sidebar-footer` sticky rule, `overflow-y: auto`
on `.sidebar`, removed `margin-top: auto` from `.session-box`,
mobile override). Production build succeeded. Commit failed in
sandbox (`.git` lock); Claude committed as `3cb0402` (philharmonic
submodule) + `eacab96` (parent).

---

## Prompt (verbatim)

<task>
Make the language-switcher and session-box block in the WebUI
sidebar stick to the bottom of the sidebar viewport regardless
of scroll position.

Current state:
- `philharmonic/webui/src/components/Layout.tsx` has the sidebar
  with: brand → nav → language-switcher → session-box
- `philharmonic/webui/src/app.css` has `.sidebar` as a flex
  column with `gap: 24px`

The language-switcher (`<label class="compact-field
language-switcher">`) and session-box (`<div class="session-box">`)
should be wrapped in a container div (class name: `sidebar-footer`)
that uses `position: sticky; bottom: 0;` to pin to the bottom of
the sidebar scroll area.

Requirements:
1. In Layout.tsx, wrap the language-switcher `<label>` and
   session-box `<div>` in a single `<div className="sidebar-footer">`.
2. In app.css, add `.sidebar-footer` with:
   - `position: sticky`
   - `bottom: 0`
   - `margin-top: auto` (push to bottom when sidebar is short)
   - `background: var(--surface)` (prevent content showing through)
   - `padding-top: 16px` (visual separation from scrolled nav)
   - `display: grid` + `gap: 14px` (replaces the gap the parent
     flex was providing between the two children)
3. The `.sidebar` needs `overflow-y: auto` so the nav scrolls
   while the footer stays pinned.
4. Remove `margin-top: auto` from the existing `.session-box`
   rule (the footer container handles the push-to-bottom now).
5. In the mobile `@media (max-width: 760px)` block, also remove
   `margin-top: 0` from `.session-box` and add `.sidebar-footer`
   with `position: static` (no sticky on mobile — sidebar is
   horizontal).
6. After making changes, run `./scripts/webui-build.sh --production`
   to rebuild the bundle. The build must succeed with all 6
   expected artifacts.

Files to modify:
- `philharmonic/webui/src/components/Layout.tsx`
- `philharmonic/webui/src/app.css`
- `philharmonic/webui/dist/*` (rebuilt by webui-build.sh)

Out of scope:
- No changes to any Rust code.
- No changes to i18n files.
- No changes to any other React component.
</task>

<action_safety>
- Do NOT run `./scripts/push-all.sh` or `cargo publish`.
- Commit via `./scripts/commit-all.sh` only.
- Do NOT modify files outside the listed scope.
</action_safety>

<verification_loop>
After making changes:
1. Run `./scripts/webui-build.sh --production` — must succeed
   with all 6 artifacts (index.html, main.js, main.css, icon.svg,
   main.js.map, main.css.map).
2. Visually inspect the CSS: the `.sidebar-footer` should be
   `position: sticky; bottom: 0` and `.sidebar` should have
   `overflow-y: auto`.
3. Commit via `./scripts/commit-all.sh "fix: make sidebar language switcher + session box sticky at bottom of viewport"`.
</verification_loop>

<structured_output_contract>
Return:
- summary: one sentence
- touched_files: list
- verification: webui-build.sh result
- git_state: commit SHA
</structured_output_contract>
