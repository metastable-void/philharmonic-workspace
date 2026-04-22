# Instructions/Notes for human developers

**For coding agents**: You can freely read the contents of
this file, but never edit this file.

**The following is notes for humans and not for coding agents**

---

## Short-term TODOs

- update ROADMAP.md regularly.
- talk with Claude Code about what to do next;
  we have a clear deadline (before the Japanese
  Golden Week holidays).
- not blocking for this project: we should refactor all
  the best practices for AI coding brewed inside this
  workspace repo into a reusable template/Rust crate/etc.
  this can happen after the MVP work is done.
- update the cornerstone crate with tests where feasible,
  and correct things if any bugs caught; publish a new
  patch version; run `cargo update` at the workspace and
  update every pointer to the cornerstone crate with
  the updated patch version.
