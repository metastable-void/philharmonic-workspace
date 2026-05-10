# `xtask/src/bin/detect-virt.rs` — portable systemd-detect-virt clone (initial dispatch)

**Date:** 2026-05-10
**Slug:** `detect-virt-xtask-bin`
**Round:** 01 (initial dispatch)
**Subagent:** `codex:codex-rescue`

## Motivation

Yuka asked Claude to add an in-tree workspace tooling bin that
mirrors `systemd-detect-virt(1)` but works portably across the
UNIXes the workspace runs on (Linux primary, FreeBSD / macOS
when CPUID is enough). Useful for environment probing in
diagnostic scripts, audit-trailer enrichment, and Codex / agent
session bootstrap (e.g. "are we inside a container?" decisions
that today rely on ad-hoc checks).

Not on the post-v1 dispatch plan in
[`docs/ROADMAP.md` §9](../ROADMAP.md#9-post-v1-dispatch-plan);
this is opportunistic workspace tooling that lives in `xtask/`
and never ships externally. No crypto path involvement, so no
Gate-1 review is needed.

## References

- Behavior target: current upstream `systemd-detect-virt(1)`
  man page.
- Implementation reference: systemd `src/basic/virt.c`
  (cite where each heuristic comes from).
- Workspace conventions:
  [`CONTRIBUTING.md §8`](../../CONTRIBUTING.md#8-in-tree-workspace-tooling-xtask)
  (in-tree xtask layout, `publish = false`, multi-bin discovery
  via `src/bin/*.rs`).
- Workspace HTTP / dependency split:
  [`CONTRIBUTING.md §10.9`](../../CONTRIBUTING.md#109-http-client-runtime-stack-vs-tooling-stack)
  — irrelevant here (no HTTP) but mentioned for context on the
  xtask vs. runtime distinction.
- No-panics rule:
  [`CONTRIBUTING.md §10.3`](../../CONTRIBUTING.md#103-panics-and-undefined-behavior)
  — the binary itself ships under `xtask/`, which is **exempt**,
  but the user's spec asks for `#![forbid(unsafe_code)]` and
  no `unwrap()` on the detection path anyway. Honor the
  spec's stricter discipline.

## Context files pointed at

- `xtask/Cargo.toml` (add `raw-cpuid` dependency; `clap` /
  `anyhow` are already present).
- `xtask/src/bin/detect-virt.rs` (NEW — the binary).
- `xtask/src/bin/` (existing siblings as style template:
  `gen-uuid.rs`, `tar-archive.rs`, `tar-concatenate.rs`,
  `resource-pressure.rs`, `system-resources.rs`).
- `xtask/tests/fixtures/detect-virt/` (NEW directory, holding
  synthetic `/proc` / `/sys` fixture trees for unit tests).
- `scripts/xtask.sh` (existing wrapper — no change needed; it
  auto-discovers any `xtask/src/bin/*.rs`).

## Outcome

**Completed in one round.** Codex landed:

- `xtask/src/bin/detect-virt.rs` (~1077 LOC) — single-file
  binary covering all spec'd CLI flags
  (`--vm` / `--container` / `-q` / `--list` / `--debug`,
  mutex via `clap` `ArgGroup`), the full container probe
  order (1-8), the full VM probe order (1-7,
  DMI-before-CPUID), the source-of-truth id table behind
  both `--list` and the matchers, fixture-driven `ProcFs`
  abstraction with a `RealFs` and `FixtureFs` impl,
  table-driven unit tests for the DMI matcher and
  `/proc/1/environ` parser, fixtures for
  `docker-on-kvm` / `kvm-on-amazon-ec2` / `vanilla-bare-metal`
  / `xen-dom0` / `wsl`, smoke test for `detect(Mode::Any,
  RealFs)`, `#![forbid(unsafe_code)]` at the bin level.
- `xtask/tests/fixtures/detect-virt/**` — five fixture
  trees (paths under each: `proc/1/environ`,
  `sys/class/dmi/id/*`, `proc/sys/kernel/osrelease`,
  `proc/xen/capabilities`, etc.).
- `xtask/Cargo.toml` — added `raw-cpuid = "11"` dep.
- `Cargo.lock` — picked up `raw-cpuid 11.6.0`.

**Verification (Codex's own pre-landing run):**
`./scripts/pre-landing.sh --xtask` passed clean.
`./scripts/xtask.sh detect-virt -- --list` printed all 29
ids ending in `none`. `./scripts/xtask.sh detect-virt --
--debug` on the build host detected `lxc` (correct — this
is a Linux container).

**Codex's open questions (resolved by Claude in review):**

1. *"Container env value `oci` was in the prompt's mapping
   but not in the source-of-truth id list — treated as
   no-supported-id rather than inventing one."* Correct
   call. The spec's id table doesn't include `oci`; if
   added later, both the table and the mapping land in the
   same edit.
2. *"`Cargo.lock` left dirty even though the action-safety
   write list omitted it — required for dep resolution."*
   Correct call. `Cargo.lock` always co-travels with a dep
   add; the action-safety block was about SCOPE, not
   exhaustivity.

**Claude follow-up before landing (target-portability fix):**
Codex placed `raw-cpuid = "11"` in unconditional
`[dependencies]`, so non-x86 build hosts (aarch64 musl,
ppc64le, etc.) would still try to pull and compile the
crate even though every line that references `raw_cpuid::*`
is already `cfg(any(target_arch = "x86", target_arch =
"x86_64"))`-gated. Moved the dep to a
`[target.'cfg(any(target_arch = "x86", target_arch =
"x86_64"))'.dependencies]` table so the dep graph itself
matches the source code's gating. Verified via
`cargo tree --target x86_64-unknown-linux-gnu` (raw-cpuid
present) vs. `--target aarch64-unknown-linux-gnu` /
`powerpc64le-unknown-linux-gnu` (raw-cpuid absent).
`pre-landing.sh --xtask` re-run after the fix is clean.

**Residual coverage gap:** `--list` advertises 29 ids but
the implementation can only actually emit ones reachable
from the probes — `qnx` / `acrn` / `powervm` / `apple` /
`sre` need s390 / arm64 / Apple Silicon / specific CPUID
vendors that the build host lacks. This matches systemd's
own behavior; flagged for the journal but not a fix
target.

---

## Prompt (verbatim)

<task>
Implement a new in-tree workspace-tooling Rust binary at
`xtask/src/bin/detect-virt.rs`. The bin is a portable clone of
`systemd-detect-virt(1)` — same command surface and same id
strings, but runs on any UNIX where the underlying signal is
available, with Linux-specific signals gated behind
`cfg(target_os = "linux")` and CPUID detection on every
x86 / x86_64 UNIX. Non-Linux non-x86 targets are allowed to
return `none`.

If anything in this prompt contradicts the workspace
authoritative docs (CLAUDE.md, AGENTS.md, CONTRIBUTING.md,
docs/ROADMAP.md, docs/design/), the docs win — flag the
contradiction in your structured output instead of guessing.

## Dependencies

- Add `raw-cpuid` (latest stable from crates.io) to
  `xtask/Cargo.toml` `[dependencies]`. Look up the latest
  non-yanked version with
  `./scripts/xtask.sh crates-io-versions -- raw-cpuid` and pin
  the major version (e.g. `raw-cpuid = "11"`).
- `clap` with the `derive` feature is already present in
  `xtask/Cargo.toml`; reuse it.
- `anyhow` is already present; reuse for error bubble-up.
- No async, no extra deps unless strictly needed.
- `#![forbid(unsafe_code)]` at the top of the bin.
- Match the workspace's edition (read from
  `xtask/Cargo.toml`).

## CLI surface

Mirror `systemd-detect-virt`:

- bare invocation → print the innermost detected id
  (`kvm`, `docker`, `none`, etc.) on stdout; exit 0 if anything
  detected, 1 if `none`.
- `--vm` → only check hypervisors.
- `--container` → only check containers.
- `-q` / `--quiet` → suppress stdout; just set the exit code.
- `--list` → print every id the tool can ever produce, one per
  line; exit 0.
- `--debug` → before the final stdout output, log every probe
  and its result on stderr (which probe ran, what file/path it
  consulted, whether it was skipped due to ENOENT/EACCES, and
  the matched id if any).
- `--vm` and `--container` are mutually exclusive (`clap`
  `group` or `conflicts_with`).

Use systemd's id strings verbatim where they exist — single
source-of-truth table mapped to enum variants so `--list`,
the matchers, and the CLI output cannot drift:

`kvm`, `qemu`, `bochs`, `xen`, `uml`, `vmware`, `oracle`
(VirtualBox), `microsoft` (Hyper-V), `zvm`, `parallels`,
`bhyve`, `qnx`, `acrn`, `powervm`, `apple`, `sre`, `google`,
`amazon`, `lxc`, `lxc-libvirt`, `systemd-nspawn`, `docker`,
`podman`, `rkt`, `wsl`, `proot`, `pouch`, `openvz`, `none`.

## Detection rules

**Innermost-wins**: a container inside a VM reports the
container. Default mode runs container detection first; on a
hit, return it. Otherwise fall back to VM detection.

### Container probes (Linux only — return `none` elsewhere)

In this exact order, first hit wins:

1. `/proc/1/environ` (NUL-separated) — find
   `container=<value>`, map: `lxc`, `lxc-libvirt`,
   `systemd-nspawn`, `podman`, `docker`, `oci`, `rkt`,
   `pouch`.
2. `/run/systemd/container` — file contents (trimmed) is the
   id.
3. `/run/host/container-manager` — same convention (rootless
   podman, etc.).
4. `/.dockerenv` exists → `docker`.
5. `/run/.containerenv` exists → `podman`.
6. `/proc/sys/kernel/osrelease` contains `microsoft` or `WSL`
   (case-insensitive) → `wsl`.
7. `/proc/vz` exists and `/proc/bc` does not → `openvz`.
8. `/proc/1/sched` first line not starting with `init `,
   `systemd `, or `launchd ` — last-resort hint only;
   do **not** invent an id from this. Treat as "unknown
   container; return generic if combined with another weak
   signal, else none".

### VM probes

Order matters. **DMI before CPUID** —
VirtualBox-on-KVM exposes `KVMKVMKVM` in CPUID; DMI gives the
correct `oracle` answer.

1. **DMI (Linux)** — read `/sys/class/dmi/id/{sys_vendor,
   product_name, bios_vendor, chassis_vendor,
   chassis_asset_tag}`. Substring match (case-insensitive
   where reasonable):
   - `KVM` → `kvm`
   - `Amazon EC2` → `amazon`
   - `Google` → `google`
   - `VMware` / `VMW` → `vmware`
   - `innotek GmbH` / `VirtualBox` /
     `Oracle Corporation` (cross-checked against chassis) →
     `oracle`
   - `Xen` → `xen`
   - `Bochs` → `bochs`
   - `Parallels` → `parallels`
   - `BHYVE` → `bhyve`
   - `Microsoft Corporation` + product `Virtual Machine` →
     `microsoft`
   - `QEMU` → `qemu` (only if no more specific vendor
     matched)
2. **`/proc/xen/capabilities` (Linux)** — contains
   `control_d` ⇒ dom0, treat as `none` for VM check;
   otherwise `xen`.
3. **`/sys/hypervisor/type` (Linux)** — fallback for `xen`.
4. **CPUID** (x86 / x86_64 anywhere) via `raw_cpuid::CpuId`:
   - `get_feature_info().has_hypervisor()` must be true,
     else `none`.
   - `get_hypervisor_info().identify()` → map known variants
     to systemd ids.
   - For `Unknown(a, b, c)`, reconstruct the 12-byte vendor
     string and match against `prl hyperv ` / ` lrpepyh vr`
     (Parallels), `VBoxVBoxVBox` (VirtualBox),
     `TCGTCGTCGTCG` (qemu/tcg), etc.
5. **`/proc/device-tree/hypervisor/compatible` (Linux,
   non-x86)** — `linux,kvm` → `kvm`; `xen` → `xen`; etc.
6. **`/proc/sysinfo` (Linux s390)** — line
   `VM00 Control Program` ⇒ `zvm`.
7. **`/proc/cpuinfo` (Linux)** —
   `vendor_id : User Mode Linux` ⇒ `uml`.

## Code structure

Single-file binary (`xtask/src/bin/detect-virt.rs`) is fine
even with several inline modules; do not split into multiple
files unless one of them grows past ~200 LOC of standalone
logic. Use:

- `mod cpuid` — pure CPUID, target-gated; non-x86 stub returns
  `None`.
- `mod dmi` — `cfg(target_os = "linux")`; substring-matcher
  module.
- `mod container` — `cfg(target_os = "linux")`; runs the
  container probe order.
- `mod procfs` — thin wrapper over `/proc` / `/sys` reads,
  parameterized by a root path so tests can point it at a
  fixture directory. Provide a `trait ProcFs` with the
  read-file / path-exists methods, plus a real-fs impl and a
  fixture-fs impl for tests.
- `enum Mode { Any, VmOnly, ContainerOnly }`.
- `enum Virt { None, Vm(VmId), Container(ContainerId) }` with
  `fn id(&self) -> &'static str`.
- `fn detect(mode: Mode, fs: &impl ProcFs) -> io::Result<Virt>`.
- `fn list_ids() -> &'static [&'static str]` driven by a
  single source-of-truth table (constants) so `--list` and
  the matchers cannot drift.

## Error handling

- ENOENT / EACCES on a probe path is **not** an error — that
  signal is just unavailable; move on. Common on FreeBSD,
  macOS, restricted containers.
- Genuine I/O errors (EIO etc.) bubble up as `Err` and exit
  non-zero with a stderr message (`error: <io::Error>`
  matching the style of `xtask/src/bin/tar-archive.rs`'s
  failure path).
- `--debug` distinguishes the three states clearly:
  "skipped, file absent" vs. "read, no match" vs. "matched X".

## Tests

- Table-driven unit tests for the DMI matcher and the
  `/proc/1/environ` parser, using synthetic byte strings —
  no real `/proc` access.
- Unit tests use the injected-root `ProcFs` to point at a
  fixture tree under `xtask/tests/fixtures/detect-virt/`
  with a few representative scenarios (e.g. `kvm-on-amazon-ec2`,
  `docker-on-kvm`, `vanilla-bare-metal`, `xen-dom0`,
  `wsl`). Each fixture is a directory tree mirroring the
  paths the bin reads (`proc/1/environ`,
  `sys/class/dmi/id/sys_vendor`, etc.); commit them as
  small text files.
- Smoke test: `detect(Mode::Any, real_fs)` must not panic
  on the host running the test (assertion: it returns
  `Ok(_)`). Don't assert any particular value — the test
  host varies.
- Skip CPUID-dependent tests on non-x86 (or gate behind a
  feature flag); use `#[cfg(any(target_arch = "x86",
  target_arch = "x86_64"))]` on the test fns.

## Style

- Idiomatic Rust, no `unsafe`, no `unwrap()` / `expect()`
  on the detection path. Tests / dev-deps may use them per
  CONTRIBUTING.md §10.3.
- Top-of-module comment explaining the data source for each
  probe; reference systemd's `src/basic/virt.c` where the
  heuristic comes from there.
- Prefer `&str` / `memchr`-style matching over regex; do not
  add a regex dependency.
- Keep all string ids in a single source-of-truth table
  mapped to enum variants, so `--list` and the matchers can't
  drift.

## Out of scope

Do **not** implement these `systemd-detect-virt` flags:
- `--cvm` (confidential VM detection).
- `--chroot`.
- `--user` / `--private-users`.

If a probe needs a feature flag in `Cargo.toml`, ask in the
structured output instead of inventing one.

<git_rules>
Codex must follow the workspace Git workflow:

- **Do not** run raw `git commit`, `git push`, `git tag`, or
  `cargo publish`. Claude commits via
  `./scripts/commit-all.sh` after reviewing your output;
  Codex itself never invokes commit-all.sh either (the
  codex-guard at the top of the script aborts under any
  Codex ancestor process).
- Leave the working tree dirty for Claude to commit. Do not
  attempt `git stash`, do not amend history, do not
  `git reset`.
- The DCO sign-off + GPG/SSH signature + `Audit-Info:`
  trailer + `Code-stats:` trailer are added by
  `commit-all.sh` at Claude's commit time. You don't touch
  any of that.

See [`CONTRIBUTING.md §4`](../../CONTRIBUTING.md#4-git-workflow)
for the rule and rationale.
</git_rules>

<verification_loop>
Before declaring task_complete, run:

1. `./scripts/xtask.sh --list | grep '^detect-virt$'` — confirm
   the bin is auto-discovered.
2. `./scripts/xtask.sh detect-virt -- --list` — sanity-check
   the help / id-list path runs to completion.
3. `./scripts/xtask.sh detect-virt -- --debug` — eyeball that
   the debug log on stderr matches the source-of-truth probe
   order, and that the final stdout id is plausible for the
   build host.
4. `./scripts/pre-landing.sh --xtask` — fmt + check + clippy
   `-D warnings` + rustdoc + test, gated to xtask only with
   `target-xtask/`. **This is the canonical pre-landing
   check for xtask-only changes.** Do not run raw
   `cargo fmt/check/clippy/test` when this script covers it.

If `pre-landing.sh --xtask` reports failures, fix them and
re-run until clean. Do not declare task_complete with
failing pre-landing.

`./scripts/build-status.sh` is the canonical "is cargo
making progress?" probe — use it instead of inventing
heuristics if a build appears stuck.
</verification_loop>

<missing_context_gating>
If any of the following is unclear after re-reading the
authoritative docs and the existing xtask/src/bin/ siblings:

- the precise systemd-detect-virt id for an edge case the spec
  doesn't enumerate;
- whether a probe should be gated behind a feature flag;
- the right way to express the source-of-truth id table while
  keeping `Mode` / `Virt` enum-driven;

stop and surface the question in the structured output's
`open_questions` section instead of guessing. Don't invent
ids; don't invent flags.
</missing_context_gating>

<completeness_contract>
- Either land all the spec'd functionality (CLI flags, container
  probes 1-8, VM probes 1-7, tests, source-of-truth table,
  `#![forbid(unsafe_code)]`) or surface a clear mid-task halt
  with what is left.
- Do not silently drop probes "for later". If you skip one,
  list it explicitly in the structured output and explain why.
- Tests are not optional. The DMI matcher, the
  `/proc/1/environ` parser, and the smoke test are all in
  scope.
</completeness_contract>

<default_follow_through_policy>
Default behavior is "complete the task end-to-end and run the
verification loop to green". If a snag forces a halt mid-task
(crate-not-found, contradiction with workspace docs,
ambiguous spec), halt cleanly with a structured output
explaining what's blocking — do not paper over it.
</default_follow_through_policy>

<action_safety>
- Read-only scope: this task does not touch any submodule.
  Stay in `xtask/`.
- No deletions of existing xtask bins or tests.
- No edits outside `xtask/Cargo.toml`,
  `xtask/src/bin/detect-virt.rs`, and
  `xtask/tests/fixtures/detect-virt/**`.
- If you find drift in the workspace (a stale doc, a missing
  Cargo.lock entry post-dep-add, etc.), surface it in the
  structured output — don't fix it inline; that's Claude's
  housekeeping lane.
</action_safety>

<structured_output_contract>
At the end of your run, return a structured summary with:

- **summary** — one paragraph: what you built, what worked,
  what you punted on (if anything).
- **touched_files** — bullet list of every file created /
  modified, with one-line role notes.
- **verification_results** — output of each
  `<verification_loop>` step, condensed (last few lines of
  each command, or "ok" / "<error>").
- **id_coverage** — table of which systemd-detect-virt ids
  the implementation can actually produce vs. ids only on
  the `--list` output (e.g. ids that need s390 hardware).
- **open_questions** — anything from
  `<missing_context_gating>` that you stopped on, or
  ambiguities you decided one way but want Claude to ratify.
- **residual_risks** — known limitations (e.g. "the
  `microsoft` Hyper-V matcher relies on the
  `Microsoft Corporation` + `Virtual Machine` combination;
  on bare-metal Microsoft Surface laptops the same vendor
  string appears without the product match — verified
  current behavior matches systemd's").
- **git_state** — branch / HEAD SHA before your run; list
  of files left dirty for Claude to commit; confirmation
  that you did not run `git commit`, `git push`, `git
  tag`, or `cargo publish`.
- **next_round_needed** — boolean. True only if
  scope-truncation forced a halt. If true, list specifically
  what's left.
</structured_output_contract>
</task>
