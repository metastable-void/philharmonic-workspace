# scripts/lib/script-help.sh — shared `-h` / `--help` handler for
# sibling scripts under `scripts/`.
#
# Expected usage (immediately after `set -eu`, before any other
# argument parsing):
#
#     #!/bin/sh
#     set -eu
#     . "$(dirname -- "$0")/lib/script-help.sh"
#     script_help_handle "$@"
#
# `script_help_handle` inspects the first positional arg. If it
# is `-h` or `--help`, the function prints the calling script's
# top-of-file comment block — line 2 through the first blank
# line, with one leading `# ` (or `#`) stripped from each line —
# and exits 0. Otherwise the function returns without consuming
# any args, so the caller's normal arg parsing proceeds
# unchanged.
#
# Convention: every `scripts/*.sh` that can be called in isolation
# documents itself in a header comment block of the shape:
#
#     #!/bin/sh
#     # scripts/<name>.sh — one-line summary.
#     #
#     # Usage:
#     #   ./scripts/<name>.sh [flags]
#     #
#     # …extended description…
#
#     set -eu
#
# The blank line between the comment block and `set -eu`
# terminates the `sed -n '2,/^$/p'` range, so `script_help_handle`
# emits exactly the header without leaking into the code below.
#
# Implementation note: inside a POSIX-sourced helper, `$0` is the
# caller's path (true under sh / dash / bash / zsh), so the
# `sed "$0"` invocation reads the *calling* script's source — the
# helper itself never appears in `--help` output.

script_help_handle() {
    case "${1:-}" in
        -h|--help)
            sed -n '2,/^$/p' "$0" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
    esac
}
