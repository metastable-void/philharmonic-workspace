# Gate-1 approval — Phase 5 Wave A

I read through the revised proposal.

**I approve the proposal, with the caveat at Non-crypt things section.**

## Answers to open questions

1. (a).
2. okay for this time.
3. (adding `subtle = "2"` is okay.)

## Non-crypt things

- Requiring consumers of our crates to pass
  a file path is not something I like: it
  should be possible to pass the bytes - we
  generally enforce this discipline by
  documenting that. config files for lib
  crates are bad things - they should belong
  to bin crate things.
