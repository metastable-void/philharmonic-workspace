# philharmonic-chat

Agent-facing chat surface for the Philharmonic platform, plus a
mock-testing UI for support agents to disguise themselves as an end
user. Talks to a `philharmonic-api` instance over HTTPS; serves a
small static frontend and a tiny token-mint proxy from its own bind.

This README is the canonical design home for the chat project — the
workspace's `docs/design/` covers the Philharmonic framework, not
this surface. Update this file in the same commit as any
structurally visible change to the chat bin.

## Status

Scaffold. The bin parses its TOML config and exits — the server
body and bundled frontend are pending. The contract below is
fixed; the workflow template implementing it is authored
separately and not packaged with this bin.

## What this bin does

Three jobs, in order of weight:

1. Serves the chat frontend (static HTML / JS / CSS) at `/`.
2. Exposes `POST /mint-ephemeral` — the only place the
   `minting_token` is touched. Creates a new chat-workflow
   instance from the configured `chat_uuid` template with
   `service_token` and mints an ephemeral token scoped to that
   instance (execute + read).
   Returns `{ "ephemeral_token": "...", "instance_id": "..." }`.
   Requires `Authorization: Bearer <agent_token>`; the bin
   compares the presented token against the configured
   `agent_token` in constant time and returns 401 on mismatch.
   Only signed-in agents can mint (no public mock-test flow).
3. Exposes `GET /config` — returns `{ "api_url": "...",
   "notify_instance_uuid": "..." }` so
   the static frontend doesn't have to be rebuilt per
   deployment.
4. Exposes `POST /sign-in` — compares the typed agent token
   against configured `agent_token` and returns 204 or 401.
5. Exposes `GET /version` — returns the bin crate version and
   embedded git SHA for frontend refresh detection.

That's the entire HTTP surface. Chat traffic itself goes browser
→ `api_url` directly with whichever token the browser holds
(`agent_token` or the ephemeral). The bin is server-stateless;
the browser holds session state in `localStorage`.

## Threat model in one paragraph

`minting_token` and `agent_token` never leave the bin's process /
its TOML config. The browser holds the `agent_token` (typed in
once by the support agent on first sign-in) or an ephemeral
instance-scoped token (received from `mint-ephemeral`); both go
directly to `api_url`. `agent_token` is the trust anchor for any
`agent: true` message; the workflow trusts the client-supplied
`name` field on those because the only path that can mint such a
message is the agent-authenticated UI, not the end-user UI.

## Authentication

- **Agent token** (`agent_token`): static, configured in the
  TOML. The support agent's UI loads it from `localStorage` (set
  on the first sign-in modal) and uses it directly against
  `api_url`. Minimum permission set: `workflow:instance_execute`,
  `workflow:instance_read`. *Not*
  an admin token — earlier sketches called it that; the name is
  retired.
- **Service token** (`service_token`): static, configured in the
  TOML and used only by the bin to create fresh chat workflow
  instances for `POST /mint-ephemeral`. Minimum permission set:
  `workflow:instance_create`.
- **Ephemeral token**: minted per chat instance by
  `POST /mint-ephemeral`. Allows execute + view on a single
  instance. Used by the mock-testing UI and, in the future,
  by the EC embed widget. Stored under
  `ephemeral_<instance_UUID>_token` in `localStorage`. On
  expiry the UI re-mints against the bin using the bin's
  configured `minting_token`.

## Branding

Tenant branding (name + monogram) is fetched by the chat
frontend from `GET {api_url}/v1/_meta/branding` directly,
mirroring the philharmonic WebUI's surface. The endpoint is
unauthenticated; the chat bin doesn't proxy it.

## UI modes

### Agent mode (default)

- Sign-in panel asks for `agent_token`. Stored in `localStorage`.
- `agent_name` modal blocks the UI until a name is set
  (configurable thereafter from a prominent edit box at the
  top). Stored in `localStorage`.
- Polls the `notify_instance_uuid` instance every 2 s (see
  *Notify channel* below) and renders the awaiting-chat list
  newer-to-older.
- When the agent opens a chat, the UI loads the chat instance's
  transcript via `GET workflows/instances/{id}/steps?limit=1`
  and renders it. Polls the same endpoint every 2 s for catch-up.
- Replies are sent as
  `{ "content": "...", "agent": true, "name": "<agent_name>" }`
  via the standard `workflows/instances/{id}/execute`.

### Mock-testing mode (end-user simulation)

- Opens a fresh end-user chat by calling `POST /mint-ephemeral`,
  storing the returned token under
  `ephemeral_<instance_UUID>_token` and using it directly
  against `api_url`.
- The UI body is the same as the agent's transcript view minus
  the `agent: true` flag — sends `{ "content": "..." }`. The
  workflow's script disambiguates and the `name` field on
  user-side messages is set by the workflow, never by the
  client.
- On first open of a fresh instance (zero steps), the UI
  triggers the workflow's greeting by calling
  `POST workflows/instances/{id}/execute` with empty input
  `{}` before the transcript loads. The workflow's script is
  expected to render an opening assistant turn in response.
  The trigger is single-shot per ChatPanel mount; subsequent
  polls render the persisted transcript. Agent mode shares
  the same code path but never hits the trigger in practice
  (agents only land on chats that already have steps via the
  notify channel).
- This is also the shape the future embed widget will follow.

## Wire shapes

### Messages stored on the workflow instance

```json
{ "role": "assistant", "content": "<text>", "name": "AI",        "date": 1779347424000 }
{ "role": "user",      "content": "<text>", "name": "Customer",  "date": 1779347425000 }
{ "role": "assistant", "content": "...",     "name": "AI",        "date": 1779347426000 }
{ "role": "assistant", "content": "...",     "name": "Jane Doe",  "date": 1779347427000 }
```

