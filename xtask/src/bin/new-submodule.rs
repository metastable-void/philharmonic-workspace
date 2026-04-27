//! new-submodule — scaffold a new workspace submodule crate,
//! correctly and repeatably; or adopt an existing remote crate
//! as a submodule via `--adopt-existing`.
//!
//! ## Usage — scaffolding (default mode)
//!
//!   ./scripts/new-submodule.sh \
//!     --name <crate-name> \
//!     --description "<one-line description>" \
//!     --remote-url <git remote URL>
//!
//!   ./scripts/new-submodule.sh \
//!     --name philharmonic-connector-impl-api \
//!     --description "Trait-only API crate …" \
//!     --remote-url https://github.com/metastable-void/philharmonic-connector-impl-api.git \
//!     --before philharmonic-connector-impl-http-forward
//!
//! ## Usage — adopting an existing crate (`--adopt-existing`)
//!
//! Use when the remote already has a real `Cargo.toml` + source
//! that you want to keep — e.g. an upstream-developed helper
//! crate being pulled into the workspace as a submodule. The
//! scaffolding step is skipped so the remote's existing files
//! are not clobbered. `--description` is not required in this
//! mode (the existing `Cargo.toml`'s description stands).
//!
//!   ./scripts/new-submodule.sh \
//!     --name inline-blob \
//!     --remote-url https://github.com/metastable-void/inline-blob.git \
//!     --adopt-existing
//!
//! ## What this bin does
//!
//! 1. Preflight checks — arg validation, tool availability,
//!    workspace-root shape, no existing directory at `<name>`,
//!    no existing `.gitmodules` reference, no existing
//!    `[workspace].members` entry, and that the remote URL has
//!    at least one reachable ref (empty repo → fail fast;
//!    create the GitHub repo with an initial commit first).
//! 2. `git submodule add <remote-url> <name>` in the workspace
//!    root — clones, stages `.gitmodules` and `<name>` in the
//!    parent's index.
//! 3. Configures the submodule's local git:
//!    `core.hooksPath` = relative path to workspace `.githooks/`,
//!    `commit.gpgsign=true`, `tag.gpgsign=true`,
//!    `rebase.gpgsign=true`. Same configuration
//!    `scripts/setup.sh` applies to existing submodules.
//! 4. **Scaffolding step (default mode only).** Skipped under
//!    `--adopt-existing`. Otherwise overwrites whatever the
//!    initial README from the remote had with the workspace
//!    placeholder set:
//!      - `Cargo.toml` (workspace-standard shape,
//!        version `0.0.0`)
//!      - `src/lib.rs` (one-line placeholder)
//!      - `README.md` (workspace placeholder template)
//!      - `LICENSE-APACHE`, `LICENSE-MPL`
//!        (copied from workspace root)
//!      - `CHANGELOG.md` (Unreleased + 0.0.0 reservation)
//!      - `.gitignore` (workspace-standard ignores)
//!
//!    Under `--adopt-existing`, the bin instead requires that
//!    the cloned submodule already contains a `Cargo.toml`
//!    (any other layout fails preflight).
//! 5. Inserts the new crate into the workspace root
//!    `Cargo.toml`:
//!      - `[workspace].members` entry (before `--before
//!        <existing-member>` if given, else before the
//!        in-tree-crates comment block).
//!      - `[patch.crates-io]` entry redirecting the crate name
//!        to its local path.
//!
//!    Skip this whole step with `--skip-workspace-member` if
//!    the new submodule isn't meant to be a workspace member.
//!
//! ## What this bin intentionally does NOT do
//!
//! - **Create the GitHub repo.** The remote must already exist
//!   with at least one commit (e.g., via `gh repo create
//!   <name> --add-readme` or the GitHub web UI "Initialize
//!   with a README"). This bin asks for the URL as input.
//! - **Commit anything** — the scaffolded files stay dirty in
//!   the submodule working tree, and the parent's
//!   `.gitmodules` / `Cargo.toml` changes stay dirty in the
//!   parent's index. The caller runs
//!   `./scripts/commit-all.sh "<message>"` next, which walks
//!   the new submodule and the parent in a single pass with
//!   the workspace's signing + Audit-Info trailer discipline,
//!   and then `./scripts/push-all.sh`.
//!
//! ## Exit codes
//!
//!   0   scaffold complete; caller should run commit-all.sh +
//!       push-all.sh.
//!   1   preflight check failed (bad args, missing tool,
//!       already-existing directory or member, empty remote).
//!   2   git operation failed (submodule add, config, etc.);
//!       the parent index may hold a partial state.
//!   3   file I/O failed while scaffolding; same caveat.

