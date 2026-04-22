//! codex-fmt — render a Codex rollout JSONL transcript into a
//! human-readable form with ANSI color escapes.
//!
//! Usage:
//!   ./scripts/xtask.sh codex-fmt -- [<path>]
//!   ./scripts/xtask.sh codex-fmt -- --no-color [<path>]
//!   <producer> | ./scripts/xtask.sh codex-fmt --
//!
//! `<path>` is a Codex rollout file (one JSON object per line)
//! like those under `$CODEX_HOME/sessions/YYYY/MM/DD/
//! rollout-*.jsonl`. If `<path>` is absent or `-`, the bin reads
//! from stdin so it can be chained with `tail -f` or a wrapper
//! script (`scripts/codex-logs.sh`).
//!
//! ## What it renders
//!
//! Records from the Codex rollout stream are grouped by the
//! top-level `"type"` field plus `payload.type`. The bin walks
//! each JSONL line and emits a compact, color-highlighted
//! summary:
//!
//! - `session_meta` — the first record; session ID, originator,
//!   cwd, model provider, git HEAD, cli version, timestamp.
//!   The embedded `base_instructions.text` (Codex system prompt)
//!   is intentionally omitted — it's the same every session and
//!   runs thousands of lines.
//! - `turn_context` — per-turn setup: model, personality,
//!   reasoning_effort, approval_policy, sandbox_policy.
//! - `event_msg:task_started` — turn start marker.
//! - `event_msg:token_count` — compact one-liner with total /
//!   input / cached / output / reasoning token counts.
//! - `event_msg:thread_name_updated` — Codex Companion thread
//!   name (the task description Claude sent).
//! - `event_msg:agent_message` and `event_msg:user_message` —
//!   **skipped** to avoid duplication with their
//!   `response_item:message` siblings, which carry the same text
//!   with role metadata.
//! - `response_item:reasoning` — summary if present, otherwise a
//!   placeholder noting the encrypted blob length. The encrypted
//!   content is **not** decoded or printed — it's opaque by
//!   design.
//! - `response_item:message` — `user`, `developer`, `assistant`
//!   roles, colored distinctly. Assistant phase (`commentary`
//!   vs. `final`) is surfaced next to the header.
//! - `response_item:function_call` — `>>> <tool>(args)` with
//!   pretty-printed inline JSON arguments.
//! - `response_item:function_call_output` — `<<<` with the
//!   returned text block indented.
//! - `response_item:custom_tool_call` and
//!   `response_item:custom_tool_call_output` — same treatment
//!   as function_call, used by `apply_patch` and other custom
//!   tools.
//!
//! Unrecognized record types are shown as a single dim line so
//! the operator can tell something was present without the bin
//! dropping it silently.
//!
//! ## Color handling
//!
//! ANSI color escapes are emitted only when stdout is a terminal
//! (checked via `IsTerminal`). Piping to a file, to `less`
//! without `-R`, or to another program switches them off. The
//! `--no-color` flag forces them off even on a TTY.
//!
//! ## Why an xtask bin rather than shell + jq
//!
//! `jq` is not POSIX and not shipped on every baseline (macOS
//! and Alpine both lack it by default). The workspace rule is
//! "if you'd reach for jq, write a Rust bin in `xtask/`" — see
//! `docs/design/13-conventions.md §In-tree workspace tooling`.
//! `codex-fmt` uses `serde_json` in-process, with no external
//! dependencies beyond what `xtask/` already pulls in.
//!
//! ## Exit codes
//!
//! - 0: ran to completion (or stdin / file EOF).
//! - 1: input file couldn't be opened.
//! - 2: CLI usage error.

use std::fs::File;
use std::io::{self, BufRead, BufReader, IsTerminal, Write};
use std::process::ExitCode;

use clap::Parser;
use serde_json::Value;

#[derive(Parser)]
#[command(
    name = "codex-fmt",
    about = "Render a Codex rollout JSONL transcript into human-readable form.",
    long_about = None,
)]
struct Args {
    /// Rollout JSONL file. Use "-" or omit to read from stdin.
    path: Option<String>,

    /// Force ANSI color off even on a TTY.
    #[arg(long)]
    no_color: bool,
}

