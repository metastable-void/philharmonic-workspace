//! openai-chat — generic caller for the OpenAI chat-completion
//! API. One-shot, non-streaming.
//!
//! Two operating modes:
//!
//! - **Freeform mode** (default): user prompt from `--prompt`
//!   or stdin, optional `--system-prompt`, response's assistant
//!   message written verbatim to stdout. This is the mode used
//!   by `scripts/project-status.sh` and is the bin's historical
//!   behavior.
//! - **Fixture-capture mode**: triggered when `--output-schema`
//!   or any `--capture-*` flag is set. Constructs a structured-
//!   output request (`response_format: json_schema` for
//!   `openai_native`, or synthetic `tools` + `tool_choice` for
//!   `tool_call_fallback`), captures the outbound request body
//!   and/or the raw response body to disk as pretty-printed
//!   JSON. When `--output-schema` is set the assistant message
//!   is NOT printed to stdout (`content` would be a
//!   JSON-in-a-string or null-with-tool-calls, not useful as a
//!   print target).
//!
//! Usage:
//!   ./scripts/xtask.sh openai-chat -- \
//!       [--system-prompt <TEXT>] \
//!       [--prompt <TEXT>] \
//!       [--model <MODEL>] \
//!       [--output-schema <PATH>] \
//!       [--tool-call-fallback] \
//!       [--max-completion-tokens <N>] \
//!       [--temperature <F>] \
//!       [--top-p <F>] \
//!       [--stop <TOKEN> ...] \
//!       [--capture-request <PATH>] \
//!       [--capture-response <PATH>] \
//!       [--capture-dir <DIR>]
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
//!   0    freeform completion to stdout, or fixture capture succeeded.
//!   1    input error (missing key, empty prompt, bad flags,
//!        unreadable schema file, capture-write failure).
//!   2    network / HTTP / JSON-shape failure talking to OpenAI.

use clap::Parser;
use serde_json::{Map, Value, json};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const DEFAULT_MODEL: &str = "gpt-5.4";
const API_URL: &str = "https://api.openai.com/v1/chat/completions";
const ENV_FILE: &str = ".env";
const API_KEY_VAR: &str = "OPENAI_API_KEY";

#[derive(Parser)]
#[command(
    name = "openai-chat",
    about = "Generic OpenAI chat-completion caller. Freeform mode: user prompt from --prompt or stdin; \
             assistant message to stdout. Fixture-capture mode (--output-schema, --tool-call-fallback, \
             or --capture-*): writes request and/or response JSON to disk. API key from \
             $OPENAI_API_KEY or ./.env."
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
    /// Path to a JSON file containing a JSON Schema. Enables
    /// structured-output request construction. Without
    /// --tool-call-fallback, sends the schema via
    /// `response_format: {type: "json_schema", ...}` (openai_native
    /// dialect). With --tool-call-fallback, sends via a synthetic
    /// `tools` + `tool_choice` (tool_call_fallback dialect).
    #[arg(long)]
    output_schema: Option<PathBuf>,
    /// When set together with --output-schema, use the synthetic
    /// tool + tool_choice pattern instead of response_format.
    /// Errors if --output-schema is not also set.
    #[arg(long)]
    tool_call_fallback: bool,
    /// Cap on completion tokens (OpenAI `max_completion_tokens`).
    #[arg(long)]
    max_completion_tokens: Option<u32>,
    /// Sampling temperature (0.0–2.0 per OpenAI).
    #[arg(long)]
    temperature: Option<f64>,
    /// Nucleus sampling threshold.
    #[arg(long)]
    top_p: Option<f64>,
    /// Stop sequence. Repeatable: multiple --stop values become
    /// a JSON array in the request.
    #[arg(long)]
    stop: Vec<String>,
    /// Path to write the outbound request body (pretty JSON).
    /// Parent directory created if missing.
    #[arg(long)]
    capture_request: Option<PathBuf>,
    /// Path to write the raw response body (pretty JSON).
    /// Parent directory created if missing.
    #[arg(long)]
    capture_response: Option<PathBuf>,
    /// Shorthand for --capture-request <DIR>/request.json
    /// --capture-response <DIR>/response.json. Mutually
    /// exclusive with either individual --capture-* flag.
    #[arg(long)]
    capture_dir: Option<PathBuf>,
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

fn load_output_schema(path: &Path) -> Result<Value, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| format!("could not read output-schema {}: {e}", path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|e| format!("output-schema {} is not valid JSON: {e}", path.display()))
}

fn build_messages(system_prompt: Option<&str>, user_prompt: &str) -> Vec<Value> {
    let mut messages = Vec::with_capacity(2);
    if let Some(s) = system_prompt
        && !s.is_empty()
    {
        messages.push(json!({"role": "system", "content": s}));
    }
    messages.push(json!({"role": "user", "content": user_prompt}));
    messages
}

