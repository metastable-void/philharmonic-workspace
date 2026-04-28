//! encode-json-str — read stdin, trim, print as a JSON string literal.
//!
//! Usage:
//!   echo 'hello world' | ./scripts/xtask.sh encode-json-str
//!
//! Reads all of stdin, trims leading/trailing whitespace, and
//! prints the result as a JSON-encoded string (with surrounding
//! quotes and all special characters escaped). Useful for
//! embedding arbitrary text into JSON payloads from shell scripts.
//!
//! Exit codes:
//!   0    success.
//!   1    stdin read failure.

use std::io::Read;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut input = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut input) {
        eprintln!("!!! encode-json-str: failed to read stdin: {e}");
        return ExitCode::from(1);
    }
    let trimmed = input.trim();
    let encoded = serde_json::to_string(trimmed).expect("string JSON encoding cannot fail");
    print!("{encoded}");
    ExitCode::SUCCESS
}
