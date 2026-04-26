# scripts/lib/attach-submodule-branch.sh — sourced helper. Defines
# `attach_submodule_branch()`, which attaches the current submodule's
# HEAD to its tracked branch (read from `.gitmodules` `branch = ...`,
# default `main`) — but only when it can be done without moving the
# working tree or discarding unique branch commits. Otherwise it
# prints one line of context to stderr and leaves HEAD detached.
#
# Why this helper exists:
#
#   `git submodule update --init` checks out the SHA recorded in the
#   parent superproject. That's a raw commit ID, not a ref, so HEAD
#   ends up detached even when `.gitmodules` has `branch = main` set
#   — the `branch` field is only consulted by `git submodule update
#   --remote`. And `update --remote --rebase` silently degrades to a
#   plain checkout when HEAD is already detached, so it does not
#   re-attach the submodule either. Result: every fresh clone leaves
#   every submodule in detached HEAD, which trips
#   `commit-all.sh`'s detached-HEAD guard the moment a contributor
#   touches anything.
#
#   This helper closes the loop: after `update --init` (in
#   `setup.sh`) or after `update --remote --rebase` (in
#   `pull-all.sh`), iterate submodules and attach each to its tracked
#   branch in the safe cases, warn in the unsafe ones.
#
# Expected execution context: inside `git submodule foreach`, where
# `cwd` is the submodule's working tree, `$name` is the submodule's
# registered name, and `$REPO_ROOT` has been exported by the caller
# to the superproject root. The helper validates both.
#
# Safety policy (the reason this is a library rather than a one-line
# `git checkout main`):
#
#   - Already on the tracked branch: no-op.
#   - On a different branch (e.g. a contributor's topic branch):
#     leave alone; print one note. The contributor's intent wins.
#   - Detached, current SHA == local <branch> tip: attach via
#     `git checkout <branch>` (preserves any unpushed commits sitting
#     on the local branch).
#   - Detached, current SHA == origin/<branch> tip, and local
#     <branch> is either missing or an ancestor of origin/<branch>:
#     fast-forward local <branch> to origin's tip and attach via
#     `git checkout -B`. Safe under CONTRIBUTING.md §4.4 because no
#     unique commits are dropped — `git merge-base --is-ancestor`
#     is the explicit guard.
#   - Detached, current SHA matches neither tip: leave detached and
#     warn. The parent superproject is pinning an off-branch commit;
#     auto-attaching would either silently move the working tree off
#     the recorded SHA, or reset the local branch to a non-tip
#     commit. Both wrong; require a human to look.
#   - Detached, local <branch> has unique commits past
#     origin/<branch> and we'd otherwise need to reset it: leave
#     detached and warn. The contributor should push (or
#     `commit-all.sh` + `push-all.sh`) first.
#
# POSIX sh only — see CONTRIBUTING.md §6.

attach_submodule_branch() {
    if [ -z "${REPO_ROOT:-}" ] || [ ! -d "${REPO_ROOT:-}" ]; then
        printf 'attach_submodule_branch: $REPO_ROOT not set or not a directory\n' >&2
        return 1
    fi
    if [ -z "${name:-}" ]; then
        printf 'attach_submodule_branch: $name not set (must run inside `git submodule foreach`)\n' >&2
        return 1
    fi

    # Tracked branch from .gitmodules; default to main when the
    # submodule entry has no `branch` field. Workspace convention
    # tracks `main` for every submodule, but the default keeps the
    # helper usable in any superproject.
    _asb_branch=$(git config -f "$REPO_ROOT/.gitmodules" --get "submodule.$name.branch" 2>/dev/null || true)
    if [ -z "$_asb_branch" ]; then
        _asb_branch=main
    fi

    _asb_current=$(git rev-parse --abbrev-ref HEAD)
    if [ "$_asb_current" = "$_asb_branch" ]; then
        return 0
    fi
    if [ "$_asb_current" != "HEAD" ]; then
        printf 'attach: %s: on branch "%s", tracked is "%s" — leaving alone\n' \
            "$name" "$_asb_current" "$_asb_branch" >&2
        return 0
    fi

    _asb_head=$(git rev-parse HEAD)
    _asb_local=$(git rev-parse --verify --quiet "refs/heads/$_asb_branch" 2>/dev/null || true)
    _asb_remote=$(git rev-parse --verify --quiet "refs/remotes/origin/$_asb_branch" 2>/dev/null || true)

    # Case: HEAD is at the local branch tip — attach directly. This
    # preserves any unpushed commits sitting on the local branch
    # that happened to be the working state when something detached
    # us (e.g. `git submodule update --init` re-checking out the
    # recorded SHA, which equalled the local tip).
    if [ -n "$_asb_local" ] && [ "$_asb_head" = "$_asb_local" ]; then
        git checkout --quiet "$_asb_branch"
        printf 'attach: %s: %s (at %s)\n' \
            "$name" "$_asb_branch" "$(git rev-parse --short HEAD)"
        return 0
    fi

    # Case: HEAD is at origin/<branch> tip — fast-forward local
    # <branch> to origin's tip and attach, but only when no unique
    # commits would be discarded. `git checkout -B` is the atomic
    # "create-or-reset and switch" form; the merge-base guard above
    # makes it safe under §4.4.
    if [ -n "$_asb_remote" ] && [ "$_asb_head" = "$_asb_remote" ]; then
        if [ -z "$_asb_local" ] || \
           git merge-base --is-ancestor "$_asb_local" "$_asb_remote" 2>/dev/null; then
            git checkout --quiet -B "$_asb_branch" "refs/remotes/origin/$_asb_branch"
            printf 'attach: %s: %s (at %s)\n' \
                "$name" "$_asb_branch" "$(git rev-parse --short HEAD)"
            return 0
        fi
        printf 'attach: %s: local "%s" (%s) has commits not on origin/%s (%s); push or rebase first — leaving detached\n' \
            "$name" "$_asb_branch" "$(git rev-parse --short "$_asb_local")" \
            "$_asb_branch" "$(git rev-parse --short "$_asb_remote")" >&2
        return 0
    fi

    # Case: HEAD is at neither tip. Parent superproject is pinning
    # an off-branch commit; auto-attaching would either move the
    # working tree off the recorded SHA or reset the local branch
    # to a non-tip commit. Both wrong; leave a human to resolve.
    if [ -z "$_asb_remote" ]; then
        printf 'attach: %s: origin/%s not found — leaving detached at %s\n' \
            "$name" "$_asb_branch" "$(git rev-parse --short HEAD)" >&2
    else
        printf 'attach: %s: HEAD (%s) does not match origin/%s (%s) — leaving detached\n' \
            "$name" "$(git rev-parse --short HEAD)" \
            "$_asb_branch" "$(git rev-parse --short "$_asb_remote")" >&2
    fi
}