/// Pure request-body builder. Tests inspect the returned Value
/// directly to assert dialect-translation shape.
#[allow(clippy::too_many_arguments)]
fn build_request_body(
    model: &str,
    messages: Vec<Value>,
    output_schema: Option<&Value>,
    tool_call_fallback: bool,
    max_completion_tokens: Option<u32>,
    temperature: Option<f64>,
    top_p: Option<f64>,
    stop: &[String],
) -> Result<Value, String> {
    if tool_call_fallback && output_schema.is_none() {
        return Err("--tool-call-fallback requires --output-schema".to_owned());
    }

    let mut body = Map::new();
    body.insert("model".to_owned(), json!(model));
    body.insert("messages".to_owned(), json!(messages));

    if let Some(schema) = output_schema {
        if tool_call_fallback {
            body.insert(
                "tools".to_owned(),
                json!([{
                    "type": "function",
                    "function": {
                        "name": "emit_output",
                        "description": "Produce the structured output.",
                        "parameters": schema,
                    }
                }]),
            );
            body.insert(
                "tool_choice".to_owned(),
                json!({
                    "type": "function",
                    "function": {"name": "emit_output"}
                }),
            );
        } else {
            body.insert(
                "response_format".to_owned(),
                json!({
                    "type": "json_schema",
                    "json_schema": {
                        "name": "output",
                        "strict": true,
                        "schema": schema,
                    }
                }),
            );
        }
    }

    if let Some(n) = max_completion_tokens {
        body.insert("max_completion_tokens".to_owned(), json!(n));
    }
    if let Some(t) = temperature {
        body.insert("temperature".to_owned(), json!(t));
    }
    if let Some(p) = top_p {
        body.insert("top_p".to_owned(), json!(p));
    }
    if !stop.is_empty() {
        body.insert("stop".to_owned(), json!(stop));
    }

    Ok(Value::Object(body))
}

fn write_fixture(path: &Path, value: &Value) -> std::io::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::File::create(path)?;
    let pretty = serde_json::to_string_pretty(value)?;
    f.write_all(pretty.as_bytes())?;
    f.write_all(b"\n")?;
    Ok(())
}