use clap::Parser;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const IN_TREE_COMMENT_ANCHOR: &str = "# In-tree (non-submodule) member crates live below.";

#[derive(Parser)]
#[command(
    name = "new-submodule",
    about = "Scaffold a new workspace submodule crate, correctly."
)]
struct Args {
    /// Crate name. Validated against a shape compatible with
    /// both Cargo crate names and GitHub repo names:
    /// lowercase ASCII, digits, `-`; must start with a letter
    /// and end with a letter or digit.
    #[arg(long)]
    name: String,

    /// One-line crate description. Goes in `Cargo.toml`
    /// `description` and in the README sub-title. Keep it
    /// under 200 chars, no surrounding quotes.
    /// Required in scaffold mode; ignored under
    /// `--adopt-existing` (the existing `Cargo.toml`'s
    /// description stands).
    #[arg(long)]
    description: Option<String>,

    /// Remote URL of the (already-created) git repository,
    /// e.g. `https://github.com/metastable-void/<name>.git`.
    /// The repo must have at least one commit.
    #[arg(long)]
    remote_url: String,

    /// If given, insert the new member into root
    /// `Cargo.toml`'s `[workspace].members` immediately
    /// BEFORE this existing member. If omitted, append before
    /// the in-tree-crates comment block.
    #[arg(long)]
    before: Option<String>,

    /// Don't touch root `Cargo.toml`. Useful for experimental
    /// submodules that aren't workspace members.
    #[arg(long)]
    skip_workspace_member: bool,

    /// Adopt an existing remote crate as a submodule rather
    /// than scaffolding placeholders. Skips the file-overwrite
    /// step (`Cargo.toml`, `src/lib.rs`, `README.md`,
    /// `LICENSE-*`, `CHANGELOG.md`, `.gitignore`) so the
    /// remote's contents survive intact. The remote must
    /// already carry a `Cargo.toml` for the new submodule to
    /// be a workable workspace member.
    #[arg(long)]
    adopt_existing: bool,

    /// Print the plan and exit without executing any step.
    #[arg(long)]
    dry_run: bool,
}

fn main() -> ExitCode {
    let args = Args::parse();

    if let Err(e) = run(&args) {
        eprintln!("!!! new-submodule: {e}");
        return e.exit_code();
    }

    ExitCode::SUCCESS
}

#[derive(Debug)]
enum Error {
    Preflight(String),
    Git(String),
    Io(String),
}

impl Error {
    fn exit_code(&self) -> ExitCode {
        match self {
            Self::Preflight(_) => ExitCode::from(1),
            Self::Git(_) => ExitCode::from(2),
            Self::Io(_) => ExitCode::from(3),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Preflight(m) | Self::Git(m) | Self::Io(m) => f.write_str(m),
        }
    }
}

