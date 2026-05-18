#!/bin/sh
# Commit pending changes across the workspace.
#
# Walks each submodule, commits any dirty tree there, then commits
# the parent (which includes the bumped submodule pointers).
#
# Every commit is signed off (`-s`, DCO trailer) *and*
# cryptographically signed (`-S`, GPG or SSH). Signing is
# enforced two ways: we pass `-S` to `git commit` (so a missing
# signing key aborts the commit before it lands), and we verify
# the resulting HEAD with `git log --format=%G?` — if the commit
# somehow lacks a signature, we roll it back and fail. See
# docs/design/13-conventions.md §Git workflow.
#
# Safety: refuses to commit in a submodule that's in detached HEAD
# state if it has changes to commit — that commit would be an
# orphan the next time the submodule is checked out. Also refuses
# the parent commit if .claude/settings.json contains any
# reference to a guarded script (`scripts/commit-all.sh` or
# `scripts/publish-crate.sh`) — a permission entry for either
# would let agents commit or publish without human approval.
#
# Side effect: before every parent commit (real, not --dry-run),
# runs `./scripts/update-stats-graph.sh` to refresh
# `docs/stats.svg` so HEAD always carries an up-to-date growth
# chart. Failure aborts the commit so a stale chart isn't shipped
# silently; pass nothing for an opt-out — the regen is part of the
# parent-commit contract. Submodule walks are unaffected.
#
# Usage:
#   scripts/commit-all.sh [--anonymize] [--parent-only] [--dry-run]
#                          [--exclude <path>]...
#                          [--message-file <path> | message]
#
# --parent-only: skip the submodule walk. Use this when the parent
#   has its own pending work (e.g. docs, scripts) that should land
#   independently of whatever the submodules are currently doing.
#   Handy when submodules hold in-progress Codex work that shouldn't
#   be committed yet.
# --dry-run: walk the submodules and the parent and print, per repo,
#   the file list that `git add -A` would sweep into the commit
#   (using `git status --short`). Does not stage, commit, sign,
#   create the temp message file, or run the .claude/settings.json
#   guard — purely a read-only preview. Recommended pre-flight
#   check whenever the working tree's contents are uncertain
#   (e.g. after a Codex dispatch or other batch of edits) so the
#   real commit-all.sh invocation doesn't sweep in unintended files.
# --exclude <path>: hold a parent-repo path back from the commit.
#   Repeatable — pass `--exclude` once per path. The path must be
#   workspace-root-relative (e.g. `Cargo.lock`,
#   `philharmonic/webui/dist/main.js`) and must not contain
#   whitespace. After `git add -A` runs, each excluded path is
#   unstaged with `git reset HEAD -- <path>` so the working-tree
#   change remains dirty for a later commit. Applies only to the
#   parent commit phase — the submodule walk is unaffected. Useful
#   when a side-effect file (typically `Cargo.lock` after a
#   submodule version bump) needs to land with the corresponding
#   submodule commit, not in a parent-only doc/script commit
#   landing first. Under `--dry-run`, the excluded paths are
#   listed alongside the would-be-committed status so the
#   preview matches the planned scope.
# --message-file <path>: read the commit message body from <path>
#   (an existing readable file) instead of the positional [message]
#   argument. Pass `-` as the path to read the message from stdin.
#   This is the canonical form for any message that contains
#   backticks, `$(...)` command substitutions, `$VAR` references,
#   or `!` history-expansion — the stdin / file path bypasses
#   bash's quote-removal pass entirely, so technical tokens in the
#   body land in the commit verbatim instead of being silently
#   expanded by the shell (the incident in commit `a5833d5` lost
#   ≈ 8 backticked tokens to that failure mode). Mutually
#   exclusive with the positional [message] argument; if both are
#   provided the script aborts. An empty file or empty stdin is
#   treated as an error — a meaningful subject line is required.
#   Canonical heredoc form:
#       ./scripts/commit-all.sh --message-file - <<'EOF'
#       subject line ≤ 72 chars
#
#       body wrapped at ≈ 72 cols, with backticked `tokens`
#       and `$()` substitutions preserved verbatim.
#       EOF
#   The single-quoted heredoc delimiter (`<<'EOF'`) is still
#   load-bearing — it suppresses shell expansion *inside* the
#   heredoc body before stdin is written. A bare `<<EOF`
#   (unquoted) would still expand backticks and `$VAR`.
# Message defaults to "updates" (positional form only) and is
# unused under --dry-run.
#
# Scope: the parent commit uses `git add -A` before `git commit`,
# so ALL dirty parent files get swept in. Pre-staging a subset
# with `git add` does NOT scope the commit — selective staging is
# meaningless here. To make a single-purpose commit when the parent
# has unrelated dirty files, clean them out of the tree first
# (move to /tmp, or commit them separately in a prior --parent-only
# invocation), then run commit-all.sh. See
# docs/design/13-conventions.md §Git workflow.
#
# POSIX sh only — see docs/design/13-conventions.md §Shell scripts.