fn main() -> ExitCode {
    let args = Args::parse();

    if args.tool_call_fallback && args.output_schema.is_none() {
        eprintln!("!!! openai-chat: --tool-call-fallback requires --output-schema");
        return ExitCode::from(1);
    }

    let (capture_request_path, capture_response_path) = match args.capture_dir.clone() {
        Some(dir) => {
            if args.capture_request.is_some() || args.capture_response.is_some() {
                eprintln!(
                    "!!! openai-chat: --capture-dir is mutually exclusive with --capture-request/--capture-response"
                );
                return ExitCode::from(1);
            }
            if let Err(e) = std::fs::create_dir_all(&dir) {
                eprintln!("!!! openai-chat: could not create {}: {e}", dir.display());
                return ExitCode::from(1);
            }
            (
                Some(dir.join("request.json")),
                Some(dir.join("response.json")),
            )
        }
        None => (args.capture_request.clone(), args.capture_response.clone()),
    };

    let api_key = match load_api_key() {
        Ok(k) => k,
        Err(e) => {
            eprintln!("!!! openai-chat: {e}");
            return ExitCode::from(1);
        }
    };

    let user_prompt = match args.prompt.clone() {
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

    let output_schema = match args.output_schema.as_deref() {
        Some(path) => match load_output_schema(path) {
            Ok(v) => Some(v),
            Err(e) => {
                eprintln!("!!! openai-chat: {e}");
                return ExitCode::from(1);
            }
        },
        None => None,
    };

    let messages = build_messages(args.system_prompt.as_deref(), &user_prompt);

    let request_body = match build_request_body(
        &args.model,
        messages,
        output_schema.as_ref(),
        args.tool_call_fallback,
        args.max_completion_tokens,
        args.temperature,
        args.top_p,
        &args.stop,
    ) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("!!! openai-chat: {e}");
            return ExitCode::from(1);
        }
    };

    if let Some(path) = &capture_request_path
        && let Err(e) = write_fixture(path, &request_body)
    {
        eprintln!("!!! openai-chat: failed to write {}: {e}", path.display());
        return ExitCode::from(1);
    }

    let body_bytes = match serde_json::to_vec(&request_body) {
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
        .send(&body_bytes[..])
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

    let response_value: Value = match serde_json::from_str(&body_str) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("!!! openai-chat: failed to parse response JSON: {e}");
            return ExitCode::from(2);
        }
    };

    if let Some(path) = &capture_response_path
        && let Err(e) = write_fixture(path, &response_value)
    {
        eprintln!("!!! openai-chat: failed to write {}: {e}", path.display());
        return ExitCode::from(1);
    }

    // Freeform mode: print the assistant's content to stdout.
    // Structured-output mode (--output-schema set): stay silent —
    // content would be a JSON-in-a-string (openai_native) or null
    // with a tool_calls payload (tool_call_fallback), neither of
    // which is a useful print target. The capture files are the
    // meaningful output of that mode.
    if args.output_schema.is_none() {
        let content = response_value
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str);
        match content {
            Some(content) => {
                if content.ends_with('\n') {
                    print!("{content}");
                } else {
                    println!("{content}");
                }
            }
            None => {
                eprintln!("!!! openai-chat: response contained no choices[0].message.content");
                return ExitCode::from(2);
            }
        }
    }

    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_messages() -> Vec<Value> {
        vec![json!({"role": "user", "content": "hi"})]
    }

    #[test]
    fn response_format_object_shape() {
        let schema = json!({"type": "object", "properties": {"x": {"type": "integer"}}});
        let body = build_request_body(
            "gpt-4o-mini",
            sample_messages(),
            Some(&schema),
            false,
            None,
            None,
            None,
            &[],
        )
        .expect("build_request_body must succeed");

        let rf = body
            .get("response_format")
            .expect("response_format present");
        assert_eq!(rf["type"], "json_schema");
        assert_eq!(rf["json_schema"]["name"], "output");
        assert_eq!(rf["json_schema"]["strict"], true);
        assert_eq!(rf["json_schema"]["schema"], schema);

        assert!(body.get("tools").is_none());
        assert!(body.get("tool_choice").is_none());
    }

    #[test]
    fn tool_call_fallback_object_shape() {
        let schema = json!({"type": "object", "properties": {"x": {"type": "integer"}}});
        let body = build_request_body(
            "gpt-4o-mini",
            sample_messages(),
            Some(&schema),
            true,
            None,
            None,
            None,
            &[],
        )
        .expect("build_request_body must succeed");

        let tools = body
            .get("tools")
            .and_then(Value::as_array)
            .expect("tools array present");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["type"], "function");
        assert_eq!(tools[0]["function"]["name"], "emit_output");
        assert_eq!(tools[0]["function"]["parameters"], schema);

        let choice = body.get("tool_choice").expect("tool_choice present");
        assert_eq!(choice["type"], "function");
        assert_eq!(choice["function"]["name"], "emit_output");

        assert!(body.get("response_format").is_none());
    }

    #[test]
    fn tool_call_fallback_without_schema_errors() {
        let err = build_request_body(
            "gpt-4o-mini",
            sample_messages(),
            None,
            true,
            None,
            None,
            None,
            &[],
        )
        .expect_err("must reject tool_call_fallback without schema");
        assert!(
            err.contains("--output-schema"),
            "error message must mention --output-schema: got {err}"
        );
    }

    #[test]
    fn generation_knobs_forwarded() {
        let body = build_request_body(
            "gpt-4o-mini",
            sample_messages(),
            None,
            false,
            Some(100),
            Some(0.2),
            Some(0.9),
            &["\n\n".to_owned(), "END".to_owned()],
        )
        .expect("build_request_body must succeed");

        assert_eq!(body["max_completion_tokens"], 100);
        assert_eq!(body["temperature"], 0.2);
        assert_eq!(body["top_p"], 0.9);
        assert_eq!(body["stop"], json!(["\n\n", "END"]));
    }

    #[test]
    fn generation_knobs_absent_when_unset() {
        let body = build_request_body(
            "gpt-4o-mini",
            sample_messages(),
            None,
            false,
            None,
            None,
            None,
            &[],
        )
        .expect("build_request_body must succeed");

        assert!(body.get("max_completion_tokens").is_none());
        assert!(body.get("temperature").is_none());
        assert!(body.get("top_p").is_none());
        assert!(body.get("stop").is_none());
    }

    #[test]
    fn messages_omit_empty_system_prompt() {
        let msgs = build_messages(Some(""), "hello");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["role"], "user");
    }

    #[test]
    fn messages_include_nonempty_system_prompt() {
        let msgs = build_messages(Some("you are helpful"), "hello");
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0]["role"], "system");
        assert_eq!(msgs[0]["content"], "you are helpful");
        assert_eq!(msgs[1]["role"], "user");
        assert_eq!(msgs[1]["content"], "hello");
    }

    #[test]
    fn capture_request_writes_pretty_json() {
        let dir = std::env::temp_dir().join(format!(
            "openai-chat-capture-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let path = dir.join("nested").join("request.json");

        let value = json!({"a": 1, "b": [2, 3], "c": {"d": "e"}});
        write_fixture(&path, &value).expect("write_fixture must succeed");

        let read = std::fs::read_to_string(&path).expect("written file must be readable");
        assert!(
            read.contains('\n'),
            "pretty-printed JSON must contain newlines"
        );
        assert!(
            read.ends_with('\n'),
            "file must end with a trailing newline"
        );

        let roundtrip: Value =
            serde_json::from_str(&read).expect("written file must be valid JSON");
        assert_eq!(roundtrip, value);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