fn run(args: &Args) -> Result<(), Error> {
    // 1. Preflight.
    validate_name(&args.name)?;
    if !args.adopt_existing && args.description.is_none() {
        return Err(Error::Preflight(
            "--description is required in scaffold mode; pass it or use --adopt-existing"
                .to_owned(),
        ));
    }
    let workspace_root = find_workspace_root()?;
    let submodule_dir = workspace_root.join(&args.name);
    let cargo_toml_path = workspace_root.join("Cargo.toml");
    let gitmodules_path = workspace_root.join(".gitmodules");

    require_tool("git")?;
    require_workspace_root(&workspace_root)?;
    require_absent_dir(&submodule_dir)?;
    require_absent_gitmodules_entry(&gitmodules_path, &args.name)?;
    if !args.skip_workspace_member {
        require_absent_workspace_member(&cargo_toml_path, &args.name)?;
        if let Some(before) = &args.before {
            require_existing_workspace_member(&cargo_toml_path, before)?;
        }
    }
    require_reachable_remote(&args.remote_url)?;

    // 2. Print plan.
    print_plan(args, &workspace_root);
    if args.dry_run {
        eprintln!("=== dry-run: no changes made");
        return Ok(());
    }

    // 3. git submodule add <url> <name>
    run_git(
        &workspace_root,
        &["submodule", "add", &args.remote_url, &args.name],
    )?;

    // 4. Configure submodule's git.
    configure_submodule_git(&workspace_root, &submodule_dir)?;

    // 5. Scaffold files (skipped when adopting).
    if args.adopt_existing {
        require_existing_cargo_toml(&submodule_dir)?;
    } else {
        let description = args
            .description
            .as_deref()
            .expect("description presence checked above");
        scaffold_files(&workspace_root, &submodule_dir, &args.name, description)?;
    }

    // 6. Insert into root Cargo.toml unless skipped.
    if !args.skip_workspace_member {
        insert_workspace_member(&cargo_toml_path, &args.name, args.before.as_deref())?;
        insert_patch_entry(&cargo_toml_path, &args.name)?;
    }

    // 7. Next-step hints.
    println!();
    if args.adopt_existing {
        println!("=== adoption complete");
    } else {
        println!("=== scaffold complete");
    }
    println!();
    println!("Next steps (nothing has been committed or pushed yet):");
    println!();
    let commit_blurb = if args.adopt_existing {
        format!("add {} submodule — adopted existing crate", args.name)
    } else {
        format!("add {} submodule — placeholder scaffolding", args.name)
    };
    println!("  ./scripts/commit-all.sh {commit_blurb:?}");
    println!("  ./scripts/push-all.sh");
    println!();
    println!("commit-all.sh will walk the new submodule and the parent in one");
    println!("pass; push-all.sh pushes both (submodule first, then the parent");
    println!("pointer).");
    Ok(())
}

/// In `--adopt-existing` mode we trust the remote's contents,
/// but the absolute minimum for a workspace member is that the
/// cloned submodule has a `Cargo.toml` at its root. Anything
/// else (multi-crate workspace inside the submodule, missing
/// manifest, etc.) is out of scope here — surface it so the
/// caller can decide.
fn require_existing_cargo_toml(submodule_dir: &Path) -> Result<(), Error> {
    let cargo_toml = submodule_dir.join("Cargo.toml");
    if !cargo_toml.exists() {
        return Err(Error::Preflight(format!(
            "--adopt-existing: cloned submodule {submodule_dir:?} has no Cargo.toml at its root; \
             cannot register as a workspace member"
        )));
    }
    Ok(())
}

// ----- preflight helpers -----

/// Validates the crate name:
///   - ASCII lowercase letters + digits + `-`.
///   - First char is a letter.
///   - Last char is a letter or digit.
///   - No double hyphens (matches crates.io + GitHub repo-name norms).
fn validate_name(name: &str) -> Result<(), Error> {
    if name.is_empty() {
        return Err(Error::Preflight("crate name is empty".to_owned()));
    }
    let bytes = name.as_bytes();
    let is_alpha = |b: u8| b.is_ascii_lowercase();
    let is_alnum = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit();
    if !is_alpha(bytes[0]) {
        return Err(Error::Preflight(format!(
            "crate name must start with a lowercase letter: {name:?}"
        )));
    }
    if !is_alnum(*bytes.last().expect("checked non-empty above")) {
        return Err(Error::Preflight(format!(
            "crate name must end with a letter or digit: {name:?}"
        )));
    }
    let mut prev_hyphen = false;
    for &b in bytes {
        if b == b'-' {
            if prev_hyphen {
                return Err(Error::Preflight(format!(
                    "crate name contains `--`: {name:?}"
                )));
            }
            prev_hyphen = true;
            continue;
        }
        if !is_alnum(b) {
            return Err(Error::Preflight(format!(
                "crate name contains disallowed character {:?}: {name:?}",
                b as char
            )));
        }
        prev_hyphen = false;
    }
    Ok(())
}

fn find_workspace_root() -> Result<PathBuf, Error> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| Error::Preflight(format!("could not invoke git: {e}")))?;
    if !output.status.success() {
        return Err(Error::Preflight(
            "`git rev-parse --show-toplevel` failed; run from inside the workspace".to_owned(),
        ));
    }
    let root = String::from_utf8(output.stdout)
        .map_err(|e| Error::Preflight(format!("git output not UTF-8: {e}")))?
        .trim()
        .to_owned();
    Ok(PathBuf::from(root))
}

