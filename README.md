# Philharmonic Workspace

Development harness for the Philharmonic crate family — a workflow
orchestration system built as a set of independent Rust crates.

This repository contains the Cargo workspace manifest, shared
development scripts, and Git submodules pointing at each crate's
own repository. Each crate is published independently to crates.io
and has its own issue tracker, CI, and release cycle.

**Contributor conventions live in [`CONTRIBUTING.md`](CONTRIBUTING.md)** —
git workflow, script wrappers, Rust code rules, versioning,
licensing, terminology, everything. Read it before your first
commit. The roadmap and active dispatch plan live in
[`docs/ROADMAP.md`](docs/ROADMAP.md).

## About Philharmonic

Philharmonic is a workflow orchestration system with JavaScript-
based workflows, sandboxed execution in stateless Boa runtimes, and
per-step encrypted authorization between scripts and external
services. The storage substrate is append-only and content-
addressed; the execution substrate runs JavaScript jobs as a
horizontally-scalable HTTP service; the connector layer mediates
all external I/O under per-realm isolation with hybrid post-quantum
cryptography.

See the design documentation for the full architectural picture —
start with [`docs/design/01-project-overview.md`](docs/design/01-project-overview.md).

## Notes for humans

Notes for humans live at [`HUMANS.md`](HUMANS.md). Claude Code can
and must commit human-made changes to it on every commit the agent
makes. Normal `./scripts/commit-all.sh` invocations will do that.

Coding agents (Claude Code and Codex) can freely read the contents
of `HUMANS.md`, but they MUST NOT change the contents there.

## Repository structure

```
philharmonic-workspace/
├── Cargo.toml                                 # workspace manifest
├── scripts/                                   # helper scripts
├── philharmonic-types/                        # submodule
├── philharmonic-store/                        # submodule
├── philharmonic-store-sqlx-mysql/             # submodule
├── mechanics-config/                          # submodule
├── mechanics-core/                            # submodule
├── mechanics/                                 # submodule
├── philharmonic-policy/                       # submodule
├── philharmonic-workflow/                     # submodule
├── philharmonic-connector-common/             # submodule
├── philharmonic-connector-client/             # submodule
├── philharmonic-connector-router/             # submodule
├── philharmonic-connector-service/            # submodule
├── philharmonic-connector-impl-api/           # submodule
├── philharmonic-connector-impl-*/             # submodules (one per impl)
├── philharmonic-api/                          # submodule
├── philharmonic/                              # submodule
├── inline-blob/                               # submodule
├── bins/                                      # in-tree bin crates
│   ├── mechanics-worker/                      #   JS executor server
│   ├── philharmonic-connector/                #   per-realm connector service
│   └── philharmonic-api-server/               #   API server + WebUI
├── xtask/                                     # in-tree workspace tooling
└── docs-jp/                                   # submodule (docs only)
```

