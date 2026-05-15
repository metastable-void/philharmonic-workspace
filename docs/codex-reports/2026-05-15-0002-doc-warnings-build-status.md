# Rustdoc Warning Cleanup And Build Status Visibility

**Date:** 2026-05-15
**Prompt:** Chat request: clarify and fix rustdoc warnings, then improve `scripts/build-status.sh` visibility for build-script and cargo phases.

## Summary

Two rustdoc warnings were caused by ambiguous or invalid intra-doc links:

- `philharmonic-connector-impl-api/src/lib.rs` linked `async_trait` without saying whether it meant the crate/module or the exported attribute macro. The docs now link to `macro@async_trait` where the macro is intended.
- `philharmonic-connector-impl-embed/src/lib.rs` linked `tract-onnx`, but the crate is imported as `tract_onnx` in Rust while the package name contains a hyphen. The package name is now plain code prose instead of a broken intra-doc link.

`scripts/build-status.sh` was also improved after a long embed ignored-test compile produced no useful status output during build-script-heavy work. The script now reports the cargo driver phase explicitly (`check`, `clippy`, `doc`, `test`, `test --ignored`, `miri ...`, etc.) rather than the generic `testing` label. It also distinguishes rustc compilation of build scripts (`build_script_build` / `build_script_main`) from normal crate compilation and continues to report running `build-script-*` executables.

## Intended Behavior

When cargo appears quiet, `./scripts/build-status.sh` should show whether the active work is the cargo driver, rustc compiling a normal crate, rustc compiling a build script, a running build script, rustdoc, clippy, rustfmt, or linking. This is especially useful for dependencies such as `aws-lc-sys`, `tract-linalg`, and the bundled embed model path, where build scripts or generated artifacts can create long stretches without ordinary crate output.

## Validation

Focused validation passed:

- `./scripts/rust-lint.sh philharmonic-connector-impl-api`
- `./scripts/rust-lint.sh philharmonic-connector-impl-embed`
- `./scripts/test-scripts.sh`
- `./scripts/pre-landing.sh`

The workspace rustdoc step no longer emits the `async_trait` or `tract-onnx` warnings. Cargo still emits the unrelated existing output filename collision warning for the `philharmonic-api` lib and the `philharmonic-api-server` bin target named `philharmonic-api`.
