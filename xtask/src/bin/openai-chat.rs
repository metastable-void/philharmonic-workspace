//! openai-chat — generic caller for the OpenAI chat-completion
//! API. One-shot, non-streaming. The response's assistant message
//! is written verbatim to stdout.
//!
//! Usage:
//!   ./scripts/xtask.sh openai-chat -- \
//!       [--system-prompt <TEXT>] \
//!       [--prompt <TEXT>] \
//!       [--model <MODEL>]
//!
//! User prompt comes from `--prompt <TEXT>` if given; otherwise
//! the bin reads all of stdin (so callers can pipe in large
//! assembled documents without worrying about argv length limits).
//!
//! API key resolution, in order:
//!   1. `$OPENAI_API_KEY` if set and non-empty.
//!   2. A `OPENAI_API_KEY=<value>` line in `./.env` at CWD. The
//!      workspace's `.gitignore` excludes `.env`, and callers
//!      invoked via `./scripts/xtask.sh` land at the workspace
//!      root, so this is the canonical location for the key.
//!
//! The `.env` parser is intentionally minimal: one `KEY=VALUE`
//! pair per line, blank / `#`-comment lines skipped, optional
//! leading `export `, matching surrounding `"` or `'` stripped.
//! No interpolation, no multi-line values. Anything more
//! elaborate belongs in a separate tool.
//!
//! Exit codes:
//!   0    completion written to stdout.
//!   1    input error (missing key, empty prompt, unreadable .env).
//!   2    network / HTTP / JSON-shape failure talking to OpenAI.

use clap::Parser;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::process::ExitCode;

const DEFAULT_MODEL: &str = "gpt-5.4";
const API_URL: &str = "https://api.openai.com/v1/chat/completions";
const ENV_FILE: &str = ".env";
const API_KEY_VAR: &str = "OPENAI_API_KEY";

#[derive(Parser)]
#[command(
    name = "openai-chat",
    about = "Generic OpenAI chat-completion caller. User prompt from --prompt or stdin; \
             optional --system-prompt; API key from $OPENAI_API_KEY or ./.env. \
             Writes the assistant message to stdout."
)]
struct Args {
    /// Inline system prompt. Omit to send only a user message.
    #[arg(long)]
    system_prompt: Option<String>,
    /// Inline user prompt. If omitted, all of stdin is read and
    /// used as the user prompt.
    #[arg(long)]
    prompt: Option<String>,
    /// Model identifier forwarded to the API `model` field.
    #[arg(long, default_value = DEFAULT_MODEL)]
    model: String,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<Message<'a>>,
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

fn load_api_key() -> Result<String, String> {
    if let Ok(v) = std::env::var(API_KEY_VAR)
        && !v.is_empty()
    {
        return Ok(v);
    }

    let contents = match std::fs::read_to_string(ENV_FILE) {
        Ok(s) => s,
        Err(e) => {
            return Err(format!(
                "{API_KEY_VAR} not set in environment and could not read {ENV_FILE}: {e}"
            ));
        }
    };

    for (idx, line) in contents.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let body = trimmed.strip_prefix("export ").unwrap_or(trimmed);
        let Some((key, raw_val)) = body.split_once('=') else {
            continue;
        };
        if key.trim() != API_KEY_VAR {
            continue;
        }
        let val = raw_val.trim();
        let val = match (val.as_bytes().first(), val.as_bytes().last()) {
            (Some(&b'"'), Some(&b'"')) | (Some(&b'\''), Some(&b'\'')) if val.len() >= 2 => {
                &val[1..val.len() - 1]
            }
            _ => val,
        };
        if val.is_empty() {
            return Err(format!(
                "{ENV_FILE} line {}: {API_KEY_VAR}= has empty value",
                idx + 1
            ));
        }
        return Ok(val.to_string());
    }

    Err(format!(
        "{API_KEY_VAR} not set in environment and no {API_KEY_VAR}= entry in {ENV_FILE}"
    ))
}

fn read_stdin() -> Result<String, std::io::Error> {
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;
    Ok(buf)
}

fn main() -> ExitCode {
    let args = Args::parse();

    let api_key = match load_api_key() {
        Ok(k) => k,
        Err(e) => {
            eprintln!("!!! openai-chat: {e}");
            return ExitCode::from(1);
        }
    };

    let user_prompt = match args.prompt {
        Some(p) => p,
        None => match read_stdin() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("!!! openai-chat: failed to read stdin: {e}");
                return ExitCode::from(1);
            }
        },
    };

    if user_prompt.trim().is_empty() {
        eprintln!("!!! openai-chat: user prompt is empty");
        return ExitCode::from(1);
    }

    let mut messages: Vec<Message> = Vec::with_capacity(2);
    if let Some(ref s) = args.system_prompt
        && !s.is_empty()
    {
        messages.push(Message {
            role: "system",
            content: s,
        });
    }
    messages.push(Message {
        role: "user",
        content: &user_prompt,
    });

    let req = ChatRequest {
        model: &args.model,
        messages,
    };

    let body = match serde_json::to_vec(&req) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("!!! openai-chat: failed to serialise request: {e}");
            return ExitCode::from(1);
        }
    };

    let auth = format!("Bearer {api_key}");

    let response = match ureq::post(API_URL)
        .header("Authorization", &auth)
        .header("Content-Type", "application/json")
        .send(&body[..])
    {
        Ok(r) => r,
        Err(ureq::Error::StatusCode(code)) => {
            eprintln!("!!! openai-chat: HTTP {code} from OpenAI");
            return ExitCode::from(2);
        }
        Err(e) => {
            eprintln!("!!! openai-chat: request failed: {e}");
            return ExitCode::from(2);
        }
    };

    let body_str = match response.into_body().read_to_string() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("!!! openai-chat: failed to read response body: {e}");
            return ExitCode::from(2);
        }
    };

    let parsed: ChatResponse = match serde_json::from_str(&body_str) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("!!! openai-chat: failed to parse response JSON: {e}");
            return ExitCode::from(2);
        }
    };

    let Some(first) = parsed.choices.into_iter().next() else {
        eprintln!("!!! openai-chat: response contained no choices");
        return ExitCode::from(2);
    };

    let content = first.message.content;
    if content.ends_with('\n') {
        print!("{content}");
    } else {
        println!("{content}");
    }

    ExitCode::SUCCESS
}
