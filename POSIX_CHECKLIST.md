# Non-POSIX Shell Checklist

A reference of common non-POSIX constructs to avoid in shell scripts, based on
**POSIX.1-2024** (IEEE Std 1003.1-2024). Use this as a review checklist when
writing or auditing `/bin/sh` scripts.

Everything listed here is **non-portable** and should either be avoided, or
handled as a marked exception (see the bottom of this file).

---

## Shebang

- [ ] `#!/bin/bash`, `#!/usr/bin/env bash` — bash-specific
- [ ] `#!/bin/ksh`, `#!/bin/zsh` — shell-specific
- [ ] `#!/usr/bin/env sh` — usable, but `#!/bin/sh` is the standard form

Use `#!/bin/sh` exclusively.

---

## Shell syntax and keywords

### Conditionals and tests

- [ ] `[[ ... ]]` — bash/ksh/zsh. Use `[ ... ]`.
- [ ] `==` inside `[ ]` — use `=` for string equality.
- [ ] `<` / `>` inside `[ ]` for string comparison — not POSIX.
- [ ] `=~` regex match — bash only. Use `expr` or `grep`/`awk`.
- [ ] `[ expr1 -a expr2 ]` / `[ expr1 -o expr2 ]` — removed for ambiguity.
      Use `[ expr1 ] && [ expr2 ]` and `[ expr1 ] || [ expr2 ]`.

### Arithmetic

- [ ] `((expr))` — bash/ksh. Use `: $((expr))` or `$((expr))` in assignment.
- [ ] `let x=1+2` — bash/ksh. Use `x=$((1+2))`.

### Function definition

- [ ] `function name { ... }` — ksh/bash keyword form. Use `name() { ... }`.
- [ ] `function name() { ... }` — mixed form, not POSIX. Use `name() { ... }`.
- [ ] `local var` — not POSIX. Use subshells, unique names, or prefix conventions.

### Variable expansion

- [ ] `${var,,}` / `${var^^}` — case conversion (bash 4+). Use `tr`.
- [ ] `${var,}` / `${var^}` — single-char case conversion. Use `tr`.
- [ ] `${var//pattern/replacement}` — bash pattern substitution. Use `sed`.
- [ ] `${var:offset:length}` — bash substring. Use `cut`, `expr`, or `printf`.
- [ ] `${!var}` — indirect expansion. Use `eval` carefully.
- [ ] `${#array[@]}`, `${array[@]}` — arrays. Not POSIX at all; rework logic.
- [ ] `${var@Q}`, `${var@E}`, etc. — bash transformations.

### Strings

- [ ] `$'...'` — C-style escape sequences (bash/ksh/zsh). Use `printf`.
- [ ] `$"..."` — locale translation (bash). Not POSIX.

### Command substitution

- [ ] `` `...` `` — backticks. Works but discouraged; use `$(...)`.
      (Technically POSIX but harder to nest and quote.)

### Process substitution and special redirection

- [ ] `<(command)` / `>(command)` — bash/zsh process substitution.
      Use named pipes (`mkfifo`) or temp files.
- [ ] `&>file` — bash shortcut for `>file 2>&1`. Use the explicit form.
- [ ] `&>>file` — bash append form. Use `>>file 2>&1`.
- [ ] `|&` — bash pipe-stderr shortcut. Use `2>&1 |`.
- [ ] `<<<"string"` — here-string (bash/ksh/zsh). Use here-doc or `printf ... |`.

### Expansion

- [ ] `{a,b,c}` — brace expansion. Write out the values.
- [ ] `{1..10}` — numeric brace expansion. Use a `while` loop.
- [ ] `{a..z}` — character brace expansion. Write out or use `awk`.

### Control flow

- [ ] `source file` — bash/ksh. Use `.` (dot).
- [ ] `pushd` / `popd` / `dirs` — bash directory stack. Use explicit `cd`.
- [ ] `coproc` — bash coprocesses. Use fifos.

### Builtins with non-POSIX options

- [ ] `echo -e`, `echo -n`, `echo -E` — behavior undefined in POSIX.
      Use `printf` exclusively.