/// ANSI escape codes grouped so switching to no-color mode is a
/// single struct swap, not a boolean check at every call site.
struct Palette {
    bold: &'static str,
    dim: &'static str,
    italic: &'static str,
    reset: &'static str,

    red: &'static str,
    green: &'static str,
    yellow: &'static str,
    blue: &'static str,
    magenta: &'static str,
    cyan: &'static str,
    gray: &'static str,
}

const COLOR: Palette = Palette {
    bold: "\x1b[1m",
    dim: "\x1b[2m",
    italic: "\x1b[3m",
    reset: "\x1b[0m",

    red: "\x1b[31m",
    green: "\x1b[32m",
    yellow: "\x1b[33m",
    blue: "\x1b[34m",
    magenta: "\x1b[35m",
    cyan: "\x1b[36m",
    gray: "\x1b[90m",
};

const NO_COLOR: Palette = Palette {
    bold: "",
    dim: "",
    italic: "",
    reset: "",
    red: "",
    green: "",
    yellow: "",
    blue: "",
    magenta: "",
    cyan: "",
    gray: "",
};

fn main() -> ExitCode {
    let args = Args::parse();

    let use_color = !args.no_color && io::stdout().is_terminal();
    let p: &Palette = if use_color { &COLOR } else { &NO_COLOR };

    let reader: Box<dyn BufRead> = match args.path.as_deref() {
        None | Some("-") => Box::new(BufReader::new(io::stdin().lock())),
        Some(path) => match File::open(path) {
            Ok(f) => Box::new(BufReader::new(f)),
            Err(e) => {
                eprintln!("codex-fmt: {}: {}", path, e);
                return ExitCode::from(1);
            }
        },
    };

    let stdout = io::stdout();
    let mut out = stdout.lock();

    // Read line-by-line; flush after each record so that piping
    // from `tail -f` streams cleanly.
    let mut buf = String::new();
    let mut reader = reader;
    loop {
        buf.clear();
        match reader.read_line(&mut buf) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) => {
                let _ = writeln!(
                    &mut out,
                    "{}codex-fmt: read error: {}{}",
                    p.red, e, p.reset
                );
                return ExitCode::from(1);
            }
        }
        let line = buf.trim_end_matches(['\n', '\r']);
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(line) {
            Ok(v) => render(&mut out, &v, p),
            Err(e) => {
                let _ = writeln!(
                    &mut out,
                    "{}[parse error] {}: {}{}",
                    p.red,
                    e,
                    truncate(line, 120),
                    p.reset
                );
            }
        }
        let _ = out.flush();
    }

    ExitCode::SUCCESS
}

fn render(out: &mut impl Write, v: &Value, p: &Palette) {
    let ts = v.get("timestamp").and_then(Value::as_str).unwrap_or("");
    // Pull HH:MM:SS out of the ISO-8601 "YYYY-MM-DDTHH:MM:SS.sssZ"
    // via index math; never panic on short / non-ASCII strings.
    let ts_short = ts.get(11..19).unwrap_or("--:--:--");

    let rtype = v.get("type").and_then(Value::as_str).unwrap_or("");
    let empty = Value::Null;
    let payload = v.get("payload").unwrap_or(&empty);
    let ptype = payload.get("type").and_then(Value::as_str).unwrap_or("");

    match (rtype, ptype) {
        ("session_meta", _) => render_session_meta(out, payload, p),
        ("turn_context", _) => render_turn_context(out, payload, ts_short, p),

        ("event_msg", "task_started") => render_task_started(out, payload, ts_short, p),
        ("event_msg", "token_count") => render_token_count(out, payload, ts_short, p),
        ("event_msg", "thread_name_updated") => render_thread_name(out, payload, ts_short, p),
        // Intentionally skipped — duplicated by response_item:message.
        ("event_msg", "agent_message") | ("event_msg", "user_message") => {}

        ("response_item", "reasoning") => render_reasoning(out, payload, ts_short, p),
        ("response_item", "message") => render_message(out, payload, ts_short, p),
        ("response_item", "function_call") => render_function_call(out, payload, ts_short, p),
        ("response_item", "function_call_output") => {
            render_function_call_output(out, payload, ts_short, p)
        }
        ("response_item", "custom_tool_call") => render_custom_tool_call(out, payload, ts_short, p),
        ("response_item", "custom_tool_call_output") => {
            render_custom_tool_call_output(out, payload, ts_short, p)
        }

        _ => {
            let _ = writeln!(
                out,
                "{}[{} / {}]{} {}{}{}",
                p.dim,
                rtype,
                ptype,
                p.reset,
                p.gray,
                ts_short,
                p.reset,
            );
        }
    }
}