set -eu

. "$(dirname -- "$0")/lib/colors.sh"
. "$(dirname -- "$0")/lib/workspace-cd.sh"
. "$(dirname -- "$0")/lib/codex-guard.sh"

parent_only=0
anonymize=0
dry_run=0
# Space-separated list of paths to hold back from the parent commit
# (see --exclude in the header). Empty means no exclusions. Paths
# are validated whitespace-free below so the unquoted-list `for`
# split is unambiguous.
parent_excludes=
# Path supplied via --message-file. Empty means the flag wasn't
# given. `-` means "read from stdin". Anything else is a regular
# file path validated below.
msg_file_arg=
while [ $# -gt 0 ]; do
    case "$1" in
        --parent-only) parent_only=1; shift ;;
        --anonymize) anonymize=1; shift ;;
        --dry-run) dry_run=1; shift ;;
        --exclude)
            if [ $# -lt 2 ]; then
                echo "commit-all.sh: --exclude requires a path argument" >&2
                exit 2
            fi
            shift
            case "$1" in
                ''|*' '*|*"$(printf '\t')"*)
                    printf 'commit-all.sh: --exclude path must be non-empty and whitespace-free: %s\n' "$1" >&2
                    exit 2
                    ;;
            esac
            if [ -z "$parent_excludes" ]; then
                parent_excludes="$1"
            else
                parent_excludes="$parent_excludes $1"
            fi
            shift
            ;;
        --message-file)
            if [ $# -lt 2 ]; then
                echo "commit-all.sh: --message-file requires a path argument ('-' for stdin)" >&2
                exit 2
            fi
            shift
            if [ -n "$msg_file_arg" ]; then
                echo "commit-all.sh: --message-file given more than once" >&2
                exit 2
            fi
            msg_file_arg=$1
            shift
            ;;
        --help)
            echo "Usage: $0 [--anonymize] [--parent-only] [--dry-run] [--exclude <path>]... [--message-file <path> | message]"
            exit
            ;;
        --)                           shift; break ;;
        -*) printf 'unknown flag: %s\n' "$1" >&2; exit 2 ;;
        *) break ;;
    esac
done

# Resolve the commit message. Three sources, mutually exclusive:
#   1. --message-file <path>   (stdin if <path> = '-')
#   2. positional [message]    (legacy / interactive shape)
#   3. fall back to "updates"  (positional default)
# Reading must finish before any later step that itself touches
# stdin (the `git submodule foreach` loop below does not, but
# keeping the read up front is safer for future-proofing). `$()`
# strips trailing newlines, which is what the existing
# `printf '%s\n\n' "$msg"` writer expects.
if [ -n "$msg_file_arg" ]; then
    if [ "$#" -gt 0 ]; then
        echo "commit-all.sh: --message-file and positional [message] are mutually exclusive" >&2
        exit 2
    fi
    if [ "$msg_file_arg" = "-" ]; then
        msg=$(cat)
    else
        if [ ! -f "$msg_file_arg" ] || [ ! -r "$msg_file_arg" ]; then
            printf 'commit-all.sh: --message-file path not readable: %s\n' "$msg_file_arg" >&2
            exit 2
        fi
        msg=$(cat -- "$msg_file_arg")
    fi
    if [ -z "$msg" ]; then
        echo "commit-all.sh: --message-file produced an empty message; a meaningful subject is required" >&2
        exit 2
    fi
