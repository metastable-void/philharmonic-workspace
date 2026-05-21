# WebUI modal and duplicate-template follow-up
**Date:** 2026-05-21
**Prompt:** `docs/codex-prompts/2026-05-21-0001-webui-modal-component-and-duplicate-template-button-02.md`

Round 02's data-router conversion landed as specified: `App.tsx`
now defines a module-level `createBrowserRouter` tree, with a local
`RootLayout` rendering `VersionRefresh` plus `Outlet`. No route
consumer required changes beyond the prompt's planned
`useUnsavedChanges` hook adoption.

The only non-obvious verification wrinkle was script shape drift:
the prompt's mandatory command says `./scripts/webui-build.sh`, but
the current script exits unless `--production` is passed. The bare
command returned the usage error before any build work; rerunning
`./scripts/webui-build.sh --production` succeeded and regenerated
the committed WebUI artifacts.