fn render_session_meta(out: &mut impl Write, payload: &Value, p: &Palette) {
    let id = payload.get("id").and_then(Value::as_str).unwrap_or("?");
    let ts = payload.get("timestamp").and_then(Value::as_str).unwrap_or("?");
    let cwd = payload.get("cwd").and_then(Value::as_str).unwrap_or("?");
    let originator = payload
        .get("originator")
        .and_then(Value::as_str)
        .unwrap_or("?");
    let source = payload.get("source").and_then(Value::as_str).unwrap_or("?");
    let model_provider = payload
        .get("model_provider")
        .and_then(Value::as_str)
        .unwrap_or("?");
    let cli = payload
        .get("cli_version")
        .and_then(Value::as_str)
        .unwrap_or("?");

    let git_hash = payload
        .pointer("/git/commit_hash")
        .and_then(Value::as_str)
        .unwrap_or("?");
    let git_branch = payload
        .pointer("/git/branch")
        .and_then(Value::as_str)
        .unwrap_or("?");
    let git_repo = payload
        .pointer("/git/repository_url")
        .and_then(Value::as_str)
        .unwrap_or("?");

    let _ = writeln!(
        out,
        "{}{}=== SESSION START ==={}",
        p.bold, p.cyan, p.reset
    );
    let _ = writeln!(out, "  id:         {}", id);
    let _ = writeln!(out, "  started:    {}", ts);
    let _ = writeln!(out, "  originator: {} ({})", originator, source);
    let _ = writeln!(out, "  cwd:        {}", cwd);
    let _ = writeln!(out, "  provider:   {}", model_provider);
    let _ = writeln!(out, "  cli:        {}", cli);
    let _ = writeln!(
        out,
        "  git:        {} @ {} ({})",
        git_branch,
        short_hash(git_hash),
        git_repo
    );
    let _ = writeln!(out);
}

fn render_turn_context(out: &mut impl Write, payload: &Value, ts_short: &str, p: &Palette) {
    let turn = payload
        .get("turn_id")
        .and_then(Value::as_str)
        .unwrap_or("?");
    let cwd = payload.get("cwd").and_then(Value::as_str).unwrap_or("?");
    let approval = payload
        .get("approval_policy")
        .and_then(Value::as_str)
        .unwrap_or("?");
    let model = payload.get("model").and_then(Value::as_str).unwrap_or("?");
    let personality = payload
        .get("personality")
        .and_then(Value::as_str)
        .unwrap_or("?");
    let reasoning_effort = payload
        .pointer("/collaboration_mode/settings/reasoning_effort")
        .and_then(Value::as_str)
        .unwrap_or("?");
    let sandbox_type = payload
        .pointer("/sandbox_policy/type")
        .and_then(Value::as_str)
        .unwrap_or("?");
    let network = payload
        .pointer("/sandbox_policy/network_access")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let _ = writeln!(
        out,
        "{}{} {}TURN{} {}",
        p.gray,
        ts_short,
        p.bold,
        p.reset,
        short_id(turn),
    );
    let _ = writeln!(
        out,
        "  model={} personality={} reasoning={}",
        model, personality, reasoning_effort
    );
    let _ = writeln!(
        out,
        "  approval={} sandbox={} network={}",
        approval, sandbox_type, network
    );
    let _ = writeln!(out, "  cwd={}", cwd);
    let _ = writeln!(out);
}

