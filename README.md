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
├── mechanics-http-client/                     # submodule
├── mechanics-http-server/                     # submodule
├── mechanics-core/                            # submodule
├── mechanics/                                 # submodule
├── dockerlet/                                 # submodule (dev-tooling)
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
`mechanics`, `mechanics-http-client` (workspace's outbound
HTTP client; hyper-rustls + webpki-roots + aws-lc-rs;
opportunistic HTTP/3 via the optional `http3` feature),
`mechanics-http-server` (opportunistic HTTP/3 listener +
Alt-Svc tower middleware that sits alongside the existing
TCP+TLS HTTP/1.1+HTTP/2 path; added 2026-05-13 for D22).

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

**Dev-tooling (published, submodule):** `dockerlet` —
minimal Docker test-container helper. Thin wrapper over
`bollard` with a deliberately narrow feature set (Unix
socket only, no `rustls-native-certs`, no `home`); used
by the SQL connector + e2e integration tests as a
lightweight alternative to the broader `testcontainers`
crate (which was evicted from the workspace dep tree
during the §3.J production-security cleanup pass on
2026-05-13).

**Meta-crate:** `philharmonic`.

**In-tree vendored fork (published, not a submodule):**
`mechanics-h3-quinn` — vendored from upstream `h3-quinn 0.0.10`
with the `quinn` dep pinned to drop the upstream `rustls-ring`
default. Eliminates the last `ring` wrapper exception from the
workspace's TLS posture. Maintained via the
`./scripts/xtask.sh vendor-upstream` bin (reads
`vendor/vendor.toml`; 3-day cooldown; SHA-256 verify). The
hand-written `Cargo.toml` is preserved across re-vendor;
`src/` is overwritten from upstream tarballs.

**In-tree workspace tooling (not published, not a submodule):**
`xtask` — multi-bin crate for dev tools written in Rust. Bins are
auto-discovered from `xtask/src/bin/*.rs`; run with
`./scripts/xtask.sh <bin> -- <args>`. See
[`CONTRIBUTING.md §8`](CONTRIBUTING.md#8-in-tree-workspace-tooling-xtask).

## Status

Design is substantially settled. v1 implementation path is
**complete through Phase 9**; reference deployment operational
since 2026-05-02 (a WebUI-created workflow runs end-to-end
through API → mechanics worker → connector router → connector
service → an OpenAI-compatible upstream LLM).

The platform's stated use-case — RAG-grounded chat backed by a
vector index and a relational DB, served by a
self-or-partner-hosted LLM — is verified end-to-end against the
deployment via the 2026-05-11 PoC milestone (embedding-dataset +
`embed` + `vector_search` + `sql_postgres` + `llm_openai_compat`
in one workflow). Authentication / authorisation / audit-log /
permission-aware WebUI / transport hardening (HTTP/3 +
HSTS-on-TLS + aws-lc-rs + webpki-roots) all in place.

**Current state (2026-05-14):** post-v1 dispatches landed
(D1-D6 embedding-datasets, D10/D11/D13/D14/D15 WebUI,
D12/D16 connector enhancements, D17 mechanics-core tail-promise
polling, D18 mechanics-core module-surface refactor
(`mechanics:html` + `:console` no-op + `:url` WHATWG + `:mime`
compose/parse + workflow-authoring guide refresh en+jp; the
setTimeout-removal sub-piece reverted D17's non-ES global),
D20 webpki-roots TLS, D21 pre-landing dep-aware test
filtering, D22 HTTP/3 client+server+streaming, D23 dockerlet,
D24 default-features audit, D25 hickory CVE bump). Plus the
in-tree vendored `mechanics-h3-quinn` (first non-submodule
publishable crate), the generic `vendor-upstream` xtask,
`check-no-registry` workspace-hardening guard, and dev-profile
incremental-build disable (2026-05-14 batch).
`mechanics-http-server 0.1.3` published to crates.io
2026-05-14.

**Remaining post-v1**: D7 SMTP, D8 Anthropic, D9 Gemini, D19
DNS (Tier 2/3 connectors — independent + parallel-safe).

§3.J production-security cleanup arc closed 2026-05-14. Banned-
dep posture: `pyo3` / `maturin` / `openssl-sys` / `native-tls` /
`rustls-platform-verifier` / `rustls-native-certs` / `ring` all
no-wrapper full bans on the workspace's ship targets
(`x86_64-unknown-linux-{gnu,musl}`).

The authoritative task list lives in
[`docs/ROADMAP.md` §3](docs/ROADMAP.md#3-post-v1-dispatch-plan).
Per-arc done-state snapshots + daily-log history in
[`docs/archive/`](docs/archive/); Codex prompt archives in
[`docs/codex-prompts/`](docs/codex-prompts/).

All 29 published-crate names are reserved on crates.io
(2026-05-13 added `mechanics-http-server`, published 0.1.0;
and `dockerlet`, published 0.1.0. 2026-05-14 added
`mechanics-h3-quinn`, published 0.0.10 as the workspace's
first in-tree non-submodule vendored fork.
`mechanics-http-server` is now at 0.1.3).
Foundational, API,
connector-triangle, and Phase 6/7 Tier 1 implementation crates
have published substantive releases at `0.1.0` or higher. The
remaining connector names (`philharmonic-connector-impl-email-smtp`,
`philharmonic-connector-impl-llm-anthropic`,
`philharmonic-connector-impl-llm-gemini`,
`philharmonic-connector-impl-dns`) are published placeholders
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
