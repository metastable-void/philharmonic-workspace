# `check-toolchain.sh` regression: `rustup check` exit 100 broke pre-landing

**Date:** 2026-04-22

## What happened

`./scripts/pre-landing.sh` aborted at step 0 today with exit
100, no error message, no test phases reached. Symptom:
check-toolchain.sh printed its rustup-check output and silently
terminated before the new nightly+miri probe section could
run, which under `set -e` in the caller propagated as a
pre-landing failure.

## Root cause

`rustup check` exits 100 when any installed toolchain has a
pending update. This is documented rustup behavior — it's how
CI scripts can detect "we're drifting, nudge the contributor"
— and it's quietly non-zero on exit.

The script's documented purpose is to **print** update status,
not **fail on** it. That intent was satisfied accidentally
before today: `rustup check` was the last statement in the
`else` branch of the `do_update`-switch, so its exit code
propagated out as the script's own exit code. Under `set -e`
in pre-landing.sh, a non-zero exit from check-toolchain.sh
would have aborted pre-landing — which means the same thing
would have broken whenever any toolchain had a pending update.

It didn't surface earlier because yesterday all three
(stable, nightly, rustup) were up-to-date — `rustup check`
returned 0. When I extended check-toolchain.sh yesterday to
probe nightly+miri presence, I put the probe AFTER the rustup
check. As long as the check returned 0 yesterday, the script
worked end-to-end. Today nightly drifted (new 9ec5d5f32 build
shipped), `rustup check` returned 100, `set -e` aborted
before the probe ran.

## Fix

Parent commit `440bec2`:

```sh
rustup check || :
```

Swallows the non-zero so downstream lines still run. Inline
comment explains the exit-100 semantic and the `|| :` choice.

POSIX-validated via `./scripts/test-scripts.sh`.

## Lesson for future shell work in this workspace

**`set -e` hides fragility until conditions change.** The
original script "worked" from day one, but only because
`rustup check` was at the end of execution. Any edit that
moved additional logic after it — as I did yesterday — turned
the latent exit-code issue into an active bug the moment the
toolchain drifted.

A couple of defaults worth keeping in mind for the next
shell edit:

- When calling an external tool whose exit-code semantics
  deviate from "0 = success, non-zero = failure", decide
  explicitly whether the script should propagate or swallow,
  and mark the choice at the call site. `cmd || :` vs.
  `cmd || some_recovery` vs. `if cmd; then ...` are all
  reasonable depending on intent — the wrong choice is the
  implicit one where the next refactor changes behavior.
- For "status printer" scripts, treat the tool's exit code
  as **data**, not control flow: capture it, print it, decide
  later whether to propagate. The check-toolchain.sh intent
  was always "print status, never fail" — making that
  explicit via `|| :` aligns the code with the docstring.
- The canonical `./scripts/test-scripts.sh` POSIX-parse check
  doesn't catch exit-code logic; only structural syntax. Exit
  semantics need manual thought.

## No follow-up work needed

The fix is minimal, tested, committed, pushed. Just noting
the regression mechanism here so it doesn't re-land next
time someone extends check-toolchain.sh or writes a similar
"tolerate non-zero from a tool" script.
