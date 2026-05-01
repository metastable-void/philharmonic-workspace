# Deployment bugs batch — 2026-05-01 evening session

Bugs found and fixed during hands-on WebUI + deployment testing.

## 1. Endpoint tests missing `implementation` field

Tests sent `CreateEndpointRequest` without the `implementation`
field added earlier today. All 7 endpoint tests + e2e pipeline
test returned 422. Fixed by adding `"implementation":
"llm_openai_compat"` to test helpers and seed functions.

## 2. Branding endpoint returned `unscoped_request`

The scope resolution middleware ran on `/v1/_meta/` paths. When
the WebUI reloaded while logged in, it sent `X-Tenant-Id` with
the branding request. If UUID resolution failed, the scope
middleware rejected the request before the branding handler ran.

Fix: scope middleware now skips `/v1/_meta/` paths (same as
the auth middleware already did). Meta endpoints always get
`RequestScope::Operator`.

## 3. Executor posted to wrong mechanics worker URL

`MechanicsWorkerExecutor` posted to the bare `mechanics_worker_url`
(e.g. `http://127.0.0.1:3001/`) but the mechanics worker listens
on `POST /api/v1/mechanics`. Result: 404.

Fix: executor now appends `/api/v1/mechanics` to the configured
base URL.

## 4. Executor didn't send bearer token to mechanics worker

The mechanics worker requires bearer token auth on every request.
Empty `tokens = []` is fail-closed (every request gets 401). The
executor had no way to send a token.

Fix: added `mechanics_worker_token` config field to api.toml.
The executor sends `Authorization: Bearer <token>` when
configured. Both sides must share the same token string.

## 5. WebUI bundle not re-embedded on rebuild

`rust_embed` reads `webui/dist/` at compile time via proc macro,
but Cargo doesn't track those files as inputs. Changing only the
bundle (no `.rs` changes) meant Cargo skipped recompilation.

Fix: added `build.rs` with `cargo::rerun-if-changed=webui/dist/`
to the `philharmonic` crate.

## 6. `webui-build.sh` allowed dev builds

Running without `--production` produced a 5 MB dev bundle with
inline CSS (style-loader) and no source maps. The build script
then failed its own artifact check (missing `main.css`,
`main.js.map`, `main.css.map`).

Fix: `--production` is now required. Without it the script
prints a clear error and exits 2.

## Mechanics worker token configuration

In `mechanics.toml`:
```toml
tokens = ["shared-secret-here"]
```

In `api.toml`:
```toml
mechanics_worker_url = "http://127.0.0.1:3001"
mechanics_worker_token = "shared-secret-here"
```

Empty `tokens = []` is intentionally fail-closed: the operator
must configure at least one token before the worker accepts
traffic.
