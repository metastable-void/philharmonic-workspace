# Audit refactor: mechanics worker executor extraction

**Date:** 2026-05-15
**Prompt:** HUMANS.md §Priority: Audit & refactor; follow-up: continue refactoring after mechanics-dns extraction.

This slice moved the mechanics-worker-backed workflow step executor out of the unpublished `philharmonic-api-server` binary and into `philharmonic-api` as the optional `mechanics-worker-executor` feature. The public type is re-exported as `philharmonic_api::MechanicsWorkerExecutor` when that feature is enabled.

The `philharmonic` meta-crate forwards the new surface through `api-mechanics-worker-executor`, and the default meta-crate feature set enables it so workspace default checks still compile the shipped API-server path. Deployment bins still opt into exactly the features they need with `default-features = false`.

The API server now imports `MechanicsWorkerExecutor` from `philharmonic::api` and no longer depends directly on `mechanics-http-client`. This keeps the bin closer to the documented thin-bin shape: CLI/config/process glue remains local, while reusable workflow execution glue lives in a library.

I intentionally left `lowerer.rs` and most of `embed_job.rs` local in this pass. Those paths include SCK decrypt/encrypt and endpoint payload handling, so a larger extraction should be a separate crypto-review-aware slice rather than incidental cleanup.
