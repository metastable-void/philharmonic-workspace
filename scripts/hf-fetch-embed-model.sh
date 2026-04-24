#!/bin/sh
# scripts/hf-fetch-embed-model.sh — fetch an embedding model's
# ONNX + tokenizer bundle from HuggingFace into a local
# directory at DEPLOYMENT BUILD TIME, pinned by revision SHA
# and recorded in a per-file SHA256 manifest.
#
# Thin wrapper around the `hf-fetch-embed-model` xtask bin.
# See `xtask/src/bin/hf-fetch-embed-model.rs` for the
# authoritative documentation (arguments, output layout,
# idempotency rules, exit codes, what it does, what it does
# NOT do).
#
# Typical invocation:
#
#   ./scripts/hf-fetch-embed-model.sh \
#       --model sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2 \
#       --revision <pinned-git-sha> \
#       --out /path/to/deployment/assets/
#
# This tool is meant for operators preparing a connector-
# service binary that will `include_bytes!` the downloaded
# weights. `philharmonic-connector-impl-embed` itself has no
# network code — the model bytes flow only through
# `Embed::new_from_bytes(...)`. Do not invoke this xtask at
# runtime.
#
# POSIX sh only — see CONTRIBUTING.md §6.

set -eu
exec "$(dirname -- "$0")/xtask.sh" hf-fetch-embed-model -- "$@"