- [ ] `read -p prompt` — bash prompt. Use `printf '%s' prompt >&2; read`.
- [ ] `read -s` — silent (password). Use `stty -echo; read; stty echo`.
- [ ] `read -t timeout` — bash timeout. Not portable.
- [ ] `read -a array` — arrays. Not POSIX.
- [ ] `read -d delim` — custom delimiter (bash). Not POSIX.
- [ ] `read` without `-r` — backslash interpretation is surprising; always use `-r`.
- [ ] `declare`, `typeset` — not POSIX (typeset is ksh).
- [ ] `trap ... ERR`, `trap ... DEBUG`, `trap ... RETURN` — bash-only signals.
      POSIX trap signals: `EXIT`, `HUP`, `INT`, `QUIT`, `TERM`, etc.
- [ ] `getopt` — GNU-enhanced external command. Use `getopts` (POSIX builtin).

### Options

- [ ] `set -o pipefail` — POSIX.1-2024 adopted this; older shells lack it.
      Acceptable under the policy but test on your target shells.
- [ ] `shopt` — bash builtin. Not POSIX.

---

## File-test and string-test operators

All listed file-test primaries in `[ ]` *are* POSIX (`-e`, `-f`, `-d`, `-h`,
`-L`, `-p`, `-S`, `-b`, `-c`, `-r`, `-w`, `-x`, `-s`, `-g`, `-u`, `-k`, `-t`,
`-nt`, `-ot`, `-ef`). The ones below are not:

- [ ] `-v varname` — checks if variable is set (bash). Use `${var+x}` test.
- [ ] `-N file` — file has been modified since last read (bash).
- [ ] `-G file`, `-O file` — ownership by effective gid/uid (bash).

---

## External utilities — GNU extensions and non-POSIX tools

### `find`

- [ ] `-maxdepth N`, `-mindepth N` — not POSIX. Use `-prune` manually.
- [ ] `-delete` — GNU. Use `-exec rm {} +`.
- [ ] `-printf fmt` — GNU. Use `-exec` with a format command.
- [ ] `-regex`, `-iregex` — GNU. Use globs + `grep`.
- [ ] `-iname` — GNU (BSD has it too). Not POSIX.
- [ ] `-readable`, `-writable`, `-executable` — GNU. Use `-perm`.
- [ ] `-empty` — GNU. Use `-size 0`.
- [ ] `-not` — GNU synonym for `!`. Use `!`.

### `grep`

- [ ] `-P` — PCRE (GNU). Use basic or extended regex.
- [ ] `-z` — null-delimited (GNU). Process with `tr` first.
- [ ] `-r`, `-R` — recursive (GNU and BSD, but not POSIX). Use `find ... -exec grep`.
- [ ] `-I` — ignore binary (GNU). No POSIX equivalent.
- [ ] `--color` — coloring flag is universal but not POSIX.

Note: `grep -o` is POSIX as of 2024.

### `sed`

- [ ] `-i` / `-i ''` — in-place editing. GNU and BSD diverge; neither is POSIX.
      Write to a temp file and `mv`.
- [ ] `-z` — null-delimited (GNU).
- [ ] `\b`, `\w`, `\s` — GNU/Perl character classes. Use `[[:space:]]` etc.
- [ ] `/regex/I` — case-insensitive (GNU).

Note: `sed -E` (extended regex) is POSIX as of 2024.

### `awk`

- [ ] `gensub()` — gawk extension. Use `gsub()` + capture-by-position tricks.
- [ ] `systime()`, `strftime()`, `mktime()` — gawk time functions.
- [ ] `asort()`, `asorti()` — gawk array sort.
- [ ] `@load`, `@include` — gawk dynamic libraries.
- [ ] `RT` (record terminator) — gawk.
- [ ] `--posix` flag is *encouraged* for testing under gawk.

POSIX awk has: `split`, `sub`, `gsub`, `match`, `index`, `length`, `substr`,
`sprintf`, `tolower`, `toupper`, `srand`, `rand`, `exit`, `next`, `getline`,
`printf`.

### `xargs`

- [ ] `-P N` — parallel (GNU/BSD). No POSIX alternative; use a loop with `&`.
- [ ] `-d delim` — custom delimiter (GNU). Use `tr` to preprocess.
- [ ] `-r` / `--no-run-if-empty` — GNU. Filter empty input explicitly.

Note: `xargs -0` and `find -print0` are POSIX as of 2024.

### File manipulation

