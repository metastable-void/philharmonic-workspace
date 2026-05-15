# Audit refactor server helpers

**Date:** 2026-05-15
**Prompt:** direct Codex dispatch from `HUMANS.md` §Priority: Audit & refactor

The first audit/refactor slice focused on unpublished deployment bins.
`bins/mechanics-worker`, `bins/philharmonic-connector`, and
`bins/philharmonic-api-server` all carried identical default-serve command
construction and near-identical "missing primary config means use built-in
defaults unless `--config` was explicit" handling. The API and connector bins
also duplicated raw-or-hex key-material file parsing.

Those helpers now live under `philharmonic::server`:

- `server::cli::default_serve_command()` and `BaseArgs::default()`.
- `server::config::load_config_defaulting_missing()`.
- `server::key_material::{read_key_file, read_fixed_key_file,
  read_fixed_secret_file}`.

The design choice was to put these in the published meta-crate's existing
server-support namespace rather than create a new crate. The code is deployment
glue shared by the workspace's bins, and `CONTRIBUTING.md` §10.14 already points
shared CLI/server scaffolding at `philharmonic/src/server/`.

This pass intentionally did not extract the duplicated HTTPS/H3 accept-loop code
from the API and connector bins. That is another real clean-separation candidate,
but moving it into `philharmonic::server` would require a larger public feature
surface around `axum`, `hyper-util`, `tokio-rustls`, and
`mechanics-http-server`. It is better handled as its own reviewable slice.
