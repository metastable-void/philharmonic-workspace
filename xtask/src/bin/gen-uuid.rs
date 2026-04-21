//! gen-uuid — workspace-canonical UUID generator.
//!
//! Every stable wire-format UUID in this workspace (entity `KIND`
//! constants, algorithm identifiers, key IDs, any value that once
//! committed must never change) is generated through this tool.
//! Reason: one canonical source of randomness instead of ad-hoc
//! `python3 -c "import uuid"` or `uuidgen` calls scattered across
//! sessions, so nobody accidentally commits a value they meant to
//! use once and throw away. See `docs/design/13-conventions.md
//! §KIND UUID generation` for the rule.
//!
//! Usage:
//!   cargo run -p xtask --bin gen-uuid -- --v4
//!
//! `--v4` is mandatory on purpose. Every KIND we mint in this
//! workspace is a v4 random UUID; requiring the version-flag
//! argument means a future shift to v5/v7 is an explicit CLI
//! change at call sites rather than a silent default swap.

use clap::Parser;
use uuid::Uuid;

/// Generate a UUID and print it to stdout.
#[derive(Parser)]
#[command(
    name = "gen-uuid",
    about = "Generate a UUID. Workspace convention: every stable KIND \
             UUID, algorithm identifier, or wire-format constant is \
             minted through this tool, so the source of randomness is \
             uniform across sessions and machines."
)]
struct Args {
    /// Generate a v4 (random) UUID. Required — see the module
    /// doc for why there's no default.
    #[arg(long, required = true)]
    v4: bool,
}

fn main() {
    let _args = Args::parse();
    println!("{}", Uuid::new_v4());
}
