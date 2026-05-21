# Philharmonic Chat Bin Backend And Frontend

**Date:** 2026-05-21
**Prompt:** `docs/codex-prompts/2026-05-21-0002-philharmonic-chat-bin-backend-and-frontend-01.md`

Implemented round-01 backend and frontend inside
`bins/philharmonic-chat/`, plus the dedicated frontend build
wrapper `scripts/philharmonic-chat-build.sh`.

Design calls:

- Added `chat.service_token` and used it only for
  `POST {api_url}/v1/workflows/instances`. This keeps the
  configured support-agent token out of the bin-to-API
  service action, matching the prompt's preferred option.
- Used `workflow:instance_read` for minted ephemeral tokens
  instead of the prompt's `workflow:instance_view`. The policy
  crate defines `workflow:instance_read` and has no
  `workflow:instance_view` atom, so `view` would be rejected by
  the mint endpoint's atom validation.
- Kept the API execute wire body as `{ "input": ... }` in the
  frontend because `philharmonic-api/src/routes/workflows.rs`
  deserializes `ExecuteInstanceRequest { input }`; the README's
  send shapes describe the workflow input nested inside that API
  envelope.
- Embedded frontend assets with `rust-embed`, mirroring the
  existing `philharmonic::webui` static-asset pattern without
  modifying the donor crate.

Residual risks / items for Yuka to inspect first:

- `POST /mint-ephemeral` remains unauthenticated and
  rate-limit-free by design for this round. It can create
  workflow instances and mint one-hour instance-scoped tokens
  for anyone who can reach the bin.
- The chat frontend treats transcript output as either a raw
  message array or `{ "messages": [...] }` to tolerate the
  README wording and the existing WebUI parser precedent. If
  the canonical chat workflow chooses only one shape, the UI can
  be narrowed later.
- The notification list is only the local browser's recent
  discoveries. It intentionally does not claim to be exhaustive
  and does not render an inbox-zero state.
- The new build script is a second Node-using workspace script,
  created because the prompt explicitly requested it and the
  existing exception pattern is the WebUI build wrapper.
