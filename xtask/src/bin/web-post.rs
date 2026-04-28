//! web-post — HTTP(S) POST a JSON payload from stdin.
//!
//! Usage:
//!   echo '{"text":"hello"}' | ./scripts/xtask.sh web-post -- <URL>
//!   echo '{"text":"hello"}' | ./scripts/xtask.sh web-post -- --header 'Authorization: Bearer tok' <URL>
//!
//! Reads the entire stdin as the request body, sends it as
//! `Content-Type: application/json` via POST to the given URL.
//! Prints the response body to stdout. Fails on HTTP 4xx/5xx.
//!
//! Headers: pass `--header 'Name: Value'` (repeatable) to add
//! custom request headers. `Content-Type: application/json` and
//! `User-Agent` are always set (User-Agent overridable via
//! `WEB_POST_UA` env).
//!
//! Exit codes:
//!   0    request succeeded; response body on stdout.
//!   1    input error (missing URL, stdin read failure).
//!   2    network / HTTP / output-write failure.

use clap::Parser;
use std::io::Read;
use std::process::ExitCode;

const DEFAULT_UA: &str = "philharmonic-dev-agent/1.0";

#[derive(Parser)]
#[command(
    name = "web-post",
    about = "HTTP(S) POST JSON from stdin to a URL. \
             Prints response body to stdout. Fails on HTTP 4xx/5xx."
)]
struct Args {
    /// Custom request headers (`Name: Value`). Repeatable.
    #[arg(long = "header", short = 'H', value_name = "HEADER")]
    headers: Vec<String>,
    /// URL to POST to.
    url: String,
}

fn main() -> ExitCode {
    let args = Args::parse();

    if args.url.trim().is_empty() {
        eprintln!("!!! web-post: URL cannot be empty");
        return ExitCode::from(1);
    }

    let mut body = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut body) {
        eprintln!("!!! web-post: failed to read stdin: {e}");
        return ExitCode::from(1);
    }

    let ua = std::env::var("WEB_POST_UA").unwrap_or_else(|_| DEFAULT_UA.to_string());

    let mut request = ureq::post(&args.url)
        .header("User-Agent", &ua)
        .header("Content-Type", "application/json");

    for header in &args.headers {
        let Some((name, value)) = header.split_once(':') else {
            eprintln!("!!! web-post: malformed header (expected 'Name: Value'): {header}");
            return ExitCode::from(1);
        };
        request = request.header(name.trim(), value.trim());
    }

    let response = match request.send(&body) {
        Ok(r) => r,
        Err(ureq::Error::StatusCode(code)) => {
            eprintln!("!!! web-post: HTTP {code} for {}", args.url);
            return ExitCode::from(2);
        }
        Err(e) => {
            eprintln!("!!! web-post: request failed: {e}");
            return ExitCode::from(2);
        }
    };

    let mut reader = response.into_body().into_reader();
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    if let Err(e) = std::io::copy(&mut reader, &mut handle) {
        eprintln!("!!! web-post: write to stdout failed: {e}");
        return ExitCode::from(2);
    }

    ExitCode::SUCCESS
}
