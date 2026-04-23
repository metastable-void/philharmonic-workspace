//! crates-io-versions — list the published (non-yanked) versions
//! of a crate on crates.io, one per line, in the order the
//! registry stores them (effectively oldest first).
//!
//! Usage:
//!   ./scripts/xtask.sh crates-io-versions -- <crate-name>
//!   ./scripts/xtask.sh crates-io-versions -- <crate> --include-prereleases
//!   ./scripts/xtask.sh crates-io-versions -- <crate> --min-age-days 7
//!   ./scripts/xtask.sh crates-io-versions -- <crate> --min-age-days 0  # disable
//!
//! Example:
//!   ./scripts/xtask.sh crates-io-versions -- mechanics-core
//!   0.1.0
//!   0.2.0
//!   0.2.1
//!   0.2.2
//!   0.3.0
//!
//! ## Behavior
//!
//! 1. Fetches the crates.io **sparse index** (index.crates.io)
//!    — the same mechanism cargo itself uses. Each line of the
//!    response is a JSON object describing one version; yanked
//!    releases are filtered out.
//! 2. By default, semver prerelease versions (`-rc.1`, `-beta`,
//!    `-alpha.2`, etc.) are **also filtered out**. Pass
//!    `--include-prereleases` to keep them.
//! 3. The remaining versions are printed one per line, in the
//!    index's insertion order (effectively oldest first).
//! 4. For each version that is **younger than** the age
//!    threshold (default 3 days), a warning is emitted on
//!    **stderr** that identifies the version and its age.
//!    The walk starts from the newest kept version and moves
//!    backward, stopping as soon as it reaches a version older
//!    than the threshold. So the tool answers: *which of the
//!    recently-published versions are "too fresh to depend on
//!    yet"*. Set `--min-age-days 0` to disable the age check.
//!
//! ## Flags
//!
//!   --include-prereleases   keep `-rc / -beta / -alpha / -pre / -dev`
//!                           versions in the output (off by default).
//!   --min-age-days <N>      warn if the newest kept version(s) were
//!                           published less than `N` days ago.
//!                           Default: 3. `0` disables the age check
//!                           entirely.
//!
//! ## HTTP path
//!
//! All crates.io traffic goes through `xtask::http::fetch_text`
//! (ureq 3 + rustls + workspace `User-Agent`), the same path the
//! `web-fetch` xtask bin uses. See `xtask/src/http.rs`.
//!
//! ## Stream separation
//!
//! - stdout: one version per line, nothing else. Machine-
//!   readable; `| tail -1` still gives the newest version.
//! - stderr: `!! crates-io-versions: <version> is <N>d old
//!   (published <timestamp>, < <threshold>-day threshold)` for
//!   each warned version; plus the existing `!!! ...` error
//!   prefix for fatal conditions.
//!
//! ## Exit codes
//!
//!   0    crate found; versions printed (empty if all filtered
//!        out). Age warnings do NOT affect the exit code.
//!   1    input error (empty crate name, bad `--min-age-days`).
//!   2    crate not found on crates.io (HTTP 404), network
//!        error, or malformed upstream response on the index
//!        fetch. Age-check fetch failures are non-fatal: a
//!        `!! ... could not verify age` note goes to stderr and
//!        the walk stops, but the exit code stays 0.
//!
//! ## Historical
//!
//! Replaces the former `scripts/crates-io-versions.sh`, which
//! depended on `jq` (non-POSIX) and `web-fetch.sh` (curl/wget/
//! fetch/ftp wrapper). This Rust bin has no external-tool
//! dependency beyond the workspace Rust toolchain.

use chrono::{DateTime, Utc};
use clap::Parser;
use serde::Deserialize;
use std::process::ExitCode;
use xtask::http::{self, HttpError};

#[derive(Parser)]
#[command(
    name = "crates-io-versions",
    about = "List the published (non-yanked) versions of a crate on crates.io."
)]
struct Args {
    /// Crate name. Lowercased before lookup (crates.io crate
    /// names are case-insensitive on the index).
    crate_name: String,

    /// Include semver prerelease versions (anything with a `-`
    /// suffix before any `+build` metadata — `-rc.1`, `-beta`,
    /// `-alpha.2`, etc.). Default: prereleases are filtered out.
    #[arg(long)]
    include_prereleases: bool,

    /// Warn if the newest kept version(s) were published less
    /// than this many days ago. Walks oldest from newest,
    /// stopping at the first version at or above the threshold.
    /// `0` disables the age check entirely. Default: 3.
    #[arg(long, default_value_t = 3)]
    min_age_days: u32,
}

#[derive(Deserialize)]
struct IndexEntry {
    vers: String,
    yanked: bool,
}

#[derive(Deserialize)]
struct VersionDetailEnvelope {
    version: VersionDetail,
}

