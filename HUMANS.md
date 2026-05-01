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

## WebUI

Next task: Test WebUI end-to-end by using it by hand.

Then apply generalized localization patterns to add a
Japanese translation to the WebUI (auto-detects language
from the browser and also switchable from a dropdown
menu).

Note: 24-hour clock is preferred, with browser-native
timezones.

## Update the workflow authoring guide

Re-read the docs at mechanics-core crate, and update the
workflow authoring guide to match API shapes defined there.