- [ ] `cp -a` — archive mode (GNU). Use `cp -pPR`.
- [ ] `cp -r` — POSIX requires `-R`. Some systems accept `-r`; use `-R`.
- [ ] `cp --reflink` — GNU btrfs/xfs optimization.
- [ ] `ln -r` / `--relative` — GNU. Compute relative path manually.
- [ ] `ln -T` — GNU "no-deref-target". Use careful ordering.
- [ ] `mv -t dir` — target-first (GNU).
- [ ] `rm -I` — prompt before many (GNU).
- [ ] `mkdir -v` — verbose (GNU).
- [ ] `install` — not POSIX (it's a BSD tool that GNU also ships).

### Path / resolution

- [ ] `readlink` without arguments — the utility itself is POSIX as of 2024
      (basic mode), but `-f`, `-e`, `-m` flags semantics differ between
      GNU and BSD.
- [ ] `realpath` — now POSIX.1-2024, but `--relative-to` is not in the standard.
      Use `python3` / `perl` or write the algorithm.
- [ ] `basename` / `dirname` — both POSIX, but chaining them is often slower
      than shell parameter expansion (`${var##*/}`, `${var%/*}`).

### Filesystem inspection

- [ ] `stat` — entirely non-POSIX. Format strings differ on every system.
      Parse `ls -l` output or accept the dependency.
- [ ] `du -h`, `df -h` — human-readable (GNU/BSD). POSIX provides `-k` only.
- [ ] `ls --color`, `ls -F` — `-F` is POSIX, `--color` is not.

### Text processing

- [ ] `seq` — not POSIX. Use `while`/`until` with arithmetic, or `awk 'BEGIN{for(...)}'`.
- [ ] `tac` — GNU reverse-cat. Use `tail -r` (BSD), `awk`, or `sed '1!G;h;$!d'`.
- [ ] `rev` — reverse each line (util-linux). Use `awk`.
- [ ] `shuf` — GNU random shuffle. Use `awk` with `rand()`.
- [ ] `sort -R` / `--random-sort` — GNU.
- [ ] `sort -V` / `--version-sort` — GNU/BSD but not POSIX.
- [ ] `sort -h` / `--human-numeric-sort` — GNU.
- [ ] `uniq -D`, `-u` behavior differences — check spec.
- [ ] `column` — util-linux, not POSIX.
- [ ] `paste -d '\0'` — delimiter interpretation varies; careful.
- [ ] `tr -s '[:cntrl:]'` style — class handling is locale-sensitive; use `LC_ALL=C`.

### Binary / encoding

- [ ] `hexdump` — BSD-origin, not POSIX. Use `od -An -tx1`.
- [ ] `xxd` — vim-shipped, not POSIX.
- [ ] `base64` — not POSIX. Use `openssl base64`, `uuencode -m`, or Python.
- [ ] `md5sum`, `sha1sum`, `sha256sum` — GNU coreutils.
      BSD uses `md5`, `sha1`, `sha256` with different output format.
      POSIX has only `cksum` (CRC, not cryptographic).
- [ ] `uuencode`, `uudecode` — were removed from POSIX in 2001 but often present.

### System / environment

- [ ] `envsubst` — gettext-utils, not POSIX.
- [ ] `tempfile` — Debian-specific, deprecated.
- [ ] `mktemp` (command) — never POSIX, not even in 2024. Always needs a fallback
      or marked exception.
- [ ] `mktemp` without template — behavior varies. Always pass a template.
- [ ] `mktemp -t` — semantics differ between GNU and BSD.
- [ ] `flock` — util-linux, not POSIX. Use `mkdir` as a lock primitive.
- [ ] `timeout` — GNU/coreutils. Not POSIX. Use `alarm`-style trap with `&`.
- [ ] `yes` — common but not POSIX.
- [ ] `watch` — not POSIX.
- [ ] `script` — not POSIX.
- [ ] `pgrep`, `pkill` — not POSIX. Use `ps | grep` or process-tree walking.
- [ ] `ps` flags — very different between GNU, BSD, Solaris. POSIX `ps` supports
      `-e`, `-f`, `-l`, `-p`, `-u`, `-g`, `-t`, `-o`; avoid `aux`-style BSD syntax
      in portable code.

### Networking

- [ ] `curl` — not POSIX. Always needs a fallback or marked exception.
- [ ] `wget` — not POSIX. Always needs a fallback or marked exception.
- [ ] `nc` / `netcat` — not POSIX; multiple incompatible implementations.
- [ ] `dig`, `host`, `nslookup` — not POSIX.
- [ ] `ping` flags — differ between GNU/BSD/Solaris.

### Structured data

- [ ] `jq` — not POSIX. Needs a fallback (python3/perl) or marked exception.
- [ ] `yq` — not POSIX, and two unrelated implementations exist.
- [ ] Anything that parses JSON/YAML/TOML with shell regex — avoid.

### Date / time

- [ ] `date -d "string"` — GNU date string parsing. BSD uses `date -j -f fmt`.
- [ ] `date -r N` (epoch to date) — GNU form; BSD has different semantics.
- [ ] `date +%N` — nanoseconds, GNU only.
- [ ] `date --iso-8601` — GNU. Use `date +%Y-%m-%dT%H:%M:%S%z` explicitly.

Note: `date +%s` (epoch output) is POSIX as of 2024.

### Text encoding

- [ ] `iconv` — POSIX! This one's fine.
- [ ] `recode` — not POSIX.

---

## Locale and environment assumptions

- [ ] Assuming UTF-8 by default — set `LC_ALL=C` for byte-oriented operations
      (especially `tr`, `sort`, `awk` on binary data).
- [ ] Assuming `$HOME`, `$USER`, `$PWD` are set without fallback.
- [ ] Assuming `/tmp` is writable without `${TMPDIR:-/tmp}`.
- [ ] Assuming specific PATH order; use `command -v` to check.

---

## Patterns that look POSIX but aren't

- [ ] `[ -v VAR ]` — not POSIX. Use `[ "${VAR+x}" = x ]`.
- [ ] `echo $((x++))` — increment in arithmetic is not in POSIX (though widely
      supported). Use `x=$((x + 1))`.
- [ ] `trap '' ERR` — `ERR` is not a POSIX signal.
- [ ] `declare -A` / associative arrays — bash 4+, not POSIX.
- [ ] `readarray`, `mapfile` — bash. Use `while read` loops.
- [ ] `wait -n` — bash. Wait for any job.
- [ ] `exec -a name` — bash; rename the spawned process.
- [ ] `kill -0` works everywhere but `kill %1` (job spec) relies on job control.

---

## Constructs that ARE POSIX (common confusion)

Just to set the record straight — these are fine:

- `$(command)` — preferred over backticks.
- `$((expr))` — arithmetic expansion.
- `${var:-default}`, `${var:=default}`, `${var:+alt}`, `${var:?error}`.
- `${var#pattern}`, `${var##pattern}`, `${var%pattern}`, `${var%%pattern}`.
- `trap 'cmd' EXIT INT HUP TERM`.
- `getopts` (the builtin).
- `printf`.
- `set -e`, `set -u`, `set -x`, `set -o pipefail` (last one as of 2024).
- `read -r`.
- `while ... do ... done < file`.
- `case` / `esac`.
- `select` / `in` — surprising but specified.
- Here-documents `<<EOF` and `<<-EOF`.
- `pwd -L`, `pwd -P`.
- `test`, `[ ]`, all file-test primaries listed above.
- `find ... -print0` and `xargs -0` — as of POSIX.1-2024.
- `head -c`, `tail -c`, `sed -E`, `grep -o` — as of POSIX.1-2024.
- `date +%s` — as of POSIX.1-2024.
- `realpath`, `readlink` — basic forms, as of POSIX.1-2024.

---

## Exceptions policy

This workspace allows the following *without* marking as an exception:

- **Git** — the `git` CLI, all subcommands, assumed available.
- **Rust toolchain** — `rustup`, `cargo`, `rustc`, and anything installed via
  rustup components. `cargo xtask ...` is the preferred home for any logic
  that would otherwise require non-POSIX shell.

Any other non-POSIX tool or construct requires a **clearly documented
exception with a proper fallback**. An exception must:

1. Be gated behind feature detection (`command -v tool`, or equivalent).
2. Provide a POSIX-only fallback, or fail clearly if no fallback is
   possible.
3. Be listed in the script's header comment, naming each non-POSIX tool
   used and the conditions under which it is used.

The location, format, and review process for these exceptions are defined
elsewhere in this workspace's contribution documentation.
