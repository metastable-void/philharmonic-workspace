# Instructions/Notes for human developers

**For coding agents**: You can freely read the contents of
this file, but never edit this file.

**The following is notes for humans and not for coding agents**

---

## Short-term TODOs

- add more scripts.
- update ROADMAP.md.
- release mechanics-config/mechanics-core.
- from next run, when we don't touch the relevant crate, we can skip the lengthy integration tests (testcontainers, etc.) for the non-touched crate. let's write `./scripts/rust-lint.sh [<crate name>]` (check + fmt + clippy) and `./scripts/rust-test.sh [--include-ignored|--ignored] [<crate name>]`. when crate name is omitted the both run against the whole workspace. when a member crate is modified, we need to run `--ignored` separately from workspace-level test run without `--include-ignored|--ignored` flags.