fn render_task_started(out: &mut impl Write, payload: &Value, ts_short: &str, p: &Palette) {
    let turn = payload
        .get("turn_id")
        .and_then(Value::as_str)
        .unwrap_or("?");
    let ctx_window = payload
        .get("model_context_window")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let _ = writeln!(
        out,
        "{}{} {}TASK STARTED{} turn={} ctx_window={}",
        p.gray,
        ts_short,
        p.bold,
        p.reset,
        short_id(turn),
        ctx_window
    );
}

fn render_token_count(out: &mut impl Write, payload: &Value, ts_short: &str, p: &Palette) {
    let total = payload
        .pointer("/info/total_token_usage/total_tokens")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let input = payload
        .pointer("/info/total_token_usage/input_tokens")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let cached = payload
        .pointer("/info/total_token_usage/cached_input_tokens")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let output = payload
        .pointer("/info/total_token_usage/output_tokens")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let reasoning = payload
        .pointer("/info/total_token_usage/reasoning_output_tokens")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let _ = writeln!(
        out,
        "{}{} tokens: total={} (in={} cached={} out={} reasoning={}){}",
        p.dim, ts_short, total, input, cached, output, reasoning, p.reset
    );
}

fn render_thread_name(out: &mut impl Write, payload: &Value, ts_short: &str, p: &Palette) {
    let name = payload
        .get("thread_name")
        .and_then(Value::as_str)
        .unwrap_or("?");
    let _ = writeln!(
        out,
        "{}{} thread: {}{}{}{}",
        p.gray, ts_short, p.italic, name, p.reset, p.reset
    );
}

fn render_reasoning(out: &mut impl Write, payload: &Value, ts_short: &str, p: &Palette) {
    // Some models include a readable `summary` array alongside the
    // encrypted reasoning. Print the summary when present; skip
    // the encrypted blob content, just note its size so the
    // operator knows a reasoning block happened.
    let summary_lines: Vec<&str> = payload
        .get("summary")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.get("text").and_then(Value::as_str))
                .collect()
        })
        .unwrap_or_default();

    let encrypted_len = payload
        .get("encrypted_content")
        .and_then(Value::as_str)
        .map(str::len)
        .unwrap_or(0);

    if summary_lines.is_empty() {
        let _ = writeln!(
            out,
            "{}{} {}[reasoning]{} {}encrypted ({} chars) — skipped{}",
            p.gray, ts_short, p.italic, p.reset, p.dim, encrypted_len, p.reset
        );
    } else {
        let _ = writeln!(
            out,
            "{}{} {}[reasoning summary]{}",
            p.gray, ts_short, p.italic, p.reset
        );
        for line in summary_lines {
            for wrapped in line.lines() {
                let _ = writeln!(out, "  {}{}{}", p.dim, wrapped, p.reset);
            }
        }
    }
}

fn render_message(out: &mut impl Write, payload: &Value, ts_short: &str, p: &Palette) {
    let role = payload
        .get("role")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let phase = payload.get("phase").and_then(Value::as_str);

    // Color-tag per role.
    let (role_color, role_label) = match role {
        "user" => (p.green, "USER"),
        "developer" => (p.yellow, "DEVELOPER"),
        "assistant" => (p.blue, "ASSISTANT"),
        _ => (p.magenta, role),
    };

    let phase_suffix = match phase {
        Some(ph) => format!(" {}({}){}", p.dim, ph, p.reset),
        None => String::new(),
    };

    let _ = writeln!(
        out,
        "{}{} {}{}:{}{}",
        p.gray, ts_short, role_color, role_label, p.reset, phase_suffix
    );

    let content = payload.get("content").and_then(Value::as_array);
    if let Some(items) = content {
        for item in items {
            let text = item
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or("");
            if text.is_empty() {
                continue;
            }
            for line in text.lines() {
                let _ = writeln!(out, "  {}", line);
            }
        }
    }
    let _ = writeln!(out);
}