else
    msg="${1:-updates}"
fi

# Skip the message-file / audit / Code-stats setup under --dry-run:
# nothing is going to consume it, and computing the audit line hits
# the network for IP/geo lookup which is wasted work for a preview.
if [ "$dry_run" -eq 0 ]; then
    if [ "${anonymize}" -eq 1 ] ; then
        audit_flags="--anonymize"
    else
        audit_flags=
    fi

    # Stash the message in a temp file so we don't have to escape it
    # through `git submodule foreach`'s nested shell.
    msgfile="$("$(dirname "$0")"/mktemp.sh commit-msg)"
    trap 'rm -f "$msgfile"' EXIT INT HUP TERM
    printf '%s\n\n' "$msg" > "$msgfile"
    printf 'Audit-Info: %s\n' "$( "$(dirname "$0")"/print-audit-info.sh $audit_flags )" >> "$msgfile"
    printf 'Code-stats: %s\n' "$( "$(dirname "$0")"/stats.sh )" >> "$msgfile"
    export MSG_FILE="$msgfile"
fi

# Signal that we call Git through a proper wrapper.
WORKSPACE_GIT_WRAPPER=1
export WORKSPACE_GIT_WRAPPER

# Make the dry-run flag visible inside the `git submodule foreach`
# subshell. Default to 0 so the foreach script can compare the
# value safely under `set -u`.
DRY_RUN="$dry_run"
export DRY_RUN

# Commit each submodule's changes (if any), unless --parent-only.
if [ "$parent_only" -eq 0 ]; then
    git submodule foreach --quiet '
branch=$(git rev-parse --abbrev-ref HEAD)
if ! git diff --quiet || ! git diff --cached --quiet || [ -n "$(git ls-files --others --exclude-standard)" ]; then
    if [ "$branch" = "HEAD" ]; then
        printf "%s!!! $name is in detached HEAD with uncommitted changes.%s\n" "$C_ERR" "$C_RESET" >&2
        echo "    Refusing to commit (would create an orphan)." >&2
        echo "    Checkout a branch inside the submodule and re-run." >&2
        exit 1
    fi
    if [ "${DRY_RUN:-0}" = "1" ]; then
        printf "%s=== [DRY-RUN] $name (branch: $branch) — would commit:%s\n" "$C_HEADER" "$C_RESET"
        git status --short
        printf "\n"
    else
        printf "%s=== committing in $name (branch: $branch) ===%s\n" "$C_HEADER" "$C_RESET"
        git add -A
        # -S forces GPG/SSH signing; commit aborts here if no key.
        git commit -s -S -F "$MSG_FILE"
        # Defence in depth: verify the commit actually carries a
        # signature. %G? returns "N" for unsigned commits.
        sig=$(git log -n 1 --format=%G? HEAD)
        if [ "$sig" = "N" ]; then
            printf "%s!!! $name: HEAD $(git rev-parse --short HEAD) has no signature.%s\n" "$C_ERR" "$C_RESET" >&2
            echo "    Rolling back with git reset --soft HEAD~1." >&2
            git reset --soft HEAD~1
            exit 1
        fi
    fi
else
    printf "%s=== $name clean ===%s\n" "$C_DIM" "$C_RESET"
fi
'
else
    printf '%s=== --parent-only: skipping submodules ===%s\n' "$C_DIM" "$C_RESET"
fi

