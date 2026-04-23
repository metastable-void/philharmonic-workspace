# Project status reports

Archive of LLM-generated summaries of the Philharmonic workspace's
development history and current status, produced by
`./scripts/project-status.sh` (see the script's header and the
top-level [`README.md §Scripts`](../../README.md) entry for it).

## File naming

`YYYY-MM-DD-hh-mm-ss.md` — local-time timestamp at the moment the
script was invoked. One file per invocation; the script refuses to
overwrite an existing path so sub-second collisions surface loudly
rather than silently stomping a previous report.

## Why commit these?

The reports are committed (not `.gitignore`-d) for two reasons:

1. **Preserve history.** Each snapshot is a point-in-time view of
   where the workspace was — useful later for reconstructing the
   shape of work even when the code and ROADMAP have moved on.
2. **Avoid repeated API calls.** Regenerating a report each time
   someone wants to read a past status costs money, tokens, and
   network round-trips. Committed reports mean re-reading an
   older snapshot is a plain `cat` away.

## What *not* to do

- **Do not edit committed reports.** They're archived model
  output; rewriting them retroactively defeats the "point-in-time
  snapshot" role. If a report is wrong or misleading, generate a
  new one rather than mutating the old one.
- **Do not treat reports as authoritative.** They summarise the
  other authoritative sources (`README.md`, `ROADMAP.md`, git
  log) at a point in time. The authoritative sources — and the
  code itself — always win when they disagree.
- **Do not commit reports without reading them first.** Model
  output can hallucinate, mis-cite SHAs, or invent roadmap items
  that don't exist. A quick scan before `./scripts/commit-all.sh`
  catches the obvious misses.
