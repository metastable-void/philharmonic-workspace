//! crates-io-versions — list the published (non-yanked) versions
//! of a crate on crates.io, one per line, in the order the
//! registry stores them (effectively oldest first).
//!
//! Usage:
//!   ./scripts/xtask.sh crates-io-versions -- <crate-name>
//!
//! Example:
//!   ./scripts/xtask.sh crates-io-versions -- mechanics-core
//!   0.1.0
//!   0.2.0
//!   0.2.1
//!   0.2.2
//!   0.3.0
//!
//! Queries the crates.io **sparse index** (index.crates.io)
//! directly, the same mechanism cargo itself uses. Each line of
//! the response is a JSON object describing one version; we
//! filter out yanked releases (they're still in the index but
//! unavailable for fresh resolution) and print the version string
//! of the rest.
//!
//! Replaces the former `scripts/crates-io-versions.sh`, which
//! depended on `jq` (non-POSIX, not shipped in stripped GNU/Linux
//! or macOS base installs) and `web-fetch.sh` (curl/wget/fetch/
//! ftp wrapper). This Rust bin has its own HTTP client (`ureq` +
//! `rustls`) and JSON parser (`serde_json`), so neither external
//! tool is on the critical path.
//!
//! Exit codes:
//!   0    crate found; versions printed (empty if all yanked).
//!   1    input error (empty crate name).
//!   2    crate not found on crates.io (HTTP 404), network
//!        error, or malformed upstream response.

use clap::Parser;
use serde::Deserialize;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "crates-io-versions",
    about = "List the published (non-yanked) versions of a crate on crates.io."
)]
struct Args {
    /// Crate name. Lowercased before lookup (crates.io crate
    /// names are case-insensitive on the index).
    crate_name: String,
}

#[derive(Deserialize)]
struct IndexEntry {
    vers: String,
    yanked: bool,
}

fn sparse_index_path(name: &str) -> String {
    // Sparse-index layout per
    // https://doc.rust-lang.org/cargo/reference/registry-index.html:
    //   length 1:  1/<crate>
    //   length 2:  2/<crate>
    //   length 3:  3/<first-char>/<crate>
    //   length 4+: <first-2-chars>/<chars-3-4>/<crate>
    //
    // Crate names on crates.io are ASCII (alphanumerics + `-_`),
    // so byte-indexing is safe here.
    match name.len() {
        1 => format!("1/{}", name),
        2 => format!("2/{}", name),
        3 => format!("3/{}/{}", &name[..1], name),
        _ => format!("{}/{}/{}", &name[..2], &name[2..4], name),
    }
}

fn main() -> ExitCode {
    let args = Args::parse();
    let name = args.crate_name.trim().to_lowercase();

    if name.is_empty() {
        eprintln!("!!! crates-io-versions: crate name cannot be empty");
        return ExitCode::from(1);
    }

    let url = format!("https://index.crates.io/{}", sparse_index_path(&name));
    let response = match ureq::get(&url).call() {
        Ok(r) => r,
        Err(ureq::Error::Status(404, _)) => {
            eprintln!(
                "!!! crates-io-versions: crate `{}` not found on crates.io (HTTP 404)",
                name
            );
            return ExitCode::from(2);
        }
        Err(ureq::Error::Status(code, resp)) => {
            eprintln!(
                "!!! crates-io-versions: HTTP {} {} for `{}`",
                code,
                resp.status_text(),
                name
            );
            return ExitCode::from(2);
        }
        Err(e) => {
            eprintln!("!!! crates-io-versions: fetch failed: {}", e);
            return ExitCode::from(2);
        }
    };
    let body = match response.into_string() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("!!! crates-io-versions: read body failed: {}", e);
            return ExitCode::from(2);
        }
    };

    for line in body.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let entry: IndexEntry = match serde_json::from_str(line) {
            Ok(e) => e,
            // Skip malformed lines rather than abort. Defensive —
            // the sparse index should be well-formed but we don't
            // want a single bad line to hide all good versions.
            Err(_) => continue,
        };
        if !entry.yanked {
            println!("{}", entry.vers);
        }
    }

    ExitCode::SUCCESS
}