# Safety: refuse the parent commit if .claude/settings.json
# contains any reference to a guarded script. A permission entry
# for either `scripts/commit-all.sh` or `scripts/publish-crate.sh`
# (e.g. `Bash(./scripts/commit-all.sh *)`) would let an agent run
# that script without human approval, defeating the human-in-the-
# loop gate this workflow relies on — commit-all.sh grants the
# ability to produce signed commits, publish-crate.sh grants the
# ability to publish to crates.io and mint signed release tags.
# The check is a verbatim fixed-string match so any form of the
# allow entry is caught. To restore the ability to commit, remove
# the offending entry from .claude/settings.json.
# Refresh docs/stats.svg from the workspace's commit-stats
# history so HEAD always carries an up-to-date growth chart.
# Skipped under --dry-run (read-only preview). The SVG renders
# only commits whose `Code-stats:` trailer is parseable; if
# nothing has changed since the last regen, `git add -A` below
# sees an unchanged file and the parent commit doesn't grow.
# Failure here aborts the commit so problems surface rather than
# producing a stale chart silently.
if [ "$dry_run" -eq 0 ] && [ -x ./scripts/update-stats-graph.sh ]; then
    printf '%s=== refreshing docs/stats.svg via update-stats-graph.sh ===%s\n' \
        "$C_DIM" "$C_RESET"
    ./scripts/update-stats-graph.sh
fi

# Skip the settings.json guard under --dry-run: nothing is being
# committed, so the gate has no work to do. Run it normally on the
# real-commit path.
if [ "$dry_run" -eq 0 ]; then
    settings_file=".claude/settings.json"
    if [ -f "$settings_file" ]; then
        # POSIX `for` over an unquoted space-separated list —
        # consistent with `crates=$*; for c in $crates` elsewhere in
        # the scripts. No entry contains whitespace, so splitting on
        # IFS is safe.
        for guarded in scripts/commit-all.sh scripts/publish-crate.sh; do
            if grep -F -q "$guarded" "$settings_file"; then
                echo "!!! $settings_file references $guarded." >&2
                echo "    Refusing to commit the parent — a permission entry for" >&2
                echo "    this script would let agents run it without human" >&2
                echo "    approval. Remove the entry from $settings_file and" >&2
                echo "    re-run." >&2
                exit 1
            fi
        done
    fi
fi

# Commit parent's changes (including bumped submodule pointers).
if ! git diff --quiet || ! git diff --cached --quiet || [ -n "$(git ls-files --others --exclude-standard)" ]; then
    if [ "$dry_run" -eq 1 ]; then
        printf '%s=== [DRY-RUN] parent — would commit:%s\n' "$C_HEADER" "$C_RESET"
        git status --short
        if [ -n "$parent_excludes" ]; then
            printf '%s    --exclude (held back from this commit):%s\n' "$C_DIM" "$C_RESET"
            for excluded in $parent_excludes; do
                printf '       %s\n' "$excluded"
            done
        fi
    else
        printf '%s=== committing in parent ===%s\n' "$C_HEADER" "$C_RESET"
        git add -A
        # Unstage any --exclude paths so they remain dirty in the
        # working tree but don't enter this commit. POSIX `for` over
        # the unquoted whitespace-free list (validated above).
        # `git reset HEAD -- <path>` is silent when the path isn't
        # currently staged, so unknown / clean paths are no-ops.
        if [ -n "$parent_excludes" ]; then
            for excluded in $parent_excludes; do
                git reset -q HEAD -- "$excluded"
            done
        fi
        # If --exclude unstaged everything that was about to land,
        # bail rather than producing an empty commit attempt. After
        # `git add -A` plus the resets above, the index-vs-HEAD diff
        # captures exactly what would be committed.
        if [ -n "$parent_excludes" ] && git diff --cached --quiet; then
            printf '%s!!! parent: --exclude removed every staged change; nothing to commit.%s\n' \
                "$C_ERR" "$C_RESET" >&2
            echo "    Re-run without --exclude or with a smaller exclude set." >&2
            exit 1
        fi
        # -S forces GPG/SSH signing; commit aborts here if no key.
        git commit -s -S -F "$msgfile"
        sig=$(git log -n 1 --format=%G? HEAD)
        if [ "$sig" = "N" ]; then
            printf '%s!!! parent: HEAD %s has no signature.%s\n' \
                "$C_ERR" "$(git rev-parse --short HEAD)" "$C_RESET" >&2
            echo "    Rolling back with git reset --soft HEAD~1." >&2
            git reset --soft HEAD~1
            exit 1
        fi
    fi
else
    printf '%s=== parent clean ===%s\n' "$C_DIM" "$C_RESET"
fi
