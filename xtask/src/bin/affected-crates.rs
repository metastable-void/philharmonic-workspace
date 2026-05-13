//! Compute the set of workspace member crates whose tests should run
//! given a set of "dirty" (recently-modified) crates: the dirty
//! crates themselves, plus every workspace member that transitively
//! reverse-depends on any dirty crate. Manifest-level edges only
//! (normal + dev + build dependencies all count) — no
//! crates.io-resolved walking.
//!
//! Reads dirty crate names from CLI positional args; if none given,
//! reads from stdin (one per line). Emits the affected-crate set to
//! stdout, one name per line, sorted.
//!
//! Used by `scripts/pre-landing.sh` to narrow `cargo test` from a
//! full `--workspace` pass to the smaller affected set when the
//! dirty crates are known. The shell-side fallback to workspace-wide
//! testing (for `scripts/`-, root-`Cargo.toml`-, or `Cargo.lock`-
//! dirty runs, or under `--full`) is implemented in pre-landing.sh,
//! not here — this bin assumes the caller has already decided the
//! narrowed path is appropriate.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io::{BufRead, IsTerminal};
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
}

#[derive(Deserialize)]
struct Package {
    name: String,
    id: String,
    dependencies: Vec<Dependency>,
}

#[derive(Deserialize)]
struct Dependency {
    name: String,
}

fn main() -> Result<()> {
    let dirty = read_dirty_names()?;

    let metadata = run_cargo_metadata()?;

    // Workspace members keyed by name.
    let members: BTreeSet<String> = metadata
        .packages
        .iter()
        .filter(|pkg| metadata.workspace_members.contains(&pkg.id))
        .map(|pkg| pkg.name.clone())
        .collect();

    // Reverse-dep graph: dep_name -> set of member names that depend on it.
    // Edges only when both endpoints are workspace members; dev / build /
    // normal dep kinds all count equivalently (a dirty build-dep can
    // change compile output; a dirty dev-dep can change test output).
    let mut rev_deps: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for pkg in &metadata.packages {
        if !members.contains(&pkg.name) {
            continue;
        }
        for dep in &pkg.dependencies {
            if members.contains(&dep.name) {
                rev_deps
                    .entry(dep.name.clone())
                    .or_default()
                    .insert(pkg.name.clone());
            }
        }
    }

    // BFS from dirty members over reverse-dep edges.
    let mut visited: BTreeSet<String> = BTreeSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    for name in dirty {
        if members.contains(&name) {
            queue.push_back(name);
        }
    }
    while let Some(name) = queue.pop_front() {
        if !visited.insert(name.clone()) {
            continue;
        }
        if let Some(parents) = rev_deps.get(&name) {
            for parent in parents {
                if !visited.contains(parent) {
                    queue.push_back(parent.clone());
                }
            }
        }
    }

    for name in &visited {
        println!("{}", name);
    }

    Ok(())
}

fn read_dirty_names() -> Result<Vec<String>> {
    // CLI positional args take precedence over stdin to keep callers
    // explicit when convenient. Stdin path lets pre-landing.sh pipe
    // `show-dirty.sh` output directly.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        return Ok(args);
    }
    let stdin = std::io::stdin();
    if stdin.is_terminal() {
        // No args and stdin attached to a TTY → empty dirty set is
        // valid (e.g. clean checkout). Don't block on user input.
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for line in stdin.lock().lines() {
        let line = line.context("read stdin line")?;
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            out.push(trimmed.to_owned());
        }
    }
    Ok(out)
}

fn run_cargo_metadata() -> Result<Metadata> {
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .output()
        .context("failed to spawn `cargo metadata`")?;
    if !output.status.success() {
        bail!(
            "cargo metadata failed (exit {}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr),
        );
    }
    let metadata: Metadata =
        serde_json::from_slice(&output.stdout).context("parse cargo metadata JSON")?;
    Ok(metadata)
}
