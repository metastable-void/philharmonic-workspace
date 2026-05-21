# philharmonic-chat bin: EN+JA i18n migration (round 02)

**Date:** 2026-05-21
**Slug:** `philharmonic-chat-bin-backend-and-frontend`
**Round:** 02 (i18n migration on top of round 01's chat frontend)
**Subagent:** dispatched via `codex-companion.mjs task --background --write --effort high`

## Motivation

Round 01 (commits `64a1fe6` → `562fd51` → `5a4f924`) shipped
the chat bin's backend body and the React+Redux+Webpack
frontend with hardcoded English strings. Yuka's directive
on 2026-05-21: "chat UI must support EN+JA i18n same as the
admin Web UI". This round wires the existing chat frontend
through the same translation surface that
`philharmonic/webui/` already uses.

One scope tweak vs. the WebUI: persist the chosen locale in
`localStorage` under `philharmonic.chat.locale`, not in
`sessionStorage`. The chat bin's other browser-side state
(`agent_token`, `agent_name`, `ephemeral_<UUID>_token`,
`seen_chat_uuids`) already lives in `localStorage`, so the
i18n state joining it unifies the persistence surface.
Confirmed by Yuka via `AskUserQuestion` answer
"Local-storage instead of session".

## References

- `bins/philharmonic-chat/README.md` — chat bin design.
- `philharmonic/webui/src/i18n/{en.ts,ja.ts,index.ts}` —
  translation structure exemplar.
- `philharmonic/webui/src/store/i18nSlice.ts` — Redux
  slice + storage persistence pattern.
- `philharmonic/webui/src/hooks/useT.ts` — typed hook.
- `philharmonic/webui/src/components/Layout.tsx` —
  language-switcher UI placement reference.
- Round 01 archive:
  `docs/codex-prompts/2026-05-21-0002-philharmonic-chat-bin-backend-and-frontend-01.md`.
- Round 01 report:
  `docs/codex-reports/2026-05-21-0002-philharmonic-chat-bin-backend-and-frontend.md`.

## Context files pointed at

- Frontend sources under
  `bins/philharmonic-chat/frontend/src/` (pages,
  components, store, hooks).
- The chat bin's `dist/` is rebuilt by this round.
- The webui's i18n machinery (read-only reference).

## Constraints baked into the prompt

- **Codex never commits.** Leave changes dirty in the
  working tree. Claude reviews + commits via
  `scripts/commit-all.sh`. Lesson learned from round 01
  where I incorrectly told Codex to use `commit-all.sh`.
- **Scope discipline**: only
  `bins/philharmonic-chat/` (incl. its `frontend/` +
  `dist/`). No edits to other crates or to existing
  workspace scripts.
- **Pre-landing once** at the end, not in a loop.
- **JA authority is Codex.** Yuka has delegated the
  Japanese translations; Codex ships natural everyday-
  business JA without flagging term-by-term for review.

## Open design calls Codex will make

- Translation-key grouping shape. Suggested in the prompt
  but Codex has discretion to pick whatever reads clean.
- Language-switcher placement inside `BrandHeader` (and
  whether to show it pre-sign-in).
- Whether to add new CSS for the switcher chrome.

## Outcome

Completed in one round (task `task-mpfcqelk-4ic6ep`). Codex
shipped the full i18n machinery as dirty changes — **did not
commit** this time, correctly following the round-02 prompt's
"Codex never commits" rule (lesson learned from round 01).

Files Codex touched (all inside `bins/philharmonic-chat/`):

- New: `frontend/src/i18n/{en.ts,ja.ts,index.ts}` with
  `Translations` type derived from the `as const` literal via
  `WidenTranslations`, matching the WebUI's shape.
- New: `frontend/src/store/i18nSlice.ts` — slice with
  `setLocale`, persists to `localStorage` under
  `philharmonic.chat.locale` (the scope tweak Yuka asked for).
- New: `frontend/src/hooks/useT.ts` — typed hook reading
  `state.i18n.locale`.
- Modified: `frontend/src/store/index.ts` (register
  `i18nReducer` under `i18n`).
- Modified: every chat page + component listed in the prompt
  (`App`, `SignIn`, `Awaiting`, `ChatTranscript`, `ChatPanel`,
  `BrandHeader`, `AgentNamePrompt`, `VersionRefresh`) —
  hardcoded English replaced with `useT()` keys.
- Modified: `frontend/src/app.css` — language-switcher styling
  added near the brand-header rules.
- Modified: `dist/{main.css,main.css.map,main.js,main.js.map}`
  — webpack rebuild.
- Modified: `README.md` — added §Internationalisation
  (EN+JA, localStorage key, language-switcher location,
  detection fallback) and the `philharmonic.chat.locale`
  storage entry.

Japanese translations Codex picked are natural everyday-
business Japanese (担当者 for "agent", サインイン /
サインアウト, 対応待ち for "Awaiting", 模擬テスト for
"Mock test", 担当者トランスクリプト, 表示言語 for
"Language", 顧客 / アシスタント for ChatBubble role
fallbacks, リクエストに失敗しました and friends for errors).
No flagging-for-Yuka-review per the prompt's instruction.

Language switcher placement: signed-in section of
`BrandHeader` (no switcher pre-sign-in), mirroring the WebUI
behaviour where the language pref lives inside the layout
chrome.

No codex-report written (smooth round, no design surprises).

Verification per Codex: `philharmonic-chat-build.sh
--production` and `./scripts/pre-landing.sh` both clean.
Claude re-ran pre-landing once more before committing the
combined diff (1m56s) — still clean.

**Bundled side-fix in the same commit**: the chat bin's
`/mint-ephemeral` flow was producing
`instance create failed with API status 403 Forbidden`
because its outbound API calls (`POST /v1/workflows/instances`
with `service_token`, then `POST /v1/tokens/mint` with
`minting_token`) didn't send `X-Tenant-Id`. The agent UI's
polling had the same shape and was fixed earlier today at
`6b5d422` by calling `/v1/whoami` from the frontend. The
server-side calls have no `whoami` shortcut, so a new
`tenant_id: Uuid` field was added to `[chat]` TOML config and
threaded through to both outbound requests via
`.header("X-Tenant-Id", config.tenant_id.to_string())`. The
operator's existing chat config will need the new key added
before this revision boots cleanly.

---

## Prompt (verbatim)

```
<task>
You are migrating the existing `bins/philharmonic-chat/`
frontend to support English + Japanese i18n, mirroring the
`philharmonic/webui/` machinery already established in this
workspace. Round 02 of the chat bin — round 01 (commit
`64a1fe6` + fix-forward at `562fd51` and `5a4f924`) shipped
the backend body and the React+Redux+Webpack frontend with
hardcoded English strings; this round wires up i18n.

The chat bin's `bins/philharmonic-chat/README.md` is the
canonical design home for that crate — read it before touching
the frontend. The wire contract there is fixed; if anything in
this prompt contradicts it, the README wins.

## Hard scope constraint

**You must not modify any file outside `bins/philharmonic-chat/`
or the workspace-root `Cargo.toml` / `Cargo.lock`.** No edits
to `philharmonic-api`, `philharmonic/webui/`, `philharmonic`,
`mechanics-*`, or any other workspace crate. If you discover
that an existing dep crate lacks something this round needs,
**do not patch the dep crate** — flag the gap in your
codex-report and either work around it locally inside the
chat bin or stop short.

`bins/philharmonic-chat/dist/` will be rebuilt by this round
(webpack output is committed alongside the source per the
crate's convention). Rebuild via
`./scripts/philharmonic-chat-build.sh --production` once;
don't loop.

## Critical Git rule

**Do NOT commit.** Leave every change dirty in the working
tree for Claude to review and commit via
`scripts/commit-all.sh`. Codex never commits to Git in this
workspace — that's a Claude responsibility after review.
Do not invoke `git commit`, `git push`, `git rebase`,
`git reset`, `git add`, `scripts/commit-all.sh`, or
`scripts/push-all.sh`. Read-only Git inspection through
`scripts/status.sh` / `scripts/log.sh` / `scripts/heads.sh`
is fine.

If you find yourself reaching for a commit, stop and let
Claude commit the in-progress diff instead.

## Read these before writing code

Authoritative:

1. `bins/philharmonic-chat/README.md` — chat bin design.
2. `CONTRIBUTING.md` §6 (POSIX-sh scripts), §11
   (pre-landing).

Reference (copy / adapt; do not modify):

- `philharmonic/webui/src/i18n/en.ts` — translation
  structure exemplar.
- `philharmonic/webui/src/i18n/ja.ts` — Japanese
  counterpart.
- `philharmonic/webui/src/i18n/index.ts` — `translations`
  map, `Locale` type, `detectLocale()`.
- `philharmonic/webui/src/store/i18nSlice.ts` — Redux
  slice with `setLocale` action and storage persistence.
- `philharmonic/webui/src/hooks/useT.ts` — typed hook.
- `philharmonic/webui/src/components/Layout.tsx` —
  language-switcher UI (the `<select>` in the sidebar
  footer). Adapt the same affordance into the chat's
  `BrandHeader`.

The chat frontend already mirrors most of the webui's
patterns (Redux Toolkit, store subscribe → persistAuth,
etc.), so the i18n addition lands as a parallel slice.

## What to build

### i18n machinery

Create three new files under
`bins/philharmonic-chat/frontend/src/i18n/`:

- `en.ts` — all chat frontend strings as a typed object
  literal, structured the way `philharmonic/webui/src/i18n/en.ts`
  is (`as const`, derived `Translations` type via
  `WidenTranslations<T>` at the bottom, named export
  `Translations`, default export the object).
- `ja.ts` — same shape, Japanese strings. **You are the
  authority on the Japanese translations for this round;
  Yuka has explicitly delegated this to you and does not
  want to review JA term-by-term.** Pick natural
  everyday-business Japanese phrasing (敬語 where a
  professional support-agent surface would use it, plain
  for in-app labels and buttons) without literal-translation
  English residue. Ship what reads natural; don't pile
  alternatives into the codex-report.
- `index.ts` — exports `Locale`, `Translations`, the
  `translations: Record<Locale, Translations>` map, and
  `detectLocale(): Locale`. Locale detection: read
  `navigator.language?.slice(0, 2)`, return `"ja"` if it's
  `"ja"`, else `"en"`. Mirrors webui exactly.

### Storage difference vs. the webui

- Use **`localStorage`**, not `sessionStorage`.
- Key: **`philharmonic.chat.locale`** (note the namespace
  difference from `philharmonic.webui.locale`).
- Persist on `setLocale`; read on slice init via a
  `storedLocale()` helper that try/catches storage access
  (matches webui's defensive shape).
- Rationale (record in your codex-report if you like): the
  chat bin's other browser-side persistence already lives
  in `localStorage` (`agent_token`, `agent_name`,
  `ephemeral_<UUID>_token`, `seen_chat_uuids`), so the
  i18n state staying in `localStorage` unifies the
  persistence surface for that UI. The webui keeps locale
  in `sessionStorage` for separate reasons; **do not
  change the webui** to match this.

### Redux + hook wiring

- `bins/philharmonic-chat/frontend/src/store/i18nSlice.ts` —
  mirror webui's slice 1:1 except:
  - storage key as above
  - storage = `localStorage` not `sessionStorage`
- `bins/philharmonic-chat/frontend/src/hooks/useT.ts` —
  identical to webui's: reads `state.i18n.locale` via
  `useAppSelector`, returns `translations[locale]`.
- Register `i18nReducer` in
  `bins/philharmonic-chat/frontend/src/store/index.ts`
  (under the existing `auth`, `branding`, `notify`
  reducers). Don't add a persistence subscriber — the slice
  persists itself on `setLocale`, same shape webui uses.

### String migration

Migrate every hardcoded English string in the chat frontend
to the new translation surface. Sweep file by file:

- `App.tsx` — "Loading..." (boot/config-load fallback) →
  `t.common.loading`.
- `pages/SignIn.tsx` — title, field label, button label,
  sign-in error fallback string.
- `pages/Awaiting.tsx` — page title, "Start mock test"
  button, table column headers, "Open" action,
  "New chat <shortId>" toast (the dynamic part stays as
  parameter via a `t.awaiting.toast(shortId)` function),
  error fallback strings ("mock test failed", "poll failed").
- `pages/ChatTranscript.tsx` — title strings for both
  modes ("Agent transcript" / "Mock test"), "Back" button.
- `components/ChatPanel.tsx` — composer placeholders
  ("Reply as support" / "Write as customer"), "Send",
  "Sending...", "No transcript yet" empty cell, role
  labels ("Customer" / "Assistant") for the ChatBubble
  fallback when `message.name` is absent, the
  "request failed" generic-error string.
- `components/BrandHeader.tsx` — "Agent" field label,
  "Sign out" button. Add the language `<select>` switcher
  here (see *Language switcher* below).
- `components/AgentNamePrompt.tsx` — modal title, field
  label, save button.
- `components/VersionRefresh.tsx` — "A new chat UI
  version is available." banner text, "Reload" button.
- `pages/MockTest.tsx` — currently a single
  `export { default }` line. No strings; leave as-is.

Organize the translation keys to mirror the file structure
where it helps readability:

- `t.common.{loading, send, sending, back, save, signOut,
  requestFailed, ...}` — strings used in 2+ places.
- `t.signIn.{title, tokenLabel, submit, failureFallback,
  ...}`
- `t.awaiting.{title, startMockTest, columns: { instance,
  firstSeen }, openAction, toast: (id) => string,
  errors: { mockTest, poll } }`
- `t.transcript.{ agentTitle, mockTitle, empty,
  composer: { agent, customer }, role: { customer,
  assistant } }`
- `t.agentName.{ promptTitle, fieldLabel }`
- `t.version.{ updateAvailable, reload }`
- `t.brand.{ agentLabel }` (the input next to "Agent" in
  BrandHeader)
- `t.language.{ label, english, japanese }`

Pick whichever grouping reads cleanest. Don't be religious
about matching the webui's exact nesting; match the chat's
own page/component layout.

### Language switcher placement

Add a `<select>` in `components/BrandHeader.tsx`, only
shown when signed in (so the SignIn screen also gets
either no switcher or a switcher — your call; defaulting
to "no switcher pre-sign-in" is fine, mirroring the
webui's behaviour where the language pref is exposed
inside the layout chrome rather than on the unauthenticated
login screen). Style with reasonable inline / utility
classes; if you need new CSS in `src/app.css`, add it
near the existing brand-header rules.

### Toast / sound

`util/notificationSound.ts` has no strings; leave
unchanged. The toast text comes from
`t.awaiting.toast(shortId)` rendered into the existing
`.toast` div in `pages/Awaiting.tsx`.

### Tests / verification

The chat frontend has no JS test suite (Round 01 didn't
add one and the prompt doesn't require it here either).
Verification is:

1. `./scripts/philharmonic-chat-build.sh --production` —
   must succeed and emit `dist/` artifacts.
2. `./scripts/pre-landing.sh` — runs lint + workspace
   `cargo test`. Auto-detects modified crates; should
   include `philharmonic-chat`. **Run once at the end**,
   not in a tight loop — Yuka has asked not to burn time
   on repeated pre-landing runs.

If any phase fails, fix the underlying issue (don't add
`#[allow(...)]` or `// eslint-disable` to silence). If you
cannot fix it, flag the blocker in your codex-report and
leave the diff dirty for Claude to inspect.

## Files you'll create / modify (expected)

- New: `bins/philharmonic-chat/frontend/src/i18n/{en.ts,ja.ts,index.ts}`
- New: `bins/philharmonic-chat/frontend/src/store/i18nSlice.ts`
- New: `bins/philharmonic-chat/frontend/src/hooks/useT.ts`
- Modify: `bins/philharmonic-chat/frontend/src/store/index.ts`
  (register `i18nReducer`)
- Modify: every component / page listed in the *String
  migration* section.
- Modify: `bins/philharmonic-chat/dist/main.js` /
  `dist/main.css` / `dist/main.js.map` / `dist/main.css.map`
  (rebuilt by the webpack run).
- Modify: `bins/philharmonic-chat/README.md` — add a
  short "## Internationalisation" section noting EN+JA
  support, the localStorage key, the language switcher
  location, and the lossy detection fallback. One short
  section, not a manual.

## Out of scope for round 02

- Adding new content to the UI (sorting, filtering,
  search, additional pages).
- New API endpoints.
- A separate language-pref-syncing-with-API surface.
- RTL languages / locale-aware formatting beyond what
  `formatTimestamp.ts` already does.
</task>

<default_follow_through_policy>
When you encounter a missing dependency, an unexpected dep
crate API, or an ambiguous requirement, the default policy is:
work around it locally inside `bins/philharmonic-chat/` if
possible, and document the workaround. If you cannot work
around it without modifying another crate, **stop and report**
— do not modify other crates. Routine adjustments (rearranging
local module structure, picking between two equivalent slice
APIs, choosing a translation grouping that reads cleanest)
you decide yourself; novel design calls you flag in the
codex-report.
</default_follow_through_policy>

<completeness_contract>
"Done" means: the i18n machinery is in place; every
hardcoded English string in the listed components is
migrated to a `useT()`-sourced key; both `en.ts` and
`ja.ts` carry the full translation set; the language
switcher works (changing it actually re-renders all
strings); `philharmonic-chat-build.sh --production`
produces fresh `dist/` artifacts; `./scripts/pre-landing.sh`
is green; the diff sits dirty in the working tree (NOT
committed) for Claude to review. Anything left incomplete
is a blocker that goes in the codex-report with a clear
"what's missing and why" note.
</completeness_contract>

<verification_loop>
Run verification **once** at the end:

1. `./scripts/philharmonic-chat-build.sh --production`
2. `./scripts/pre-landing.sh`

If either fails: inspect the failure, make minimal edits,
re-run. Avoid the edit/re-run loop — pre-landing is
slow-by-design. Do not re-run pre-landing after every
small edit; batch fixes.
</verification_loop>

<missing_context_gating>
If you need to look at how a thing works in a workspace
crate you're not allowed to modify, read it via the file
system and replicate the pattern locally. Read freely —
don't stop short asking for permission. Stop short only
when:
- The verification commands fail in a way you can't
  diagnose.
- An ambiguity in the prompt would affect the chat
  bin's contract with the rest of the system.

Translation calls do **not** stop you — pick the natural
Japanese term and ship.
</missing_context_gating>

<action_safety>
Allowed: edit any file inside `bins/philharmonic-chat/`,
edit workspace-root `Cargo.toml` (if a dep needs to be
added — but this round shouldn't need any new Rust deps;
only frontend deps, which live in
`bins/philharmonic-chat/frontend/package.json` not the
workspace root), edit
`bins/philharmonic-chat/frontend/package.json` /
`package-lock.json` if a new TS/React dep is genuinely
needed, write the codex-report under
`docs/codex-reports/`.

Forbidden — file scope: edit any other file under
`philharmonic-api/`, `philharmonic-*`, `mechanics-*`,
`philharmonic/webui/`, `bins/philharmonic-api-server/`,
`bins/philharmonic-connector/`, `bins/mechanics-worker/`,
`inline-blob/`, `xtask/`, `scripts/*` (do not touch the
existing build scripts). If you need behaviour from one
of those crates and find it missing, **stop and report**.

Forbidden — git: `git commit`, `git push`, `git rebase`,
`git reset --hard`, `--amend`, `git add`, `cargo publish`,
`cargo yank`, `--no-verify`, `--no-gpg-sign`,
`scripts/commit-all.sh`, `scripts/push-all.sh`. The
workspace's codex-guard hook will abort
`commit-all.sh` invocations from under a Codex ancestor
process; don't rely on it — write your work as
dirty-tree-leaving by default. Read-only git inspection
via `scripts/status.sh` / `scripts/log.sh` /
`scripts/heads.sh` is fine.

Forbidden — pre-landing loops: don't re-run
`pre-landing.sh` more than once unless the first run
produced a fix that needs verification.
</action_safety>

<structured_output_contract>
When you finish (success or blocker), return a final report
containing:

1. **Summary** — one paragraph: what landed, what didn't.
2. **Touched files** — list of files created/modified
   under `bins/philharmonic-chat/` and any codex-report.
   Anything you modified outside that allowed set is a
   bug — call it out so it can be reverted.
3. **Verification results** — output (or "clean") for:
   - `./scripts/philharmonic-chat-build.sh --production`
   - `./scripts/pre-landing.sh`
4. **Residual risks** — anything Yuka should look at
   first. Do **not** list translation choices for review;
   ship JA confidently. Only include risks orthogonal to
   the translations themselves (slice wiring, build
   output, behaviour changes you noticed mid-migration,
   etc.).
5. **Git state** — confirm the diff sits dirty in the
   working tree (NOT committed). State the branch
   (should be `main`) and that no new commits were
   created.
</structured_output_contract>
```