#[derive(Deserialize)]
struct VersionDetail {
    created_at: String,
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

/// Semver prerelease detection: a version is a prerelease if its
/// core (the part before any `+build` metadata) contains a `-`.
/// Examples: `1.0.0-rc.1`, `2.0.0-beta`, `0.1.0-alpha.2`,
/// `1.0.0-dev+build.5` → prerelease. `1.0.0+deprecated`,
/// `1.0.0` → not.
fn is_prerelease(vers: &str) -> bool {
    let core = vers.split('+').next().unwrap_or(vers);
    core.contains('-')
}

fn main() -> ExitCode {
    let args = Args::parse();
    let name = args.crate_name.trim().to_lowercase();

    if name.is_empty() {
        eprintln!("!!! crates-io-versions: crate name cannot be empty");
        return ExitCode::from(1);
    }

    // 1. Fetch sparse index.
    let url = format!("https://index.crates.io/{}", sparse_index_path(&name));
    let body = match http::fetch_text(&url) {
        Ok(b) => b,
        Err(HttpError::StatusCode { code: 404 }) => {
            eprintln!(
                "!!! crates-io-versions: crate `{}` not found on crates.io (HTTP 404)",
                name
            );
            return ExitCode::from(2);
        }
        Err(HttpError::StatusCode { code }) => {
            eprintln!("!!! crates-io-versions: HTTP {} for `{}`", code, name);
            return ExitCode::from(2);
        }
        Err(e) => {
            eprintln!("!!! crates-io-versions: fetch failed: {}", e);
            return ExitCode::from(2);
        }
    };

    // 2. Parse + filter (yanked, and optionally prereleases).
    let mut versions: Vec<String> = Vec::new();
    for line in body.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let entry: IndexEntry = match serde_json::from_str(line) {
            Ok(e) => e,
            // Defensive: skip malformed index lines so one bad
            // entry doesn't hide all good versions.
            Err(_) => continue,
        };
        if entry.yanked {
            continue;
        }
        if !args.include_prereleases && is_prerelease(&entry.vers) {
            continue;
        }
        versions.push(entry.vers);
    }

    // 3. Print versions (machine-readable, one per line, stdout).
    for v in &versions {
        println!("{}", v);
    }

    // 4. Age check (stderr warnings, oldest-walk from newest).
    if args.min_age_days > 0 && !versions.is_empty() {
        let threshold_seconds: i64 = i64::from(args.min_age_days).saturating_mul(86_400);
        let now: DateTime<Utc> = Utc::now();

        // Walk newest → oldest. Stop at the first version that
        // passes the threshold (or on a fetch failure / parse
        // failure — those are treated as "cannot verify, stop").
        for version in versions.iter().rev() {
            match age_seconds(&name, version, now) {
                Ok(age) if age < threshold_seconds => {
                    let days = age / 86_400;
                    let hours = (age % 86_400) / 3_600;
                    eprintln!(
                        "!! crates-io-versions: {version} is {days}d{hours}h old \
                         (< {}d threshold)",
                        args.min_age_days
                    );
                    // recurse to the next-older version
                }
                Ok(_) => break,
                Err(msg) => {
                    eprintln!(
                        "!! crates-io-versions: could not verify age of {version}: {msg} \
                         (stopping age walk)"
                    );
                    break;
                }
            }
        }
    }

    ExitCode::SUCCESS
}

/// Fetches the version-detail JSON from crates.io API and
/// returns the age of the version in whole seconds relative to
/// `now`. Returns `Err(String)` with a short human-readable
/// reason on any failure (HTTP, JSON, timestamp parse).
fn age_seconds(name: &str, version: &str, now: DateTime<Utc>) -> Result<i64, String> {
    let url = format!("https://crates.io/api/v1/crates/{name}/{version}");
    let body = http::fetch_text(&url).map_err(|e| e.to_string())?;
    let envelope: VersionDetailEnvelope =
        serde_json::from_str(&body).map_err(|e| format!("bad JSON: {e}"))?;
    let published = DateTime::parse_from_rfc3339(&envelope.version.created_at)
        .map_err(|e| format!("bad created_at: {e}"))?
        .with_timezone(&Utc);
    Ok(now.signed_duration_since(published).num_seconds())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prerelease_detects_rc_and_beta_and_alpha() {
        assert!(is_prerelease("1.0.0-rc.1"));
        assert!(is_prerelease("2.0.0-beta"));
        assert!(is_prerelease("0.1.0-alpha.2"));
        assert!(is_prerelease("1.0.0-pre"));
        assert!(is_prerelease("0.0.1-dev"));
    }

    #[test]
    fn prerelease_accepts_build_metadata_without_hyphen_in_core() {
        // "+deprecated" is build metadata, not a prerelease.
        assert!(!is_prerelease("0.9.34+deprecated"));
        assert!(!is_prerelease("1.0.0"));
        assert!(!is_prerelease("1.2.3+build.5"));
    }

    #[test]
    fn prerelease_detects_hyphen_in_core_even_with_build() {
        // The core is "1.0.0-dev", which has a hyphen → prerelease.
        assert!(is_prerelease("1.0.0-dev+build.5"));
    }

    #[test]
    fn sparse_index_path_handles_length_edges() {
        assert_eq!(sparse_index_path("a"), "1/a");
        assert_eq!(sparse_index_path("ab"), "2/ab");
        assert_eq!(sparse_index_path("abc"), "3/a/abc");
        assert_eq!(sparse_index_path("serde"), "se/rd/serde");
        assert_eq!(sparse_index_path("mechanics-core"), "me/ch/mechanics-core");
    }
}