Each submodule is a standalone Git repository at
`github.com/metastable-void/<crate-name>`. **`docs-jp/`** is docs-
only (Japanese-language briefing material; exempt from the
English-only rule per [`CONTRIBUTING.md §14.6`](CONTRIBUTING.md#146-english-as-the-default)).

### Crates at a glance

**Core vocabulary:** `philharmonic-types`.

**Storage substrate:** `philharmonic-store`,
`philharmonic-store-sqlx-mysql`.

**Execution substrate:** `mechanics-config`, `mechanics-core`,
`mechanics`.

**Policy and workflow:** `philharmonic-policy`,
`philharmonic-workflow`.

**Connector layer:** `philharmonic-connector-common`,
`philharmonic-connector-client`, `philharmonic-connector-router`,
`philharmonic-connector-service`, `philharmonic-connector-impl-api`.

**Connector implementations:** `philharmonic-connector-impl-` ×
`http-forward`, `llm-openai-compat`, `llm-anthropic`, `llm-gemini`,
`sql-postgres`, `sql-mysql`, `email-smtp`, `embed`, `vector-search`,
`dns` (planned — see D19).

**API:** `philharmonic-api`.

**Build tooling (published, submodule):** `inline-blob` —
proc-macro for embedding large binaries in `.lrodata.*` ELF
sections; used by the embed impl for ONNX model bundling.

**Meta-crate:** `philharmonic`.

**In-tree workspace tooling (not published, not a submodule):**
`xtask` — multi-bin crate for dev tools written in Rust. Bins are
auto-discovered from `xtask/src/bin/*.rs`; run with
`./scripts/xtask.sh <bin> -- <args>`. See
[`CONTRIBUTING.md §8`](CONTRIBUTING.md#8-in-tree-workspace-tooling-xtask).

## Status

Design is substantially settled and the v1 implementation path is
complete through Phase 9. The reference deployment is operational
since 2026-05-02: a WebUI-created workflow has run through the full
production path from API server to mechanics worker, connector
router, connector service, and an OpenAI-compatible upstream LLM
via `llm_openai_compat`.

**End-to-end PoC milestone — 2026-05-11**: a complete chatbot
use-case ran successfully across the full stack: an embedding
dataset + `embed` + `vector_search` retrieval path, a
`sql_postgres` DB connector endpoint, and an OpenAI-compatible
LLM (OVHCloud HF-endpoint serving `Qwen/Qwen3-32B`) — all in
one workflow, all through the production API + mechanics + connector
chain. This is the first full real-world chatbot RAG flow on the
platform; the platform's stated use-case (RAG-grounded chat
backed by a vector index and a relational DB, served by a
self-or-partner-hosted LLM) is now verified end-to-end against an
actual deployment, not just integration tests. The deployment-time
fixes that landed earlier in the day (mechanics-core 0.4.0
unhandled-rejection runtime fix, WebUI permission-aware
nav/buttons, audit-log producer gap close, connector body cap
2 MiB → 32 MiB) all surfaced from and were validated against
this same PoC session.

The **embedding-datasets feature** is fully shipped end-to-end
as of 2026-05-10 — data layer, workflow-engine integration,
API CRUD, WebUI, both crypto gates cleared. The **chat-style
testing UI (D13)** also landed 2026-05-10: one-click create-
and-chat from a workflow template, in-WebUI chat tab on
`InstanceDetail`, runtime structural detection of the OpenAI-
style `{messages: [{role, content}, ...]}` shape. The
**workflow authoring guide (D11)** was rewritten the same
day with three load-bearing recipes (D13 chat,
embedding-datasets, combined RAG); 2026-05-11 added
per-connector request/response shape tables across all
shipped connectors. The three 2026-05-11 HUMANS.md
follow-ups (D14 markdown rendering with DOMPurify in the
chat UI; D15 `abstract_config` structured editor; D16
`tool_choice: "auto"` for `llm_openai_compat`) all landed
that day.

**Deployment-time testing on 2026-05-11** drove additional
fixes that aren't numbered Codex dispatches but are
worth knowing about: `mechanics-core` 0.4.0 (runtime no
longer overrides `main`-fulfilled success with
"unhandled promise rejection" for workflows whose
`try { await endpoint(...) } catch { }` correctly handled
the error); WebUI permission-aware navigation + disabled
non-actionable buttons + sticky sidebar footer
(`philharmonic-api` 0.1.8 `WhoamiResponse` extended with
`permissions: Vec<String>`); audit-log producer gap
closed (`philharmonic-policy` 0.2.3
`audit_event_type` module with 17 canonical
discriminants; `philharmonic-api` wired 19 producer
call sites with privacy-restricted token-mint payloads
enforced by absence-assertion tests); connector-path
body cap raised 2 MiB → 32 MiB
(`philharmonic-connector-router` 0.1.2).

Remaining post-v1 scope: Phase 7 Tier 2/3 connector
implementations (SMTP, Anthropic, Gemini — D7/D8/D9), a new
DNS connector (D19, fresh crate
`philharmonic-connector-impl-dns` — Tier 2), and a
`mechanics-core` module-surface refactor (feature gating
+ new `mime`/`url`/`console`/`html` modules — D18). The
2026-05-12 work — `mechanics-core` 0.4.0 → 0.4.1 with
tail-promise polling (D17 landed) — moved the worker
run-job response fence from quiescence to the script's
`return`. The authoritative task list lives in
[`docs/ROADMAP.md` §3](docs/ROADMAP.md#3-post-v1-dispatch-plan)
with verbatim pre-trim ROADMAP content at
[`docs/archive/2026-05-11-roadmap-completed-arc-trim.md`](docs/archive/2026-05-11-roadmap-completed-arc-trim.md)
and [`docs/archive/2026-05-12-roadmap-d17-done-d7-spec-d18-added.md`](docs/archive/2026-05-12-roadmap-d17-done-d7-spec-d18-added.md).

All 25 crate names are reserved on crates.io. Foundational, API,
connector-triangle, and Phase 6/7 Tier 1 implementation crates
have published substantive releases at `0.1.0` or higher. The
remaining connector names (`philharmonic-connector-impl-email-smtp`,
`philharmonic-connector-impl-llm-anthropic`,
`philharmonic-connector-impl-llm-gemini`) are published placeholders
at `0.0.x` until their implementations land.

## Working in the repo

The fast path:

```bash
git clone --recurse-submodules https://github.com/metastable-void/philharmonic-workspace.git
cd philharmonic-workspace
./scripts/setup.sh
```

Everything you need to know to develop here is in
[`CONTRIBUTING.md`](CONTRIBUTING.md):

- §1 Quick start, §2 Development environment (POSIX-ish hosts only)
- §4 Git workflow (`scripts/*.sh` wrappers, sign-off + signing,
  append-only history)
- §5 Script wrappers over raw cargo
- §6 POSIX sh discipline, §7 External tool wrappers,
  §8 In-tree `xtask/` tooling
- §10 Rust code rules (no panics in lib `src/`, library boundaries,
  HTTP client split)
- §11 Pre-landing checks (mandatory before every commit touching
  Rust)
- §15 Notes-to-humans, Codex prompt archive, project status reports

For agents: [`CLAUDE.md`](CLAUDE.md) (Claude Code) and
[`AGENTS.md`](AGENTS.md) (Codex). The two-gate cryptographic
review protocol is documented in the
[`crypto-review-protocol`](.claude/skills/crypto-review-protocol/SKILL.md)
skill, with a cross-reference at
[`docs/ROADMAP.md §2`](docs/ROADMAP.md#2-crypto-review-protocol-pointer).

Older detail (the workspace-scripts inventory, audit-trailer schema,
xtask bin enumeration, environment matrix, publishing flow) was
trimmed from this README on 2026-05-10 and is preserved verbatim at
[`docs/archive/2026-05-10-readme-roadmap-trim.md`](docs/archive/2026-05-10-readme-roadmap-trim.md).
The authoritative current versions of those topics live in
`CONTRIBUTING.md`.

## License

All crates are dual-licensed under `Apache-2.0 OR MPL-2.0` at the
consumer's choice. Both are on the FSF's approved free-software
list; both are FLOSS by OSI classification. The combination covers
more deployment scenarios than `Apache-2.0 OR MIT` while keeping
every crate inside the FLOSS category.

The workspace root carries reference copies at
[`LICENSE-APACHE`](LICENSE-APACHE) and [`LICENSE-MPL`](LICENSE-MPL).
Individual crates carry their own `LICENSE-APACHE` / `LICENSE-MPL`
files under each submodule.

---

**Funding note**: The maintainer of this project (Yuka MORI,
`metastable-void` on GitHub) is paid for the work inside it by a
company, which is not Menhera.org. **This is not a Menhera.org
project.**
