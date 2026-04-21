# scripts/lib/workspace-members.sh — sourced helper.
#
# Sets the shell variable `workspace_members` to a newline-
# separated list of workspace member paths, parsed from the root
# `Cargo.toml`. Both submodule-backed members (the majority) and
# in-tree members (e.g. `xtask`) are included — the `[workspace]
# members` list is the canonical source.
#
# Source (not execute) from a caller that has already sourced
# `scripts/lib/workspace-cd.sh` (so CWD is the workspace root).
#
# Implementation: awk-parses the `members = [ ... ]` block,
# extracting the first quoted string on each line inside the
# block. Relies on the convention of one member per line in the
# root `Cargo.toml`. TOML parsing-by-awk is fragile by nature; if
# the format diverges from that layout, move this to a Rust bin
# in `xtask/` (preferred per `docs/design/13-conventions.md
# §In-tree workspace tooling`).
#
# POSIX sh only.

workspace_members=$(
    awk '
        /^members = \[/ { in_m = 1; next }
        in_m && /^\]/ { in_m = 0; next }
        in_m && match($0, /"[^"]+"/) {
            print substr($0, RSTART + 1, RLENGTH - 2)
        }
    ' Cargo.toml
)
