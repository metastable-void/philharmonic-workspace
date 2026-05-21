# Instructions/Notes for human developers

**For coding agents**: You can freely read the contents of
this file, but never edit this file.

## Notes for coding agents

### Read HUMANS.md before committing

Codex never commits to Git. This rule is for Claude Code.

Claude Code must read the contents of HUMANS.md before
committing its changes.

---

**The following is notes for humans and not for coding agents**

## Reminders

- make sure we always make docs/roadmaps up-to-date.
- not blocking for this project: we should refactor all
  the best practices for AI coding brewed inside this
  workspace repo into a reusable template/Rust crate/etc.
  this can happen after the MVP work is done.

## Day-to-day housekeeping: Audit & refactor

### Maintainability notes

Always watch the whole workspace (spawning subagents is
preferred) for maintainability issues, dirty/spaghetti codes,
and quality issues (e.g. memory leaks, deadlocks, races, etc.).

Refactor codes to make the code structured, small, de-duplicated.

### Clean separation of concerns

- Unpublished bin crates should be minimal;
  **they own Clap CLI** (that should not be upstreamed),
  but any real codes should be upstreamed, creating
  crates if really necessary.
- Chats are a workflow knowledge; the framework in principle
  should not know anything about the workflows, but it's really
  useful for testing, so Chat UI will live elsewhere (in-tree
  `philharmonic-chat-app` bin for frontend/backend unified, or
  in another project) in the future, although we don't remove
  the old Chat UI immediately right now. See below.

## Chat UI separation

- Chat UI (Web UI + backend) server bin at `bins/philharmonic-chat`
  bin crate.
- Chat UI is stateless; no backend that maintains states. The API
  server is the backend.
- Customer support agent signs in with the same token as admin_token
  configured; otherwise, Chat UI backend rejects the sign in.
- Chat UI stores the token and other states in localStorage; not
  sessionStorage.
- `agent_name` key in localStorage saves the name of the customer
  support agent. It is configurable at the prominent edit box at the
  top of the UI. If not set, a modal element asks the support agent
  for a name (not dismissable unless one closes the window).
- Chat UI also has a mock-testing UI for disguising an end user's
  chat. When opening one, it mints a new ephemeral token with the
  server's minting token, and it is stored at
  `ephemeral_<instance_UUID>_token` key at localStorage. When session
  expires, the UI asks the backend to re-mint an ephemeral token.
- end user's side of Chat UI is the same as the Chat UI in the Web UI
  (legacy).
- support agent's side of Chat UI is also the same, except that it
  sends `{"content": "<text>", "agent": true}` instead.
- Chat UI polls the state of the workflow instance pointed by 
  `notify_instance_uuid` and its output, and when `instances` key of
  the output has a new value (string UUID), it notifies the agent of
  a new chat awaiting the agent's reply, with a sound and a toast.
- Chat UI stores the chat instances' UUID it have seen to
  localStorage. it displays the agent side's chat views newer to
  older.
- Chat's JS code (I wrote it) rejects any chat attempt with
  agent: true when its context is AI mode and not HUMAN mode (just
  a flag). You just need to know this behavior. When the JS code
  decides the chat must transition to HUMAN mode, it notifies the
  Chat UI via updating `notify_instance_uuid` instance (`http_forward`
  connector usage).

Date is milliseconds since the UNIX epoch. role is assistant for the
support agent's side, with only the name different.

```json
{"role":"assistant","content":"<GREETING>","name":"AI","date":1779347424000}
{"role":"user","content":"<something difficult>","name":"Customer","date":1779347425000}
{"role":"assistant","content":"That needs a human attention. I've called a human agent. Please wait.","name":"AI","date":1779347426000}
{"role":"assistant","content":"Hello. I'm Jane Doe.","name":"Jane Doe","date":1779347426000}
```

Please note that when executing a workflow in HUMAN state, the
response has only the user's message appended, and the human agent's
messages are retrieved by polling.

Polling is every 2s.

### Config

Config:

```toml
[chat]
bind = "[::]:443"
bind_h3 = "[::]:443"

api_url = "https://philharmonic-api.tld"
admin_token = "pht_..."
minting_token = "..."
chat_uuid = "..." # UUID of the chat workflow template
notify_instance_uuid = "..." # UUID of the notify workflow instance

[tls]
cert_path = "/path/to/fullchain.pem"
key_path = "/path/to/privkey.pem"
```

## WebUI

Note: Keep WebUI up-to-date with any API features added
in the future.

## Keep the workflow authoring guide up-to-date

Re-read the docs/codex of everything related, and re-write
workflow authoring guides in en/jp to reflect the facts
on any surface changes.