fn require_tool(name: &str) -> Result<(), Error> {
    let status = Command::new(name)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    match status {
        Ok(s) if s.success() => Ok(()),
        _ => Err(Error::Preflight(format!(
            "required tool not on PATH: {name}"
        ))),
    }
}

fn require_workspace_root(root: &Path) -> Result<(), Error> {
    let marker = root.join("scripts").join("test-scripts.sh");
    if !marker.exists() {
        return Err(Error::Preflight(format!(
            "expected {marker:?} to exist — not a philharmonic workspace root"
        )));
    }
    Ok(())
}

fn require_absent_dir(dir: &Path) -> Result<(), Error> {
    if dir.exists() {
        return Err(Error::Preflight(format!(
            "{dir:?} already exists — refusing to overwrite"
        )));
    }
    Ok(())
}

fn require_absent_gitmodules_entry(gitmodules: &Path, name: &str) -> Result<(), Error> {
    if !gitmodules.exists() {
        return Ok(());
    }
    let body = std::fs::read_to_string(gitmodules)
        .map_err(|e| Error::Preflight(format!("reading {gitmodules:?}: {e}")))?;
    let needle = format!("[submodule \"{name}\"]");
    if body.contains(&needle) {
        return Err(Error::Preflight(format!(
            "{gitmodules:?} already has an entry for {name:?}"
        )));
    }
    Ok(())
}

fn require_absent_workspace_member(cargo_toml: &Path, name: &str) -> Result<(), Error> {
    let body = std::fs::read_to_string(cargo_toml)
        .map_err(|e| Error::Preflight(format!("reading {cargo_toml:?}: {e}")))?;
    let needle = format!("\"{name}\"");
    if body.contains(&needle) {
        return Err(Error::Preflight(format!(
            "root Cargo.toml already mentions {name:?}"
        )));
    }
    Ok(())
}

fn require_existing_workspace_member(cargo_toml: &Path, name: &str) -> Result<(), Error> {
    let body = std::fs::read_to_string(cargo_toml)
        .map_err(|e| Error::Preflight(format!("reading {cargo_toml:?}: {e}")))?;
    let needle = format!("\"{name}\"");
    if !body.contains(&needle) {
        return Err(Error::Preflight(format!(
            "--before member not found in root Cargo.toml: {name:?}"
        )));
    }
    Ok(())
}

/// Checks that the remote is reachable and has at least one
/// reference. `git ls-remote <url>` emits one line per ref;
/// an empty repo produces empty output.
fn require_reachable_remote(url: &str) -> Result<(), Error> {
    let output = Command::new("git")
        .args(["ls-remote", url, "HEAD"])
        .output()
        .map_err(|e| Error::Preflight(format!("invoking `git ls-remote`: {e}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Preflight(format!(
            "remote {url:?} not reachable: {}",
            stderr.trim()
        )));
    }
    if output.stdout.is_empty() {
        return Err(Error::Preflight(format!(
            "remote {url:?} has no commits yet — create it with at least one \
             initial commit (e.g. `gh repo create <name> --add-readme`)"
        )));
    }
    Ok(())
}

// ----- plan output -----

fn print_plan(args: &Args, workspace_root: &Path) {
    eprintln!("=== new-submodule plan");
    eprintln!("  workspace root:   {}", workspace_root.display());
    eprintln!("  crate name:       {}", args.name);
    eprintln!(
        "  mode:             {}",
        if args.adopt_existing {
            "adopt-existing (skip scaffolding; require remote Cargo.toml)"
        } else {
            "scaffold (overwrite remote files with placeholders)"
        }
    );
    if args.adopt_existing {
        eprintln!("  description:      (ignored in adopt mode)");
    } else {
        eprintln!(
            "  description:      {}",
            args.description.as_deref().unwrap_or("(missing)")
        );
    }
    eprintln!("  remote URL:       {}", args.remote_url);
    eprintln!("  path under root:  {}/", args.name);
    if args.skip_workspace_member {
        eprintln!("  Cargo.toml edit:  skipped (--skip-workspace-member)");
    } else {
        match &args.before {
            Some(b) => eprintln!("  Cargo.toml edit:  insert before {b:?} in [workspace].members"),
            None => eprintln!(
                "  Cargo.toml edit:  append before in-tree comment in [workspace].members"
            ),
        }
    }
    eprintln!("  dry run:          {}", args.dry_run);
    eprintln!();
}

