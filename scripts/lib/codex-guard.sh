# scripts/lib/codex-guard.sh — abort if running inside a Codex process tree.
#
# Codex must never commit to Git. It should leave the tree dirty for
# Claude to review and commit. This guard walks the ancestor process
# chain and aborts if any process name matches *codex* (case-
# insensitive). Sourced by commit-all.sh and .githooks/pre-commit.
#
# POSIX sh — see CONTRIBUTING.md §6.

_codex_guard_check() {
    _cg_pid=$$
    while [ "$_cg_pid" -gt 1 ]; do
        _cg_cmd=$(ps -o comm= -p "$_cg_pid" 2>/dev/null) || break
        case $(printf '%s' "$_cg_cmd" | tr '[:upper:]' '[:lower:]') in
            *codex*)
                printf '%s!!! Commit blocked: detected Codex ancestor process (pid %s: %s).%s\n' \
                    "${C_ERR:-}" "$_cg_pid" "$_cg_cmd" "${C_RESET:-}" >&2
                printf '    Codex must not commit to Git. Leave the tree dirty for Claude.\n' >&2
                exit 1
                ;;
        esac
        _cg_ppid=$(ps -o ppid= -p "$_cg_pid" 2>/dev/null) || break
        _cg_pid=$(printf '%s' "$_cg_ppid" | tr -d ' ')
        case "$_cg_pid" in
            ''|0) break ;;
        esac
    done
    unset _cg_pid _cg_cmd _cg_ppid
}

_codex_guard_check