fn render_function_call(out: &mut impl Write, payload: &Value, ts_short: &str, p: &Palette) {
    let name = payload.get("name").and_then(Value::as_str).unwrap_or("?");
    let call_id = payload
        .get("call_id")
        .and_then(Value::as_str)
        .unwrap_or("?");
    let args_raw = payload
        .get("arguments")
        .and_then(Value::as_str)
        .unwrap_or("");

    let args_display = match serde_json::from_str::<Value>(args_raw) {
        Ok(parsed) => {
            // Pretty JSON when it's big; inline when short.
            let compact = serde_json::to_string(&parsed).unwrap_or_else(|_| args_raw.to_owned());
            if compact.len() <= 120 {
                compact
            } else {
                serde_json::to_string_pretty(&parsed).unwrap_or_else(|_| args_raw.to_owned())
            }
        }
        Err(_) => args_raw.to_owned(),
    };

    let _ = writeln!(
        out,
        "{}{} {}>>> {}{}  {}[call {}]{}",
        p.gray,
        ts_short,
        p.magenta,
        name,
        p.reset,
        p.dim,
        short_id(call_id),
        p.reset,
    );
    for line in args_display.lines() {
        let _ = writeln!(out, "  {}{}{}", p.magenta, line, p.reset);
    }
}

fn render_function_call_output(
    out: &mut impl Write,
    payload: &Value,
    ts_short: &str,
    p: &Palette,
) {
    let call_id = payload
        .get("call_id")
        .and_then(Value::as_str)
        .unwrap_or("?");
    let output = payload
        .get("output")
        .and_then(Value::as_str)
        .unwrap_or("");
    let line_count = output.lines().count();

    let _ = writeln!(
        out,
        "{}{} {}<<<{} {}[call {}, {} line{}]{}",
        p.gray,
        ts_short,
        p.cyan,
        p.reset,
        p.dim,
        short_id(call_id),
        line_count,
        if line_count == 1 { "" } else { "s" },
        p.reset,
    );
    for line in output.lines() {
        let _ = writeln!(out, "  {}{}{}", p.gray, line, p.reset);
    }
    let _ = writeln!(out);
}

fn render_custom_tool_call(out: &mut impl Write, payload: &Value, ts_short: &str, p: &Palette) {
    let name = payload.get("name").and_then(Value::as_str).unwrap_or("?");
    let call_id = payload
        .get("call_id")
        .and_then(Value::as_str)
        .unwrap_or("?");
    let status = payload.get("status").and_then(Value::as_str).unwrap_or("");
    let input = payload.get("input").and_then(Value::as_str).unwrap_or("");

    let status_suffix = if status.is_empty() {
        String::new()
    } else {
        format!(" {}[{}]{}", p.dim, status, p.reset)
    };

    let _ = writeln!(
        out,
        "{}{} {}>>> {}{} {}[custom call {}]{}{}",
        p.gray,
        ts_short,
        p.magenta,
        name,
        p.reset,
        p.dim,
        short_id(call_id),
        p.reset,
        status_suffix,
    );
    for line in input.lines() {
        let _ = writeln!(out, "  {}{}{}", p.magenta, line, p.reset);
    }
}

fn render_custom_tool_call_output(
    out: &mut impl Write,
    payload: &Value,
    ts_short: &str,
    p: &Palette,
) {
    let call_id = payload
        .get("call_id")
        .and_then(Value::as_str)
        .unwrap_or("?");
    let output = payload
        .get("output")
        .and_then(Value::as_str)
        .unwrap_or("");
    let line_count = output.lines().count();

    let _ = writeln!(
        out,
        "{}{} {}<<<{} {}[custom call {}, {} line{}]{}",
        p.gray,
        ts_short,
        p.cyan,
        p.reset,
        p.dim,
        short_id(call_id),
        line_count,
        if line_count == 1 { "" } else { "s" },
        p.reset,
    );
    for line in output.lines() {
        let _ = writeln!(out, "  {}{}{}", p.gray, line, p.reset);
    }
    let _ = writeln!(out);
}

fn short_id(s: &str) -> String {
    // UUIDs and call_ids are long; eight characters is enough to
    // disambiguate within a session and keeps lines readable.
    // `.get(..8)` returns `Option<&str>` so this is panic-free on
    // short inputs.
    s.get(..8).unwrap_or(s).to_string()
}

fn short_hash(s: &str) -> String {
    s.get(..12).unwrap_or(s).to_string()
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        // Respect char boundaries so we never slice mid-codepoint.
        let mut end = max;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...", s.get(..end).unwrap_or(""))
    }
}
