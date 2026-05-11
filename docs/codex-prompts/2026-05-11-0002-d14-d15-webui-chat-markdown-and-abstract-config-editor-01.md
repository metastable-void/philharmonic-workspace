# D14 + D15 — chat markdown rendering + `abstract_config` structured editor (initial dispatch, batched)

**Date:** 2026-05-11
**Slug:** `d14-d15-webui-chat-markdown-and-abstract-config-editor`
**Round:** 01 (initial dispatch — D14 + D15, ROADMAP §3.D,
single submodule `philharmonic/webui`, no backend changes,
batched per Yuka's 2026-05-11 dispatch directive)
**Subagent:** `codex:codex-rescue`

## Motivation

Two D-numbered WebUI follow-ups from HUMANS.md
§"Follow-up tasks from 2026-05-10 work", both isolated
to `philharmonic/webui` with no backend implications,
both independent of each other but parallel-safe.
Bundling into one prompt because:

- Both touch the same submodule (one production build,
  one pre-landing pass instead of two).
- D15 reuses the `DataConfigEditor` pattern shipped as
  the D11 follow-up on 2026-05-10 (`f040dce`) — same
  page files (`Templates.tsx`, `TemplateDetail.tsx`),
  same component conventions, same i18n namespace shape.
- D14 lives entirely inside the chat tab on
  `InstanceDetail.tsx`; D15 lives entirely inside the
  template forms. No conflicts.

If D14 hits a blocker (new npm dep surprises, sanitiser
edge cases, bundle-size unexpected) but D15 is clean,
land D15 and report D14 as partial. The reverse is also
fine. **Do not** ship a half-working version of either.

## References

- [`docs/ROADMAP.md` §3.D](../ROADMAP.md#d-webui-infrastructure-features-and-docs-5-dispatches)
  — D14 and D15 entries with the load-bearing rules
  (sanitiser allowlist for D14; structural-editor mirror
  pattern for D15).
- [`HUMANS.md` §"Follow-up tasks from 2026-05-10 work"](../../HUMANS.md)
  — Yuka's original directive listing both tasks.
- D13 prompt + outcome:
  [`docs/codex-prompts/2026-05-10-0007-d13-chat-testing-ui-01.md`](2026-05-10-0007-d13-chat-testing-ui-01.md)
  — built the chat tab D14 extends. The chat-bubble
  render site is around
  [`philharmonic/webui/src/pages/InstanceDetail.tsx:587`](../../philharmonic/webui/src/pages/InstanceDetail.tsx#L587).
- D11 follow-up #3 prompt + outcome:
  [`docs/codex-prompts/2026-05-10-0009-webui-template-data-config-editor-01.md`](2026-05-10-0009-webui-template-data-config-editor-01.md)
  — built `DataConfigEditor.tsx`, which D15 mirrors for
  `abstract_config`.
- Component to mirror:
  [`philharmonic/webui/src/components/DataConfigEditor.tsx`](../../philharmonic/webui/src/components/DataConfigEditor.tsx)
  (315 LOC; D15's new `AbstractConfigEditor.tsx` follows
  the same shape).
- API shape (no backend changes):
  - `POST /v1/workflows/templates` already accepts
    `abstract_config: JsonValue` (a `{<name>: <uuid>}` map).
  - `PATCH /v1/workflows/templates/{id}` already accepts
    `abstract_config: JsonValue` on update.
  - `GET /v1/workflows/templates/{id}` returns
    `abstract_config: JsonValue`.
  - `GET /v1/endpoints` already returns the active-tenant
    endpoint list used for the dropdown.

## Context files pointed at

`philharmonic/webui/src/`:

- `pages/InstanceDetail.tsx` — chat tab; the bubble
  render at line 587 `<div className="chat-bubble
  {message.role}">{message.content}</div>` (or
  equivalent) becomes
  `<MarkdownView className="chat-bubble {message.role}"
  source={message.content} />`. The
  `parseChatOutput`-derived `transcript.messages` array
  is the source of `message.content` — strings, opaque,
  must be treated as untrusted (workflow scripts are
  authored by anyone with `workflow:template_create`).
- `pages/Templates.tsx` — Create form, raw-JSON
  `abstract_config` editor at lines 270-277 (the
  CodeMirror `<CodeEditor>` with `value={configText}`).
  Replace with `<AbstractConfigEditor
  value={abstractConfig} onChange={setAbstractConfig}
  availableEndpoints={...} />`. Drop the `configText`
  state slot in favor of a typed
  `Record<string, string>` (script-name → endpoint UUID).
- `pages/TemplateDetail.tsx` — Edit form, same swap at
  lines 269-277. On template load, normalize
  `template.abstract_config` from `JsonValue` to
  `Record<string, string>` for the editor's initial
  state.
- `api/client.ts` — types are already in place
  (`TemplateDetail` / `CreateTemplateRequest` /
  `UpdateTemplateRequest` carry `abstract_config:
  JsonValue`). Add a `listEndpoints` helper (mirror
  `listEmbeddingDatasets` at line 392). Add a typed
  `AbstractConfig` interface
  (`Record<string, string>`) for the editor surface;
  the wire shape stays `JsonValue`, the editor converts
  at the form's boundary.
- `components/DataConfigEditor.tsx` — **mirror this
  shape** for `AbstractConfigEditor.tsx` (315 LOC
  precedent). Don't share code via abstraction; the
  two editors have different validation, different
  dropdown sources, and different empty-state copy.
  Code reuse via shared CSS is fine.
- `components/MarkdownView.tsx` (new for D14) —
  ~50-80 LOC wrapper that parses markdown source +
  passes through DOMPurify with a strict allowlist,
  then renders the sanitised HTML inside a div with
  `dangerouslySetInnerHTML`. The component takes
  `{ className, source }`.
- `i18n/en.ts` + `i18n/ja.ts` — add
  `templates.abstractConfig.*` namespace mirroring
  `templates.dataConfig.*` shape (D11 follow-up #3).
  No new strings needed for D14 — markdown rendering is
  invisible UX.
- `app.css` — add D14 markdown styles (code blocks
  monospaced; standard
  block-element spacing inside chat bubbles). Reuse
  existing classes where possible.
- `package.json` — D14 adds two npm dependencies (see
  §Hard requirements below). D15 adds no new dependencies.

## Outcome

Pending — will be updated after Codex run.

---

## STRUCTURED-OUTPUT-CONTRACT — READ THIS FIRST

Streak is **9/9** since the contract was added. **Do not
break it.** Six-section report (Summary / Touched files /
Verification results / Residual risks / Git state / Open
questions) with the verbatim `RUN STATUS: COMPLETE`
token, emitted before `task_complete`.

Reminder: D16's previous background task died after
verification finished but before emitting the report,
and a separate resume spawn was needed. If you hit the
context-window edge after verification passes, **prioritise
emitting the report** over any final polish. The contract
is binding even if the code lands cleanly.

---

## Pre-landing-sh hygiene equivalent for the WebUI

Run BEFORE invoking `webui-build.sh --production`:

```sh
cd philharmonic/webui
npx tsc --noEmit
```

Catches TypeScript errors fast (faster than letting webpack
report them at build time). Then run the production build to
regenerate the four committed `dist/` artifacts. Then run
`./scripts/pre-landing.sh` once at the end (the API server
bin embeds the WebUI bundle, so a broken bundle breaks
`cargo build`).

---

## Prompt (verbatim)

<task>
Two D-numbered tasks, batched, both in
`philharmonic/webui`, both independent of each other.

## D14 — markdown rendering in WebUI chat bubbles with DOMPurify hardening

**What**: The chat tab on `InstanceDetail.tsx` currently
renders `message.content` as plain text. Workflow scripts
can produce markdown-formatted assistant turns (lists,
links, code blocks, etc.), so the chat tab should
parse-and-sanitise-then-render. The content is
script-generated and the script is authored by anyone
with `workflow:template_create`, so **the content must
be treated as untrusted**.

**Library choices** (decide; both are acceptable):

- Markdown parser: **`marked`** (~30 KB min, simpler API)
  or **`markdown-it`** (~50 KB min, more configurable).
  `marked` is preferred for the chat use case; both are
  fine.
- HTML sanitiser: **`dompurify`** (~17 KB min+gzip). Not
  optional; this is the security boundary.

**Sanitiser configuration**:

- **Drop**: `<script>`, all inline event handlers
  (`onclick`, `onerror`, etc.), `<iframe>`, `<object>`,
  `<embed>`, `<form>`, `<input>`, `<style>`, `<link>`,
  `<meta>`.
- **URI schemes allowed on `<a href>`**: only `http:` and
  `https:`. Drop `javascript:`, `data:`, `vbscript:`,
  `file:`, any non-http(s) custom scheme. DOMPurify's
  default `ALLOWED_URI_REGEXP` covers most of this;
  prefer the default unless there's a specific reason to
  override.
- **Drop**: `srcset`, `formaction`, `xlink:href`, any
  attribute beginning with `on*`.
- **Keep**: `<p>`, `<br>`, `<strong>`, `<em>`, `<del>`,
  `<code>`, `<pre>`, `<blockquote>`, `<ul>`, `<ol>`,
  `<li>`, `<h1>` through `<h6>`, `<table>`, `<thead>`,
  `<tbody>`, `<tr>`, `<th>`, `<td>`, `<a href>`,
  `<hr>`, `<img>` (only with `src` from http(s) and
  `alt`/`title` strings), and the default DOMPurify
  allowlist for those tags.
- **Link target hardening**: add a post-sanitise pass
  (DOMPurify `afterSanitizeAttributes` hook is the
  canonical place) that for every `<a href>` sets
  `target="_blank"` and `rel="noopener noreferrer
  nofollow"`. Admin testers shouldn't accidentally
  navigate away from the chat tab.

**Code blocks**: render with monospaced styling. **No
syntax highlighting in this dispatch** (highlight.js +
its many languages adds significant bundle weight; defer
to a future follow-up). Plain `<pre><code>` is fine.

**Component shape**:

```tsx
// philharmonic/webui/src/components/MarkdownView.tsx
import { type JSX, useMemo } from "react";
import DOMPurify from "dompurify";
import { marked } from "marked";  // or markdown-it

interface MarkdownViewProps {
  className?: string;
  source: string;
}

export default function MarkdownView({
  className,
  source,
}: MarkdownViewProps): JSX.Element {
  const html = useMemo(() => {
    const raw = marked.parse(source, { /* options */ });
    return DOMPurify.sanitize(raw, {
      ALLOWED_TAGS: [/* see allowlist above */],
      ALLOWED_ATTR: [/* see allowlist above */],
      // ...plus the afterSanitizeAttributes hook for
      // link target hardening
    });
  }, [source]);

  return (
    <div
      className={className}
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
}
```

The `useMemo` is load-bearing: chat-tab re-renders happen
on every send, and re-parsing/re-sanitising every bubble
every render is wasteful. Memoise on `source`.

**Integration**: in `pages/InstanceDetail.tsx`'s chat-tab
bubble render (around line 587), swap

```tsx
<div className={`chat-bubble ${message.role}`}>
  {message.content}
</div>
```

for

```tsx
<MarkdownView
  className={`chat-bubble ${message.role}`}
  source={message.content}
/>
```

Keep the existing role-styling classes (`chat-bubble
user`, `chat-bubble assistant`, etc.) — markdown
rendering is **inside** the bubble, the bubble layout
stays the same.

**CSS** in `app.css`: add styles for the markdown
elements inside chat bubbles (code block monospaced
+ subtle background, list indentation that doesn't
break the bubble layout, link colour, etc.). Don't
add new design tokens — reuse the existing palette.

**Detection unchanged**: `parseChatOutput` in
`api/client.ts` continues to validate the literal
`content: string` shape. Markdown is a rendering
concern, not a wire-format concern. Don't touch the
parser.

**Bundle delta**: the `marked` + `dompurify` pair adds
roughly +45-60 KB gzipped to `main.js`. **Surface the
exact measured delta in the structured-output report's
residual-risks section.** Yuka cares about bundle weight
trends.

## D15 — workflow-template `abstract_config` structured editor

**What**: Both the Create form (in `Templates.tsx`) and
the Edit form (in `TemplateDetail.tsx`) currently
present `abstract_config` as raw JSON in a CodeMirror 6
editor. Replace that with a structured editor —
**mirror the `DataConfigEditor.tsx` pattern shipped as
the D11 follow-up on 2026-05-10**.

**Shape**:

```ts
// In api/client.ts — add this type:
/** Abstract config map: script-side endpoint name → endpoint UUID.
 *  Wire shape is JsonValue; the editor uses the typed view. */
export type AbstractConfig = Record<string, string>;
```

The `TemplateDetail`'s `abstract_config: JsonValue`
field stays as-is on the wire; the editor converts at
the form boundary (similar to how `DataConfigEditor`
handles `TemplateDataConfig`).

```tsx
// philharmonic/webui/src/components/AbstractConfigEditor.tsx
interface AbstractConfigEditorProps {
  value: AbstractConfig;
  onChange: (next: AbstractConfig) => void;
  availableEndpoints: EndpointSummary[];
  disabled?: boolean;
}
```

**Rows**: one per (binding_name, endpoint_id) pair, with:

- **Binding name** text input. Validates client-side
  against `/^[A-Za-z_$][A-Za-z0-9_$]{0,63}$/` (same regex
  as `DataConfigEditor` — these names become the JS
  property the script uses in `endpoint("<name>",
  ...)`). Per-row inline error on invalid name; the
  parent form's submit button is disabled while any row
  is invalid.
- **Endpoint dropdown** populated from
  `props.availableEndpoints` (filtered for
  `is_retired === false` by the parent before passing
  in). Show `display_name` (with `endpoint_id`
  short-hash in parens if multiple endpoints share the
  display name — mirror DataConfigEditor's
  disambiguation). Filter retired endpoints **out** of
  new-row options.
- **For an existing row whose `endpoint_id` references
  a retired or missing endpoint**: keep the row visible
  with a warning badge ("retired" / "missing"). Don't
  silently drop user data — surfacing the warning lets
  the admin re-bind to a different endpoint. Same UX as
  `DataConfigEditor`.
- **Duplicate-name check** client-side: highlight both
  offending rows when two rows share a binding name.
  Server-side duplicates collapse on the JSON object
  (second wins); the editor prevents the confusion
  before submit.

**Empty state** when `value` is `{}`:

```
No endpoint bindings.
Add one to expose an endpoint to this template's script.
[+ Add binding]
```

(via `t.templates.abstractConfig.emptyState` /
`emptyStateHint`.)

**Disabled mode** when `props.disabled` is true: render
read-only. Used while the parent form is saving.

**API helper**: add `listEndpoints` to `api/client.ts`
(mirror `listEmbeddingDatasets` at line 392):

```ts
/** Fetch one page of tenant endpoints. */
export async function listEndpoints(
  cursor: string | null = null,
  limit = 100,
): Promise<PaginatedResponse<EndpointSummary>> {
  return apiCall<PaginatedResponse<EndpointSummary>>(
    `endpoints${queryString({ cursor, limit })}`,
  );
}
```

The `limit = 100` default is enough for the editor's
dropdown for most tenants. Multi-page corpus enumeration
is **out of scope**; for tenants with > 100 endpoints,
the dropdown shows the first page and an "X more not
shown" hint at the bottom — surface this constraint in
residuals if you implement it that way, or alternatively
implement a small auto-loader that walks the cursor
until exhausted (Codex's call; the latter is cleaner but
more code).

**Integration**:

- `Templates.tsx` Create form (around lines 270-277):
  fetch active endpoints once on mount; replace the raw
  JSON CodeMirror editor with `<AbstractConfigEditor>`.
  Initial state `{}`; submit sends
  `abstract_config: <value-as-JsonValue>` (cast the
  `Record<string, string>` directly into
  `JsonValue` at the form boundary).
- `TemplateDetail.tsx` Edit form (around lines 269-277):
  fetch active endpoints once on mount; on template
  load, normalize `template.abstract_config` into
  `Record<string, string>` (it should already be that
  shape on the wire; defensively coerce strings only).
  Replace the raw JSON CodeMirror editor with
  `<AbstractConfigEditor>`. On submit, send
  `abstract_config: <value-as-JsonValue>`.
- **Drop the `configText` state slot** on both pages —
  the raw-JSON editor goes away entirely. Don't leave
  it as a fallback option.

**i18n**: add `templates.abstractConfig.*` mirroring
`templates.dataConfig.*` (en + ja). Cover: section
title, description, empty state, add/remove labels,
binding-name placeholder, endpoint dropdown placeholder,
invalid-name error, duplicate-name error,
retired/missing warning badges,
no-active-endpoints hint.

JP translations should match the surrounding i18n style.
If unsure on a term, leave a `// TODO(jp):` comment and
flag in residuals.

**Permissions**: no new atoms. The editor reuses
`endpoint:read_metadata` (already required to populate
the dropdown) + the existing
`workflow:template_create`. Document this in the
prompt's summary; do NOT add anything to
`components/permissions.ts`.

## Cross-deliverable: build, verification, artifacts

After both D14 and D15 deliverables are in place:

1. **TypeScript typecheck first**:

   ```sh
   cd philharmonic/webui
   npx tsc --noEmit
   ```

   Fix any errors before invoking webpack.

2. **Production build**:

   ```sh
   ./scripts/webui-build.sh --production
   ```

3. **Workspace `pre-landing.sh`**: the API server bin
   compiles the new bundle into the binary.

4. **Run the existing WebUI tests** if any (check
   `philharmonic/webui/package.json` for a `test`
   script; may not exist).

The committed artifacts (post-build) are part of the
diff — `dist/main.js`, `dist/main.css`,
`dist/index.html`, `dist/icon.svg`.

## Cross-deliverable: no version bump on the WebUI

The WebUI lives in `philharmonic/webui` (a sub-tree of
the `philharmonic` meta-crate's submodule, NOT a
published crate). No `Cargo.toml` version bump. The
`package.json` is `private: true` with version
`0.0.0` — leave it alone.

The new `marked` and `dompurify` dependencies bump
`package-lock.json` (and the `dist/` artifacts grow);
that's expected.

<structured_output_contract>
**Critical: emit this report before `task_complete`.**

Six sections, in this order:

1. **Summary** — one paragraph: what landed for D14,
   what landed for D15, which deliverables are
   complete vs partial vs not started. Include the
   verbatim string "RUN STATUS: COMPLETE" or "RUN
   STATUS: PARTIAL — <reason>" for grep.

2. **Touched files** — exhaustive list with
   `(new|edited|deleted) <path> — <one-line note>`.
   Include the four regenerated artifacts in
   `philharmonic/webui/dist/` and `package.json` +
   `package-lock.json` changes.

3. **Verification results** — exact commands + outcomes:
   - `npx tsc --noEmit` (in `philharmonic/webui/`) —
     pass/fail/output excerpt.
   - `./scripts/webui-build.sh --production` —
     pass/fail/exit code.
   - `./scripts/pre-landing.sh` — pass/fail/exit code.
   - `./scripts/test-scripts.sh` — pass/fail (run only
     if you touched any `scripts/*.sh`; you shouldn't
     have).

4. **Residual risks / known issues** — including:
   - **Bundle-size delta** in KB gzipped, broken down
     by D14 (marked + dompurify + new component + CSS)
     vs. D15 (new component + i18n + form integration,
     no new deps).
   - Which markdown parser you chose (`marked` vs.
     `markdown-it`) and why.
   - Sanitiser config divergences from the prompt's
     allowlist (if any) and why.
   - For D15: how the > 100-endpoint case is handled
     (truncated dropdown with hint, or auto-loader that
     walks the cursor). If truncated, the threshold and
     hint copy.
   - Any `any` types you had to use because of
     `JsonValue` widening.
   - Whether the JP translations are placeholders Yuka
     should review.
   - Any test coverage gaps. (No unit tests are
     required for D14/D15; this WebUI has no test
     harness. Note any place you wished one existed.)
   - Whether you preserved the raw-JSON CodeMirror
     editor as a fallback (you shouldn't have — D15
     replaces it entirely).

5. **Git state** — current `HEAD` SHA in
   `philharmonic` submodule. Confirm no commits made.

6. **Open questions** — questions for Yuka or Claude:
   - Syntax highlighting in code blocks (currently
     deferred; mention bundle-weight cost if Codex
     measured it).
   - Whether the `<img>` tag should be allowed at all
     in chat (current allowlist includes it; some
     deployments may prefer to block inline images
     entirely).
   - Whether the > 100-endpoint dropdown case should be
     surfaced as a separate ROADMAP task (D15 follow-up).
   - JP translation quality / terminology choices.
</structured_output_contract>

<default_follow_through_policy>
- **Suggested order**: D15 first (no new deps, smaller
  blast radius, mirrors existing component), then D14
  (new deps + bigger bundle delta + sanitiser config).
  If D14 runs into a blocker after D15 is clean, land
  D15 and report D14 as partial.
- Run `npx tsc --noEmit` after each major surface
  (component drafted, page integrated, i18n added),
  before moving on. WebUI debug loops are slow if you
  let webpack flush the type errors at build time.
- Add JSDoc on all new exported types and functions in
  `api/client.ts` for downstream consumers.
- For D14, run a manual sanity check on the sanitiser
  by including a small inline test that feeds a known
  XSS-attempting input through `MarkdownView` (e.g.
  `<script>alert(1)</script>` + `<a
  href="javascript:alert(1)">x</a>`) and inspects the
  resulting HTML. Include the verification output in
  the residuals section. **Do not commit the test as
  code** if the WebUI has no test harness; just
  document the inputs you tried and the observed
  sanitised output.
- If you discover that an API route's actual response
  shape differs from this prompt's spec, **trust the
  API route** and adapt — flag the discrepancy.
- The editor lives entirely client-side; no backend
  changes for either D14 or D15. If you find yourself
  wanting to add a field to any Rust crate, **stop**
  and surface in residual risks.
</default_follow_through_policy>

<completeness_contract>
"Done" means:

- D14: `MarkdownView.tsx` ships with the sanitiser
  allowlist + link-target hardening; chat tab uses it;
  bundle builds clean; sanitiser tested informally
  against XSS inputs.
- D15: `AbstractConfigEditor.tsx` ships; Templates
  Create form integrates it; TemplateDetail Edit form
  integrates it; raw-JSON CodeMirror editor for
  `abstract_config` removed from both pages;
  `listEndpoints` helper in `api/client.ts`;
  `templates.abstractConfig.*` i18n in en + ja.
- TypeScript typecheck clean.
- `./scripts/webui-build.sh --production` clean.
- `./scripts/pre-landing.sh` clean.
- The four artifacts in `philharmonic/webui/dist/`
  regenerated, plus `package.json` and
  `package-lock.json` if D14 added deps.
- Structured output report emitted before
  `task_complete`.

**Partial completion is acceptable** if one of D14 / D15
runs into a blocker the other doesn't share. In that
case:

- Say so explicitly with "RUN STATUS: PARTIAL —
  <reason>" + which task is incomplete + which is
  complete.
- The complete task's surfaces must all be functional
  (no half-shipped components, no orphaned i18n
  keys, no broken type signatures).
- The incomplete task's source files are either fully
  reverted or guarded behind a "not yet implemented"
  placeholder so the rest of the UI still works.

Half-built UI surfaces are worse than missing surfaces.
A run without the structured-output report is
**incomplete**, even if all surfaces landed.
</completeness_contract>

<verification_loop>
For every surface (component / type / i18n / integration):
1. Implement.
2. `cd philharmonic/webui && npx tsc --noEmit` — green.
3. Move on. Don't run webpack between surfaces (slow).
4. Once both D14 and D15 surfaces are typechecked clean,
   run `./scripts/webui-build.sh --production` once.
5. Run `./scripts/pre-landing.sh` once.
6. Emit the structured output report.
7. Then `task_complete`.

**If you hit the context-window edge after step 5
passes**: prioritise emitting the structured-output
report over any final polish. The contract is binding
even if the code lands cleanly.
</verification_loop>

<missing_context_gating>
If you find yourself needing information not in this
prompt or the cited authoritative docs, **stop** and
report what's missing in the structured output's "Open
questions" section.

Specifically: do **not**:

- Touch any Rust crate. Both D14 and D15 are WebUI-only.
- Add npm dependencies beyond `marked` (or
  `markdown-it`) and `dompurify`. If you genuinely need
  something else (e.g. a CSS-in-JS helper), surface in
  residual risks before adding.
- Edit `webpack.config.js` or `tsconfig.json`.
- Mint new permission atoms or groups. D14 needs none.
  D15 reuses `endpoint:read_metadata` +
  `workflow:template_create`.
- Edit `philharmonic-api`, `philharmonic-policy`,
  `philharmonic-workflow`, or any backend crate.
- Add a new route to `App.tsx`.
- Add transcript persistence to localStorage (HUMANS.md
  D13 spec).
- Add syntax highlighting in chat code blocks
  (deferred; flag in residuals if you measured the
  bundle cost).
- Add raw-JSON fallback editor to D15 (replacement is
  total).
- Edit `HUMANS.md` (agent-readable, agent-writable
  forbidden).
</missing_context_gating>

<action_safety>
Files allowed to be created or modified:

- `philharmonic/webui/src/api/client.ts` (edited — new
  `AbstractConfig` type alias + `listEndpoints` helper).
- `philharmonic/webui/src/pages/InstanceDetail.tsx`
  (edited — chat tab bubble render swaps to
  `MarkdownView`).
- `philharmonic/webui/src/pages/Templates.tsx` (edited
  — Create form integrates `AbstractConfigEditor`,
  drops `configText`).
- `philharmonic/webui/src/pages/TemplateDetail.tsx`
  (edited — Edit form integrates `AbstractConfigEditor`,
  drops `configText`).
- `philharmonic/webui/src/components/MarkdownView.tsx`
  (new — D14).
- `philharmonic/webui/src/components/AbstractConfigEditor.tsx`
  (new — D15).
- `philharmonic/webui/src/i18n/en.ts` (edited —
  `templates.abstractConfig.*`).
- `philharmonic/webui/src/i18n/ja.ts` (edited —
  `templates.abstractConfig.*`).
- `philharmonic/webui/src/i18n/index.ts` (edited only
  if the existing pattern requires per-namespace
  registration).
- `philharmonic/webui/src/app.css` (edited — markdown
  chat-bubble styles for D14).
- `philharmonic/webui/package.json` (edited — D14 adds
  `marked` (or `markdown-it`) + `dompurify` + their
  `@types/*` if available).
- `philharmonic/webui/package-lock.json` (regenerated
  on dep install).
- `philharmonic/webui/dist/index.html`,
  `dist/main.js`, `dist/main.css`, `dist/icon.svg`
  (regenerated by `webui-build.sh --production`).

Files NOT to touch (flag if you find a reason to):

- Any file under `bins/`, `philharmonic-api/`,
  `philharmonic-policy/`, `philharmonic-workflow/`,
  `philharmonic-store*/`, `mechanics-*/`, or any
  connector crate.
- The workspace `Cargo.toml` `[patch.crates-io]` block.
- `philharmonic/webui/webpack.config.js`,
  `philharmonic/webui/tsconfig.json`.
- `philharmonic/webui/src/components/CodeEditor.tsx`
  (D10's CodeMirror wrapper — D15 stops calling it
  for `abstract_config`, but the component itself
  stays for `script_source` / other JSON consumers).
- `philharmonic/webui/src/components/SourceItemsEditor.tsx`
  or `DataConfigEditor.tsx` (extract patterns; don't
  edit).
- `philharmonic/webui/src/components/permissions.ts`
  (no permission changes).
- `philharmonic/webui/src/App.tsx` (no new routes).
- `philharmonic/webui/src/components/Layout.tsx` (no
  new sidebar entries).
- `philharmonic/webui/src/util/chatStorage.ts` (D13
  helper; unrelated).
- `philharmonic/webui/src/pages/EmbedDataset*.tsx`
  (D6's pages; unrelated).
- `HUMANS.md`, `CLAUDE.md`, `AGENTS.md`,
  `CONTRIBUTING.md`, any `.claude/`, `docs/`,
  `docs-jp/`, or `scripts/` content.

Git rules: signed-off + signed commits via
`scripts/commit-all.sh` only. **Codex never runs
`commit-all.sh`** — Claude commits after reviewing your
work. No `cargo publish`. No raw `git commit` /
`git push` / `git add` / `git reset` / `git rebase` /
`git revert`. Read-only `git log` / `git diff` /
`git show` is fine.
</action_safety>
</task>
