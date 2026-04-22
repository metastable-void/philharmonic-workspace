//! web-fetch — HTTP(S) GET a URL; write the body to stdout or
//! to the given output file.
//!
//! Usage:
//!   ./scripts/xtask.sh web-fetch -- <URL> [<outfile>]
//!   ./scripts/web-fetch.sh <URL> [<outfile>]    # shim wrapper
//!
//! Without `<outfile>`, the body is written to stdout. With
//! `<outfile>`, the body is written to that path.
//!
//! User-Agent: overridable via the `WEB_FETCH_UA` environment
//! variable. Default is `philharmonic-dev-agent/1.0`.
//!
//! HTTP 4xx/5xx responses cause a non-zero exit (the Rust port
//! is always fail-on-error; the shell version needed an explicit
//! `-f` flag for curl to behave this way). Callers that want to
//! tolerate HTTP errors use `... || :` at the call site —
//! `print-audit-info.sh` uses that idiom for best-effort IP
//! geolocation lookups.
//!
//! Replaces the former `scripts/web-fetch.sh`, which tried
//! `curl` → `wget` → `fetch` (FreeBSD) → `ftp` (OpenBSD HTTP
//! mode) in order. `curl` and `wget` aren't part of every
//! stripped GNU/Linux or macOS baseline, so this Rust bin has
//! its own HTTP client (`ureq` + `rustls`) and works wherever
//! the workspace's Rust toolchain is installed. `scripts/web-
//! fetch.sh` remains on disk as a thin shim that execs into
//! this bin, so shell callers keep working unchanged.
//!
//! Exit codes:
//!   0    body fetched and written.
//!   1    input error (missing URL, unwritable output path).
//!   2    network / HTTP / output-write failure.

use clap::Parser;
use std::path::PathBuf;
use std::process::ExitCode;

const DEFAULT_UA: &str = "philharmonic-dev-agent/1.0";

#[derive(Parser)]
#[command(
    name = "web-fetch",
    about = "HTTP(S) GET a URL. Body goes to stdout, or to <outfile> if given. \
             Fails on HTTP 4xx/5xx. User-Agent overridable via WEB_FETCH_UA env."
)]
struct Args {
    /// URL to fetch.
    url: String,
    /// Optional output file. If omitted, body is written to stdout.
    outfile: Option<PathBuf>,
}

fn main() -> ExitCode {
    let args = Args::parse();

    if args.url.trim().is_empty() {
        eprintln!("!!! web-fetch: URL cannot be empty");
        return ExitCode::from(1);
    }

    let ua = std::env::var("WEB_FETCH_UA").unwrap_or_else(|_| DEFAULT_UA.to_string());

    let response = match ureq::get(&args.url).header("User-Agent", &ua).call() {
        Ok(r) => r,
        // ureq 3.x flattens HTTP error responses into `StatusCode(code)` —
        // the response body is not attached on the error path, so we
        // print just the code. 4xx/5xx → Err by default
        // (`http_status_as_error = true`).
        Err(ureq::Error::StatusCode(code)) => {
            eprintln!("!!! web-fetch: HTTP {} for {}", code, args.url);
            return ExitCode::from(2);
        }
        Err(e) => {
            eprintln!("!!! web-fetch: fetch failed: {}", e);
            return ExitCode::from(2);
        }
    };

    let mut reader = response.into_body().into_reader();

    match args.outfile {
        Some(path) => {
            let mut f = match std::fs::File::create(&path) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!(
                        "!!! web-fetch: cannot create output file {}: {}",
                        path.display(),
                        e
                    );
                    return ExitCode::from(1);
                }
            };
            if let Err(e) = std::io::copy(&mut reader, &mut f) {
                eprintln!("!!! web-fetch: write to {} failed: {}", path.display(), e);
                return ExitCode::from(2);
            }
        }
        None => {
            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            if let Err(e) = std::io::copy(&mut reader, &mut handle) {
                eprintln!("!!! web-fetch: write to stdout failed: {}", e);
                return ExitCode::from(2);
            }
        }
    }

    ExitCode::SUCCESS
}
