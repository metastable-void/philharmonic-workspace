//! Philharmonic Chat agent UI and mock-testing server.

use std::{future, path::PathBuf, process};

use axum::Router;
use clap::{Args, Parser, Subcommand};
use mechanics_http_client::Client as HttpClient;
use tokio::net::TcpListener;

#[cfg(feature = "https")]
use philharmonic::server::https::{start_tls_axum_server, validate_tls_server_files};

mod config;
mod error;
mod mint;
mod routes;
mod static_assets;

use config::Config;
use routes::AppState;

const GIT_COMMIT_SHA: Option<&'static str> = option_env!("PHILHARMONIC_CHAT_GIT_COMMIT_SHA");

#[derive(Parser)]
#[command(
    name = "philharmonic-chat",
    version,
    about = "Philharmonic chat UI server"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    Serve(ServeArgs),
    Version,
}

#[derive(Args)]
struct ServeArgs {
    #[arg(long, default_value = "/etc/philharmonic/chat.toml")]
    config: PathBuf,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    if let Err(error) = run(Cli::parse()).await {
        eprintln!("philharmonic-chat: {error}");
        process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<(), String> {
    match cli.command.unwrap_or_else(|| {
        Command::Serve(ServeArgs {
            config: PathBuf::from("/etc/philharmonic/chat.toml"),
        })
    }) {
        Command::Serve(args) => serve(args).await,
        Command::Version => {
            match GIT_COMMIT_SHA {
                Some(sha) => println!("{} ({sha})", env!("CARGO_PKG_VERSION")),
                None => println!("{}", env!("CARGO_PKG_VERSION")),
            }
            Ok(())
        }
    }
}

async fn serve(args: ServeArgs) -> Result<(), String> {
    let config = Config::load(&args.config)?;
    validate_startup_config(&config)?;

    let client = HttpClient::builder()
        .user_agent(concat!("philharmonic-chat/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| format!("failed to build HTTP client: {error}"))?;

    let bind = config.chat.bind;
    let bind_h3 = config.chat.bind_h3;
    let server_config = config.clone();
    let state = AppState::new(config, client);
    let app = routes::router(state);
    let protocol = start_server(app, &server_config).await?;

    match bind_h3 {
        Some(addr) => eprintln!("philharmonic-chat listening on {bind} ({protocol}, h3 {addr})"),
        None => eprintln!("philharmonic-chat listening on {bind} ({protocol})"),
    }

    future::pending::<()>().await;
    Ok(())
}

fn validate_startup_config(config: &Config) -> Result<(), String> {
    config.validate()?;

    #[cfg(feature = "https")]
    if let Some(tls) = &config.tls {
        validate_tls_server_files(&tls.cert_path, &tls.key_path)?;
    }

    Ok(())
}

async fn start_server(app: Router, config: &Config) -> Result<&'static str, String> {
    #[cfg(feature = "https")]
    if let Some(tls) = &config.tls {
        start_tls_axum_server(
            app,
            config.chat.bind,
            config.chat.bind_h3,
            &tls.cert_path,
            &tls.key_path,
            "philharmonic-chat",
            "chat",
        )
        .await?;
        return Ok(if config.chat.bind_h3.is_some() {
            "https+h3"
        } else {
            "https"
        });
    }

    if config.chat.bind_h3.is_some() {
        return Err("HTTP/3 requires TLS; configure `[tls]`".to_string());
    }

    let listener = TcpListener::bind(config.chat.bind)
        .await
        .map_err(|error| format!("failed to bind chat HTTP listener: {error}"))?;
    tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, app).await {
            eprintln!("philharmonic-chat HTTP server stopped: {error}");
        }
    });
    Ok("http")
}