// ----- git operations -----

fn run_git(cwd: &Path, argv: &[&str]) -> Result<(), Error> {
    let status = Command::new("git")
        .current_dir(cwd)
        .args(argv)
        .status()
        .map_err(|e| Error::Git(format!("spawning `git {argv:?}`: {e}")))?;
    if !status.success() {
        return Err(Error::Git(format!("`git {argv:?}` exited with {status}")));
    }
    Ok(())
}

fn configure_submodule_git(workspace_root: &Path, submodule_dir: &Path) -> Result<(), Error> {
    // Relative path from the submodule to the workspace's
    // `.githooks` — same shape `scripts/setup.sh` applies
    // through `lib/relpath.sh`.
    let hooks_path = relative_path(submodule_dir, &workspace_root.join(".githooks"))?;
    for (key, value) in [
        ("core.hooksPath", hooks_path.as_str()),
        ("commit.gpgsign", "true"),
        ("tag.gpgsign", "true"),
        ("rebase.gpgsign", "true"),
    ] {
        run_git(submodule_dir, &["config", "--local", key, value])?;
    }
    Ok(())
}

/// Computes a POSIX-style relative path `from` → `to`. Both
/// must be absolute. Mirrors what `scripts/lib/relpath.sh`
/// does in pure shell.
fn relative_path(from: &Path, to: &Path) -> Result<String, Error> {
    let from = from
        .canonicalize()
        .map_err(|e| Error::Io(format!("canonicalize {from:?}: {e}")))?;
    let to = to
        .canonicalize()
        .map_err(|e| Error::Io(format!("canonicalize {to:?}: {e}")))?;

    let mut from_iter = from.components();
    let mut to_iter = to.components();
    loop {
        match (from_iter.clone().next(), to_iter.clone().next()) {
            (Some(a), Some(b)) if a == b => {
                from_iter.next();
                to_iter.next();
            }
            _ => break,
        }
    }
    let up = from_iter.count();
    let rest: PathBuf = to_iter.collect();
    let mut out = String::new();
    for _ in 0..up {
        out.push_str("../");
    }
    out.push_str(&rest.to_string_lossy());
    Ok(out)
}

// ----- scaffolding -----

fn scaffold_files(
    workspace_root: &Path,
    submodule_dir: &Path,
    name: &str,
    description: &str,
) -> Result<(), Error> {
    // Cargo.toml
    let cargo_toml = render_cargo_toml(name, description);
    write(&submodule_dir.join("Cargo.toml"), &cargo_toml)?;

    // src/lib.rs
    let src_dir = submodule_dir.join("src");
    std::fs::create_dir_all(&src_dir).map_err(|e| Error::Io(format!("mkdir {src_dir:?}: {e}")))?;
    write(
        &src_dir.join("lib.rs"),
        &format!("// {name}: placeholder\n"),
    )?;

    // README.md (overwrite whatever the remote's initial commit had)
    let readme = render_readme(name);
    write(&submodule_dir.join("README.md"), &readme)?;

    // LICENSE-APACHE, LICENSE-MPL (copy from workspace root)
    for license in ["LICENSE-APACHE", "LICENSE-MPL"] {
        let src = workspace_root.join(license);
        let dst = submodule_dir.join(license);
        std::fs::copy(&src, &dst)
            .map_err(|e| Error::Io(format!("copy {src:?} -> {dst:?}: {e}")))?;
    }

    // CHANGELOG.md
    write(&submodule_dir.join("CHANGELOG.md"), CHANGELOG_TEMPLATE)?;

    // .gitignore
    write(&submodule_dir.join(".gitignore"), GITIGNORE_TEMPLATE)?;

    Ok(())
}

fn write(path: &Path, body: &str) -> Result<(), Error> {
    std::fs::write(path, body).map_err(|e| Error::Io(format!("writing {path:?}: {e}")))
}

