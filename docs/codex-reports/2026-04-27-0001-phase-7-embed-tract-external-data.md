# Phase 7 embed external-data notes

**Date:** 2026-04-27
**Prompt:** docs/codex-prompts/2026-04-27-0001-phase-7-embed-tract.md

Round 03 kept `BAAI/bge-m3` as the default embedding model and added an in-memory external-data path for tract.

The tract API path used for external data is `tract_onnx::Onnx::proto_model_for_read` to decode the ONNX protobuf, a custom `tract_onnx::data_resolver::ModelDataResolver` that serves slices from caller-supplied external-data bytes, and `tract_onnx::Onnx::parse(&tract_onnx::pb::ModelProto, Some(""))` to build the tract model without a runtime temp directory. A probe loaded the pinned bge-m3 ONNX graph through this path and reported 2,886 nodes and 2 outputs. A separate probe loaded the pinned MiniLM override through the single-file `model_for_read` path.

`build.rs` uses a HEAD request for `onnx/model.onnx_data` to decide whether to fetch and bundle external data. This was chosen over adding `prost` to parse `model.onnx` in the build script. It is sufficient for the pinned bge-m3 layout and the small MiniLM override, but it is less general for future models that use nonstandard external-data filenames or multiple external data files.

The required bge-m3 default `cargo check` passed with `PHILHARMONIC_EMBED_CACHE_DIR=/tmp/philharmonic-embed-cache-bge`. A targeted bge-m3 `cargo test -p philharmonic-connector-impl-embed --test external_data_loads` did not link: the 2.27 GB `include_bytes!` external-data object pushed the debug test binary past rust-lld's 32-bit PC-relative relocation range. That means the current build-time bundled bge-m3 path is checkable, and the tract load path was validated by the probe, but a full bge-m3 linked test binary needs a follow-up design if it must run under the current linker configuration.
