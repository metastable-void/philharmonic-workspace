#!/bin/sh
# scripts/lib/cargo-noise-filter.sh — strip cargo's repeated
# "non root package profile" warning trio from cargo's output so
# colored build logs stay readable.
#
# Submodule member crates intentionally carry `[profile.*]`
# sections in their own `Cargo.toml` so they build independently
# of the parent workspace (CONTRIBUTING.md §3.1 "Submodules must
# build independently"). When cargo loads the workspace it emits a
# 3-line warning per such member:
#
#   warning: profiles for the non root package will be ignored, ...
#   package:   /<workspace-path>/<crate>/Cargo.toml
#   workspace: /<workspace-path>/Cargo.toml
#
# Across the ~25 member crates that's ~75 lines of noise per
# build. Hide them with this filter rather than removing the
# `[profile.*]` blocks (which would silence the warning at the
# cost of breaking standalone submodule builds).
#
# Two entry points:
#
#   `run_with_cargo_noise_filter <argv>...`
#       Run a command with stdout+stderr merged through the noise
#       filter. Exit status of the command is preserved as the
#       function's return value (POSIX-only; no `pipefail`).
#       Use this in build / lint / test scripts where the entire
#       output stream is diagnostic prose for humans.
#
#   `run_with_cargo_noise_filter_stderr <argv>...`
#       Run a command with only stderr filtered; stdout passes
#       through unchanged. Use this in tool wrappers whose stdout
#       is captured by callers (e.g. `xtask.sh`'s
#       `sysres=$(./scripts/xtask.sh system-resources)` capture in
#       `print-audit-info.sh` — the cargo noise lands on stderr
#       and would pollute the captured value if merged).
#
# `filter_cargo_noise` is exposed too for direct piping (`cmd |
# filter_cargo_noise`), but exit-status preservation across the
# pipe is the caller's responsibility — without `pipefail` the
# shell only sees grep's status.
#
# Patterns target the workspace's actual cargo output exactly:
# the ANSI-colored `warning:` prefix is followed by plain ASCII
# we can match unambiguously, and the `package:`/`workspace:`
# lines are emitted without color so `^…:[[:space:]]+/`
# anchoring is safe. `LC_ALL=C` forces byte-mode matching so the
# filter doesn't choke on terminal-control bytes mid-stream.
#
# Pair with `cargo --color=always` at the call site so colored
# output survives the pipe (cargo otherwise auto-disables color
# when stdout isn't a TTY).
#
# POSIX sh — see CONTRIBUTING.md §6. Sourced by build/test/lint
# scripts that invoke cargo.

# Drop the noise trio from stdin; pass everything else through.
# Treat grep's "no match" exit (1) as success — that just means
# the entire input was filtered away (rare but possible) or that
# the input was empty (e.g. cargo fmt --check on a clean tree
# with no warnings to emit). Real grep errors are >= 2 and
# propagate.
#
# `|| _cnf_grep_rc=$?` is required to dodge `set -e` in the
# caller: under errexit, grep's exit-1 (no-match) would abort
# the function *before* we got a chance to capture and remap it.
# The `|| ...` chain is a POSIX-blessed errexit-suppression
# context, so the assignment runs regardless of grep's status.
filter_cargo_noise() {
    _cnf_grep_rc=0
    LC_ALL=C grep -Ev \
        -e 'profiles for the non root package will be ignored' \
        -e '^package:[[:space:]]+.*Cargo\.toml$' \
        -e '^workspace:[[:space:]]+.*Cargo\.toml$' \
        || _cnf_grep_rc=$?
    if [ "$_cnf_grep_rc" -le 1 ]; then
        return 0
    else
        return "$_cnf_grep_rc"
    fi
}

# Run "$@" with stdout+stderr merged through filter_cargo_noise,
# preserving the command's exit status. Streams output (no
# completion-buffering) by writing the exit status to a temp
# file from inside a subshell that runs alongside the pipeline.
run_with_cargo_noise_filter() {
    _cnf_status_file=$(./scripts/mktemp.sh cnf-status)
    # shellcheck disable=SC2064
    # Expand $_cnf_status_file *now*, not at trap time, so a
    # later overwrite of the var by another invocation can't
    # redirect this trap to the wrong file.
    trap "rm -f \"$_cnf_status_file\"" EXIT INT HUP TERM
    (
        # Local set +e so a non-zero exit from "$@" doesn't
        # abort the subshell before we record its status.
        set +e
        "$@"
        echo $? > "$_cnf_status_file"
    ) 2>&1 | filter_cargo_noise
    _cnf_rc=$(cat "$_cnf_status_file" 2>/dev/null || echo 1)
    rm -f "$_cnf_status_file"
    trap - EXIT INT HUP TERM
    return "${_cnf_rc:-1}"
}

# Run "$@" with only stderr filtered; stdout passes through
# unchanged. Streams via a FIFO so the filter consumes stderr
# concurrently with the command. Exit status preserved.
run_with_cargo_noise_filter_stderr() {
    _cnf_fifo=$(./scripts/mktemp.sh cnf-stderr)
    rm -f "$_cnf_fifo"
    mkfifo -m 600 "$_cnf_fifo"
    # shellcheck disable=SC2064
    trap "rm -f \"$_cnf_fifo\"" EXIT INT HUP TERM

    filter_cargo_noise <"$_cnf_fifo" >&2 &
    _cnf_pid=$!

    _cnf_rc=0
    "$@" 2>"$_cnf_fifo" || _cnf_rc=$?

    # Drain the filter; it sees EOF when the cargo process
    # closes the FIFO writer side on exit.
    wait "$_cnf_pid" 2>/dev/null || :
    rm -f "$_cnf_fifo"
    trap - EXIT INT HUP TERM
    return "$_cnf_rc"
}
