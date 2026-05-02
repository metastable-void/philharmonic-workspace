# Philharmonic Workspace

Generic workflow orchestration infrastructure. Rust crate
family with three deployment binaries: a public HTTP API
server, a mechanics worker (JavaScript executor), and a
per-realm connector binary. End-to-end encrypted connector
pipeline with post-quantum cryptography.

This book is the design documentation and reference material.
For development conventions (Git workflow, POSIX shell rules,
Rust code rules, versioning, terminology), see
[CONTRIBUTING.md](https://github.com/metastable-void/philharmonic-workspace/blob/main/CONTRIBUTING.md)
in the repository root. For current phase status and crate
publication state, see the workspace
[README.md](https://github.com/metastable-void/philharmonic-workspace/blob/main/README.md)
and [ROADMAP.md](ROADMAP.md).

## How to read this book

- **For the big picture**: start with
  [Project overview](design/01-project-overview.md), then
  [V1 scope](design/15-v1-scope.md) to see what shipping
  looks like.
- **For architectural depth**: read
  [Design principles](design/02-design-principles.md), then
  the layer-by-layer docs in order
  ([Cornerstone vocabulary](design/04-cornerstone-vocabulary.md)
  → [Storage substrate](design/05-storage-substrate.md)
  → [Execution substrate](design/06-execution-substrate.md)
  → [Workflow orchestration](design/07-workflow-orchestration.md)
  → [Connector architecture](design/08-connector-architecture.md)
  → [Policy and tenancy](design/09-policy-and-tenancy.md)
  → [API layer](design/10-api-layer.md)).
- **For cryptographic specifics**:
  [Security and cryptography](design/11-security-and-cryptography.md)
  consolidates the design across token systems, encryption
  systems, and key management.
- **For open decisions**:
  [Open questions](design/14-open-questions.md) aggregates
  everything still pending.
- **For post-MVP extensions**:
  [Embedding datasets](design/16-embedding-datasets.md).
- **For the user-facing workflow guide**:
  [Workflow authoring](guide/workflow-authoring.md).

Files mix **settled decisions** with **open questions** and
mark the difference explicitly. When a doc and the code
disagree, the code wins; flag the doc drift.
