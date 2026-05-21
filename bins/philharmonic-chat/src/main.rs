//! Philharmonic Chat — agent-facing chat UI plus mock-testing
//! harness. See `README.md` next to this crate's `Cargo.toml`
//! for the design; this binary is currently a scaffold.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use serde::Deserialize;

#[derive(Parser)]
#[command(
    name = "philharmonic-chat",
    version,
    about = "Philharmonic chat UI server"
)]
struct Cli {
    /// Path to the chat bin's TOML configuration file.
    #[arg(long)]
    config: PathBuf,
}

// Field-level `dead_code` will fire until the server body is
// written; the structs are checked into the scaffold because
// they document the config contract README.md describes. Drop
// the allow when fields start being consumed.
#[allow(dead_code)]
#[derive(Deserialize)]
struct Config {
    chat: ChatConfig,
    tls: Option<TlsConfig>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct ChatConfig {
    bind: String,
    bind_h3: Option<String>,
    api_url: String,
    agent_token: String,
    minting_token: String,
    chat_uuid: String,
    notify_instance_uuid: String,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct TlsConfig {
    cert_path: PathBuf,
    key_path: PathBuf,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let text = match std::fs::read_to_string(&cli.config) {
        Ok(text) => text,
        Err(err) => {
            eprintln!(
                "philharmonic-chat: cannot read {}: {err}",
                cli.config.display()
            );
            return ExitCode::from(2);
        }
    };
    let _config: Config = match toml::from_str(&text) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!(
                "philharmonic-chat: cannot parse {}: {err}",
                cli.config.display()
            );
            return ExitCode::from(2);
        }
    };
    eprintln!("philharmonic-chat: server stub; implementation pending — see README.md");
    ExitCode::from(0)
}