`date` is milliseconds since UNIX epoch. `role` stays
`assistant` for human-agent turns; only `name` distinguishes
them from AI turns.

### Send shapes

- End user / mock-testing: `{ "content": "..." }`. `name`
  comes from the workflow, never from the client.
- Agent: `{ "content": "...", "agent": true, "name": "<agent_name>" }`.
  Client-populated `name` is trusted because the request
  carries `agent_token`.

## State model

Two modes inside the chat workflow:

- **AI mode** — the workflow's script handles the conversation
  itself, appending an assistant turn per user turn. An
  inbound message with `agent: true` in AI mode is silently
  dropped (returning an error would terminate the instance —
  worse).
- **HUMAN mode** — the workflow appends end-user messages as
  before but does not auto-respond. Human-agent turns arrive
  via separate `execute` calls (one per agent reply) and are
  appended to the transcript by the script.

Transition AI → HUMAN is decided by the workflow script. On
transition the script calls the `http_forward` connector
against
`POST {api_url}/workflows/instances/{notify_instance_uuid}/execute`
with input identifying the just-transitioning chat instance.
The notify instance's own script appends that UUID into its
`output.instances` array (see below).

The reverse transition (HUMAN → AI) is out of scope for v0.

## Notify channel

`notify_instance_uuid` is a long-running workflow instance
whose `output.instances` is an **array of awaiting chat
UUIDs**. The notify workflow's script appends a chat
instance's UUID on each AI → HUMAN transition. In practice
the array often carries a single entry, but it can carry
several — clients must iterate, not assume length ≤ 1.

The notify instance is **not guaranteed to accumulate every
UUID forever**: its script may trim or rotate the array. So
across two poll cycles, an awaiting chat UUID may disappear
even though no agent picked it up.

Agent UIs poll the notify instance every 2 s and compare the
current `output.instances` against `seen_chat_uuids` in
`localStorage`; each UUID seen for the first time fires a
toast + sound and is added to `seen_chat_uuids`.

Consequence: **the notify channel is lossy by design.** A
chat awaiting a human reply for longer than the notify
instance's retention window may stop being announced. The
agent UI doesn't pretend the list is exhaustive — there's no
"0 awaiting" inbox-zero affordance. Exclusion / queueing
guarantees are future work.

## Polling

Globally 2 s:

- Agent UIs poll the notify instance.
- End-user and agent UIs poll their chat instance for catch-up.

The catch-up poll target is read-only:
`GET workflows/instances/{id}/steps?limit=1`. It surfaces the
latest step's output, which contains the full transcript.

Execute responses already include any messages appended through
that execute (including the just-sent turn), so polling is a
fallback to catch human-agent turns appended between the
client's own executes.

## Agent concurrency

Shared awaiting list — no claim, no presence, no assignment.
If two agents start replying to the same chat simultaneously,
whichever reply lands first wins; the second agent sees the
first reply on their next poll. Exclusion / claim is left as
future work.

## Browser-side storage

`localStorage` keys this UI relies on (no `sessionStorage`):

- `agent_token` — set on sign-in modal.
- `agent_name` — set on the (initial, blocking) name-prompt
  modal; editable thereafter from a prominent edit box at the
  top of the agent UI.
- `ephemeral_<instance_UUID>_token` — per mock-testing or EC
  widget session.
- `seen_chat_uuids` — the agent's debouncer for notify-channel
  events.
- `philharmonic.chat.locale` — English / Japanese UI language
  preference, set by the language switcher in the signed-in
  header.

"Server-stateless" describes the *bin*, not the browser; the
browser holds session state per the keys above.

## Internationalisation

The frontend supports English and Japanese. Locale preference is
stored in `localStorage` under `philharmonic.chat.locale`; when no
valid stored value exists, the UI checks `navigator.language`,
uses Japanese for `ja*`, and falls back to English for every other
language. The language switcher lives in the signed-in
`BrandHeader`.

## Configuration

```toml
[chat]
bind = "[::]:443"
bind_h3 = "[::]:443"

api_url       = "https://philharmonic-api.tld"
tenant_id     = "..." # UUID of the tenant the bin operates within
service_token = "pht_..."
agent_token   = "pht_..."
minting_token = "..."
chat_uuid            = "..." # UUID of the chat workflow template
notify_instance_uuid = "..." # UUID of the notify workflow instance

[tls]
cert_path = "/path/to/fullchain.pem"
key_path  = "/path/to/privkey.pem"
```

The bin's HTTP surface (`/`, `/config`, `/sign-in`,
`/mint-ephemeral`, `/version`) is served from `bind` (HTTPS);
`bind_h3` runs the HTTP/3 path on the same authority. TLS
material is rustls-loaded per the workspace's HTTP stack rules.

## Reference workflow template

Authored separately, not shipped with this bin. The chat UI
relies on the template implementing:

- AI-mode message handling (assistant turns appended by the
  script per user turn).
- Silent drop of `agent: true` while in AI mode.
- AI → HUMAN transition that calls `http_forward` against the
  notify instance.
- HUMAN-mode handling that appends end-user messages and any
  agent-supplied turn as `assistant` with the client-supplied
  `name`.

## Out of scope for v0

- Multi-tenancy. One bin = one tenant. Run another bin for
  another tenant.
- Agent assignment / claim / presence.
- EC embed widget (the bin will serve the embed JS in a
  future revision; not now).
- Customising the chat workflow template — one canonical
  template, authored by the project owner.
- HUMAN → AI transition.