fn render_cargo_toml(name: &str, description: &str) -> String {
    format!(
        "[package]\n\
         name = \"{name}\"\n\
         version = \"0.0.0\"\n\
         edition = \"2024\"\n\
         rust-version = \"1.88\"\n\
         license = \"Apache-2.0 OR MPL-2.0\"\n\
         readme = \"README.md\"\n\
         repository = \"https://github.com/metastable-void/{name}\"\n\
         description = \"{description}\"\n\
         \n\
         [dependencies]\n\
         \n\
         [profile.release]\n\
         opt-level = 3\n\
         lto = true\n\
         strip = true\n\
         codegen-units = 1\n\
         panic = \"abort\"\n\
         overflow-checks = true\n"
    )
}

fn render_readme(name: &str) -> String {
    format!(
        "# {name}\n\
         \n\
         Part of the Philharmonic workspace: https://github.com/metastable-void/philharmonic-workspace\n\
         \n\
         ## Contributing\n\
         \n\
         This crate is developed as a submodule of the Philharmonic\n\
         workspace. Workspace-wide development conventions — git workflow,\n\
         script wrappers, Rust code rules, versioning, terminology — live\n\
         in the workspace meta-repo at\n\
         [metastable-void/philharmonic-workspace](https://github.com/metastable-void/philharmonic-workspace),\n\
         authoritatively in its\n\
         [`CONTRIBUTING.md`](https://github.com/metastable-void/philharmonic-workspace/blob/main/CONTRIBUTING.md).\n\
         \n\
         SPDX-License-Identifier: Apache-2.0 OR MPL-2.0\n"
    )
}

const CHANGELOG_TEMPLATE: &str = "\
# Changelog

All notable changes to this crate are documented in this file.

