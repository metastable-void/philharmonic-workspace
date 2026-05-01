# Instructions/Notes for human developers

**For coding agents**: You can freely read the contents of
this file, but never edit this file.

**The following is notes for humans and not for coding agents**

---

## Reminders

- make sure we always make docs/roadmaps up-to-date.
- not blocking for this project: we should refactor all
  the best practices for AI coding brewed inside this
  workspace repo into a reusable template/Rust crate/etc.
  this can happen after the MVP work is done.

## Restructure docs/

- `docs/` directory should be a home to mdBook-based docs on
  GitHub Pages. If there is a conflict, `docs` can be renamed
  to `docs-src` or something.
- Move fixtures, crypto vectors, etc. out of `docs`.
- docs/instructions/README.md should be demolished and use
  CONTRIBUTING.md and AGENTS.md/CLAUDE.md.
- docs/design/crypto-* should be moved outside the design chapter.

Structure:

- docs/ - something friendly but concise at top
  - docs/ROADMAP.md <- moved from the top
  - docs/design/
  - docs/crypto/...
  - docs/codex-{prompts,reports}
  - docs/notes-to-humans
  - docs/project-status-reports
  - docs/POSIX_CHECKLIST.md <- moved from the top

Add `cargo install mdbook` at setup.sh.

Update all remaining references to new ones.

Note: this docs will be at https://metastable-void.github.io/philharmonic-workspace/. Following notes apply.

> If your book is not deployed at the root of the domain, then you should set the output.html.site-url setting so that the 404 page works correctly. It needs to know where the book is deployed in order to load the static files (like CSS) correctly. For example, this guide is deployed at https://rust-lang.github.io/mdBook/, and the site-url setting is configured like this:
> 
> ```
> # book.toml
> [output.html]
> site-url = "/mdBook/"
> ```
