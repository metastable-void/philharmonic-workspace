# Phase 7 Tier 1 — embed default-bundled-model architecture

**Author:** Claude Code · **Audience:** Yuka · **Date:** 2026-04-27 (Mon) JST

Yuka's calls B/C/D/E/F on the
[2026-04-26-0001 ambiguity sheet](2026-04-26-0001-phase-7-tier-1-next-steps-and-ambiguities.md)
came in this session, drafting the embed-tract Codex
prompt. The B answer landed on an architecture that
deviates from the original
[pivot plan](2026-04-24-0008-phase-7-embed-tract-pivot-plan.md)
§"Public surface" and §"Testing" — recording it here so
the deviation is durable.

## The decision

The `philharmonic-connector-impl-embed` lib crate carries
a default ONNX + tokenizer bundle, fetched at lib build
time by the crate's own `build.rs`, cached outside the
repo, and `include_bytes!`-d into the lib so consumers
can construct an `Embed` via `Embed::new_default()`
without supplying bytes themselves. `Embed::new_from_bytes(...)`
remains for consumers that bring their own model.

**Default model**: `BAAI/bge-m3` (multilingual, 1024-dim,
~2.3GB ONNX), pinned to a specific HF revision SHA. Picked
for multilingual quality.

**Knob shape**:
- **Cargo feature `bundled-default-model`** (default-on)
  gates everything. `--no-default-features` opts out
  cleanly: no build.rs network IO, no `Embed::new_default()`,
  `Embed::new_from_bytes(...)` still works.
- **Env vars** `PHILHARMONIC_EMBED_DEFAULT_MODEL=<HF repo>`
  and `PHILHARMONIC_EMBED_DEFAULT_REVISION=<sha>` change
  the bundled model at lib build time.
- **`PHILHARMONIC_EMBED_CACHE_DIR=<path>`** overrides the
  cache root; default is `$XDG_CACHE_HOME/philharmonic/embed-bundles/`
  with `$HOME/.cache/...` fallback. Outside the repo so
  no `.gitignore` entry is needed.
- **`DOCS_RS=1` auto-skip** in build.rs as a belt-and-
  suspenders for downstream consumer docs.rs builds; this
  crate's own docs.rs build also has
  `[package.metadata.docs.rs] no-default-features = true`.

`Embed::new_default()` is gated `#[cfg(all(feature =
"bundled-default-model", embed_default_bundle))]` —
exists iff the feature is on AND build.rs successfully
bundled.

## Trade-offs accepted

Surfaced these to Yuka before the call; all explicitly
accepted on 2026-04-27:

- **Build-script network IO** at lib build time. Hostile
  to Debian, NixOS, Bazel-style sandboxes. They opt out
  via the Cargo feature.
- **~2.3GB ONNX baked into the lib's static bytes** when
  the default holds. Compile + link times balloon; output
  binaries are ~2.3GB+. Acceptable for the deployment
  binary; for routine dev iteration we override to a
  smaller multilingual model via the env vars.
- **docs.rs / offline** handled by the feature opt-out
  + `DOCS_RS=1` auto-skip. Anyone consuming the published
  crate from a no-network environment uses
  `--no-default-features`.
- **Reproducibility** depends on pinning a real HF
  revision SHA — `main`/`HEAD` are rejected by build.rs
  with a clear error.

## What this supersedes

The pivot plan's §"Public surface" did not contemplate
`Embed::new_default()` (round-01's design was bytes-only).
The pivot plan's §"Testing" assumed env-var-gated
`#[ignore]` integration tests; the new design ships them
ungated under the cfg gate so they run by default with
the small-model override env vars set.

The rest of the pivot plan — tract op set, tokenizer
single-file change (`tokenizer.json` only, no four-file
bundle), error mapping, mean-pool with attention mask,
no-runtime-network constraint — stays authoritative.

## What's next

The Codex prompt is at
[`docs/codex-prompts/2026-04-27-0001-phase-7-embed-tract.md`](../codex-prompts/2026-04-27-0001-phase-7-embed-tract.md).
Once committed via `commit-all.sh --parent-only`, dispatch
via `codex:codex-rescue`.

The Phase 1 op-coverage probe runs against bge-m3 first;
if any tract op is rejected, the round stops and reopens
your call B (you'd pick a different default, possibly
something smaller). MiniLM-class models are pre-vetted as
op-clean per the pivot plan, but bge-m3's op set has not
been verified against tract until the probe runs.

## Things still pending

- A `scripts/embed-test.sh` wrapper that automates the
  small-model env-var setup for routine iteration. Out
  of scope for the Codex round; will land post-review if
  it earns its keep.
- A pivot-plan doc-reconciliation pass: ideally the plan
  itself is updated to point at this note rather than
  carrying the now-obsolete §"Public surface" /
  §"Testing" sections. Not blocking the Codex dispatch
  — landing this note plus the prompt is enough; the
  plan can be reconciled in the same Claude session that
  reviews the Codex output.