The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this crate adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Implementation pending. See the
[Philharmonic workspace ROADMAP](https://github.com/metastable-void/philharmonic-workspace/blob/main/ROADMAP.md)
for the phase that populates this crate.

## [0.0.0]

Name reservation on crates.io. No functional content yet.
";

const GITIGNORE_TEMPLATE: &str = "\
target/
**/*.rs.bk
Cargo.lock
.DS_Store
._*
.codex
.vscode/settings.json
";

// ----- root Cargo.toml edits -----

fn insert_workspace_member(
    cargo_toml: &Path,
    name: &str,
    before: Option<&str>,
) -> Result<(), Error> {
    let body = std::fs::read_to_string(cargo_toml)
        .map_err(|e| Error::Io(format!("reading {cargo_toml:?}: {e}")))?;
    let new_body = insert_member_into_toml(&body, name, before)?;
    std::fs::write(cargo_toml, new_body)
        .map_err(|e| Error::Io(format!("writing {cargo_toml:?}: {e}")))
}

/// Line-based insertion into `[workspace].members`:
///   - If `before` is `Some("<member>")`: insert a new
///     `    "<name>",` line immediately before the first line
///     whose trimmed form is `"<member>",`.
///   - Else: insert before the first line whose trimmed form
///     starts with the in-tree-crates comment anchor.
///
/// Returns the new TOML body. Preserves all other content
/// verbatim.
fn insert_member_into_toml(body: &str, name: &str, before: Option<&str>) -> Result<String, Error> {
    let target_needle = before.map(|b| format!("\"{b}\","));
    let new_line = format!("    \"{name}\",\n");
    let mut out = String::with_capacity(body.len() + new_line.len());
    let mut inserted = false;

    for line in body.lines() {
        let trimmed = line.trim();
        let hit = match &target_needle {
            Some(n) => trimmed == n.as_str(),
            None => trimmed.starts_with(IN_TREE_COMMENT_ANCHOR),
        };
        if hit && !inserted {
            out.push_str(&new_line);
            inserted = true;
        }
        out.push_str(line);
        out.push('\n');
    }
    // Preserve trailing-newline behaviour (body.lines() drops the final newline
    // if present; we re-add one per line so the result ends with exactly one).
    if !body.ends_with('\n') {
        out.pop();
    }
    if !inserted {
        return Err(Error::Io(format!(
            "couldn't find insertion point in root Cargo.toml \
             (expected {:?})",
            target_needle.as_deref().unwrap_or(IN_TREE_COMMENT_ANCHOR)
        )));
    }
    Ok(out)
}

fn insert_patch_entry(cargo_toml: &Path, name: &str) -> Result<(), Error> {
    let body = std::fs::read_to_string(cargo_toml)
        .map_err(|e| Error::Io(format!("reading {cargo_toml:?}: {e}")))?;
    let new_body = insert_patch_into_toml(&body, name)?;
    std::fs::write(cargo_toml, new_body)
        .map_err(|e| Error::Io(format!("writing {cargo_toml:?}: {e}")))
}

/// Append a `<name> = { path = "<name>" }` line at the end of
/// the `[patch.crates-io]` block. The block is delimited by
/// the `[patch.crates-io]` header on one side and the next
/// `[...]` header (or end of file) on the other.
fn insert_patch_into_toml(body: &str, name: &str) -> Result<String, Error> {
    let new_line = format!("{name} = {{ path = \"{name}\" }}\n");
    let header = "[patch.crates-io]";
    let lines: Vec<&str> = body.lines().collect();
    let start = match lines.iter().position(|l| l.trim() == header) {
        Some(i) => i,
        None => {
            return Err(Error::Io(format!(
                "no `{header}` section found in root Cargo.toml"
            )));
        }
    };

    // Find the last non-empty line of the block (i.e. the last
    // `crate = { path = ... }` entry) or the next section
    // header, whichever comes first.
    let mut insert_at = start + 1;
    for (i, line) in lines.iter().enumerate().skip(start + 1) {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            break; // hit the next section
        }
        if !t.is_empty() {
            insert_at = i + 1;
        }
    }

    let mut out = String::with_capacity(body.len() + new_line.len());
    for (i, line) in lines.iter().enumerate() {
        if i == insert_at {
            out.push_str(&new_line);
        }
        out.push_str(line);
        out.push('\n');
    }
    if insert_at == lines.len() {
        out.push_str(&new_line);
    }
    if !body.ends_with('\n') {
        out.pop();
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_name_accepts_canonical_shapes() {
        validate_name("serde").unwrap();
        validate_name("philharmonic-connector-impl-api").unwrap();
        validate_name("tokio-util").unwrap();
        validate_name("a1").unwrap();
    }

    #[test]
    fn validate_name_rejects_bad_shapes() {
        assert!(validate_name("").is_err());
        assert!(validate_name("1crate").is_err());
        assert!(validate_name("-leading-hyphen").is_err());
        assert!(validate_name("trailing-").is_err());
        assert!(validate_name("double--hyphen").is_err());
        assert!(validate_name("UPPER").is_err());
        assert!(validate_name("under_score").is_err());
        assert!(validate_name("ascii.dot").is_err());
    }

    #[test]
    fn insert_member_default_lands_before_in_tree_comment() {
        let body = "\
[workspace]
members = [
    \"crate-a\",
    \"crate-b\",
    # In-tree (non-submodule) member crates live below. They're
    \"xtask\",
]
";
        let out = insert_member_into_toml(body, "new-crate", None).unwrap();
        assert!(out.contains("    \"crate-b\",\n    \"new-crate\",\n    # In-tree"));
    }

    #[test]
    fn insert_member_before_specific_entry() {
        let body = "\
[workspace]
members = [
    \"crate-a\",
    \"crate-b\",
    # In-tree
    \"xtask\",
]
";
        let out = insert_member_into_toml(body, "new-crate", Some("crate-b")).unwrap();
        assert!(out.contains("    \"crate-a\",\n    \"new-crate\",\n    \"crate-b\","));
    }

    #[test]
    fn insert_member_fails_when_anchor_missing() {
        let body = "[workspace]\nmembers = []\n";
        assert!(insert_member_into_toml(body, "nope", None).is_err());
        assert!(insert_member_into_toml(body, "nope", Some("missing")).is_err());
    }

    #[test]
    fn insert_patch_appends_at_end_of_section() {
        let body = "\
[workspace]
members = []

[patch.crates-io]
crate-a = { path = \"crate-a\" }
crate-b = { path = \"crate-b\" }

[profile.release]
opt-level = 3
";
        let out = insert_patch_into_toml(body, "new-crate").unwrap();
        assert!(out.contains(
            "crate-b = { path = \"crate-b\" }\nnew-crate = { path = \"new-crate\" }\n\n[profile.release]"
        ));
    }

    #[test]
    fn insert_patch_fails_when_section_missing() {
        let body = "[workspace]\nmembers = []\n";
        assert!(insert_patch_into_toml(body, "nope").is_err());
    }
}
