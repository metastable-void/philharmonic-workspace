# CONTRIBUTING historical notes cleanup

**Date:** 2026-05-02
**Prompt:** Direct user request in session: "please update CONTRIBUTING.md to archive/summarize historical notes as such. if in doubt, ask me before proceeding."

Codex updated `CONTRIBUTING.md` to make several historical asides visibly historical instead of blending them into active conventions. The active rules were left in place; only surrounding explanatory context was rephrased.

Changed areas:

- `CONTRIBUTING.md` §4.4 now keeps the `scripts/pull-all.sh` rebase exception as the active rule and labels the originally rejected alternatives as a `Historical note:`.
- `CONTRIBUTING.md` §4.7 now labels pre-`Code-stats:` trailer behavior in old commits as archival history.
- `CONTRIBUTING.md` §8.1 now keeps the xtask target-dir split rationale concise and moves the 2026-04-23 target-lock incident into a `Historical note:`.
- `CONTRIBUTING.md` §12.3 now states the current per-crate `cargo-semver-checks` rule first and labels the old `--workspace --baseline-rev <rev>` approach as historical.
- `CONTRIBUTING.md` §17.4 now treats the old `docs/instructions/` location as historical context, not a current documentation home.
- `CONTRIBUTING.md` §18.2 now adds a convention for future edits: historical context inside `CONTRIBUTING.md` should be labeled `Historical note:` and summarized briefly, with longer narratives kept in journal archives and cross-referenced only when load-bearing.

No Rust, script, or configuration files were changed by this cleanup. Verification for the markdown-only edit was:

- `git diff --check -- CONTRIBUTING.md`
- `./scripts/check-md-bloat.sh`
- `./scripts/tokei.sh`

`./scripts/pre-landing.sh` was not run because the change was documentation-only and did not touch Rust code or dependency metadata.
