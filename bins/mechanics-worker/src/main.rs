//! Mechanics worker — JavaScript execution HTTP service.

use std::collections::HashSet;
use std::path::Path;
use std::process;
use std::time::Duration;

use clap::Parser;
use philharmonic::mechanics::{MechanicsPoolConfig, MechanicsServer};
use philharmonic::mechanics_core::job::MechanicsExecutionLimits;
use philharmonic::server::cli::{
    BaseArgs, BaseCommand, default_serve_command, resolve_config_paths,
};
use philharmonic::server::config::load_config_defaulting_missing;
use philharmonic::server::install::{self, InstallPlan};
use philharmonic::server::reload::ReloadHandle;

mod config;
use config::MechanicsWorkerConfig;

const DEFAULT_CONFIG: &str = r#"bind = "127.0.0.1:3001"
# Empty list = every request returns 401 (fail-closed).
# Add at least one token to accept traffic.
tokens = []

[pool]
execution_timeout_secs = 3600
run_timeout_secs = 3600
"#;

#[derive(Parser)]
#[command(
    name = "mechanics-worker",
    version,
    about = "Philharmonic mechanics JS executor"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<BaseCommand>,
}

#[tokio::main]
async fn main() {
    if let Err(error) = run(Cli::parse()).await {
        eprintln!("mechanics-worker: {error}");
        process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<(), String> {
    match cli.command.unwrap_or_else(default_serve_command) {
        BaseCommand::Version => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        BaseCommand::Serve(args) => serve(args).await,
        BaseCommand::Install(args) => install::execute_install(&InstallPlan {
            service_name: "mechanics-worker".to_string(),
            binary_name: "mechanics-worker".to_string(),
            description: "Philharmonic mechanics JS executor".to_string(),
            config_file_name: "mechanics.toml".to_string(),
            default_config_content: DEFAULT_CONFIG.to_string(),
            args,
        }),
        BaseCommand::GenSigningKey(args) => philharmonic::server::keygen::gen_signing_key(&args),
        BaseCommand::GenSck(args) => philharmonic::server::keygen::gen_sck(&args),
        BaseCommand::GenRealmKey(args) => philharmonic::server::keygen::gen_realm_key(&args),
        BaseCommand::Bootstrap(_) => {
            Err("bootstrap is only supported by philharmonic-api".to_string())
        }
    }
}

async fn serve(args: BaseArgs) -> Result<(), String> {
    let (primary, drop_in) = resolve_config_paths("mechanics", &args);
    let mut config = load_worker_config(&primary, &drop_in, &args)?;
    if let Some(bind) = args.bind {
        config.bind = bind;
    }
    if let Some(bind_h3) = args.bind_h3 {
        config.bind_h3 = Some(bind_h3);
    }

    let pool_config = build_pool_config(&config)?;
    let server = MechanicsServer::new(pool_config)
        .map_err(|error| format!("failed to create mechanics server: {error}"))?;

    for token in &config.tokens {
        server.add_token(token.clone());
    }

    let bind = config.bind;
    let bind_h3 = config.bind_h3;
    let protocol = start_server(&server, &config)?;
    let mut token_count = normalized_token_count(&config.tokens);
    match bind_h3 {
        Some(addr) => eprintln!("mechanics-worker listening on {bind} ({protocol}, h3 {addr})"),
        None => eprintln!("mechanics-worker listening on {bind} ({protocol})"),
    }
    eprintln!("loaded {token_count} bearer token(s)");

    let reload_handle = ReloadHandle::new()
        .map_err(|error| format!("failed to install SIGHUP reload handler: {error}"))?;

    loop {
        reload_handle.notified().await;
        match load_worker_config(&primary, &drop_in, &args) {
            Ok(mut reloaded) => {
                if let Some(bind) = args.bind {
                    reloaded.bind = bind;
                }
                if let Some(bind_h3) = args.bind_h3 {
                    reloaded.bind_h3 = Some(bind_h3);
                }
                let new_token_count = normalized_token_count(&reloaded.tokens);
                log_tls_reload_note(&reloaded);
                server.replace_tokens(reloaded.tokens);
                log_token_reload(token_count, new_token_count);
                token_count = new_token_count;
            }
            Err(error) => {
                eprintln!("mechanics-worker reload failed: {error}");
            }
        }
    }
}

fn load_worker_config(
    primary: &Path,
    drop_in: &Path,
    args: &BaseArgs,
) -> Result<MechanicsWorkerConfig, String> {
    let (config, defaulted) = load_config_defaulting_missing::<MechanicsWorkerConfig>(
        primary,
        drop_in,
        args.config.is_none(),
    )
    .map_err(|error| error.to_string())?;
    if defaulted {
        eprintln!(
            "mechanics-worker config {} not found; using built-in defaults",
            primary.display()
        );
    }
    Ok(config)
}

fn build_pool_config(config: &MechanicsWorkerConfig) -> Result<MechanicsPoolConfig, String> {
    let max_memory = u64::try_from(config.pool.max_memory)
        .map_err(|_| "pool.max_memory is too large for mechanics execution limits".to_string())?;
    let limits = MechanicsExecutionLimits::new(
        Duration::from_secs(config.pool.execution_timeout_secs),
        max_memory,
        config.pool.max_stack,
        config.pool.max_output,
    )
    .map_err(|error| format!("invalid mechanics execution limits: {error}"))?;

    Ok(MechanicsPoolConfig::default()
        .with_execution_limits(limits)
        .with_run_timeout(Duration::from_secs(config.pool.run_timeout_secs))
        .with_default_http_timeout_ms(Some(config.pool.default_http_timeout_ms)))
}

fn start_server(
    server: &MechanicsServer,
    config: &MechanicsWorkerConfig,
) -> Result<&'static str, String> {
    #[cfg(feature = "https")]
    if let Some(tls) = &config.tls {
        let tls_config = read_tls_config(tls)?;
        if let Some(bind_h3) = config.bind_h3 {
            let h3_config = philharmonic::mechanics::Http3ServerConfig {
                bind_h3: Some(bind_h3),
                ..philharmonic::mechanics::Http3ServerConfig::default()
            };
            server
                .run_tls_with_h3(config.bind, tls_config, Some(h3_config))
                .map_err(|error| format!("failed to start HTTPS/H3 mechanics server: {error}"))?;
            return Ok("https+h3");
        }
        server
            .run_tls(config.bind, tls_config)
            .map_err(|error| format!("failed to start HTTPS mechanics server: {error}"))?;
        return Ok("https");
    }

    if config.bind_h3.is_some() {
        return Err("HTTP/3 requires TLS; configure `[tls]`".to_string());
    }

    server
        .run(config.bind)
        .map_err(|error| format!("failed to start HTTP mechanics server: {error}"))?;
    Ok("http")
}

#[cfg(feature = "https")]
fn read_tls_config(
    tls: &config::TlsFileConfig,
) -> Result<philharmonic::mechanics::TlsConfig, String> {
    let cert_bytes = std::fs::read(&tls.cert_path).map_err(|error| {
        format!(
            "failed to read TLS certificate file {}: {error}",
            tls.cert_path.display()
        )
    })?;
    let key_bytes = std::fs::read(&tls.key_path).map_err(|error| {
        format!(
            "failed to read TLS private key file {}: {error}",
            tls.key_path.display()
        )
    })?;
    philharmonic::mechanics::TlsConfig::from_pem(&cert_bytes, &key_bytes)
        .map_err(|error| format!("failed to parse TLS certificate/key: {error}"))
}

fn normalized_token_count(tokens: &[String]) -> usize {
    tokens
        .iter()
        .map(|token| token.trim())
        .filter(|token| !token.is_empty())
        .collect::<HashSet<_>>()
        .len()
}

fn log_token_reload(old_count: usize, new_count: usize) {
    if old_count == new_count {
        eprintln!("mechanics-worker reloaded config; bearer tokens unchanged ({new_count})");
    } else {
        eprintln!(
            "mechanics-worker reloaded config; bearer tokens changed: {old_count} -> {new_count}"
        );
    }
}

#[cfg(feature = "https")]
fn log_tls_reload_note(config: &MechanicsWorkerConfig) {
    if let Some(tls) = &config.tls {
        match read_tls_config(tls) {
            Ok(_) => eprintln!(
                "mechanics-worker re-read TLS certificate/key; restart required to apply TLS changes"
            ),
            Err(error) => eprintln!("mechanics-worker TLS reload check failed: {error}"),
        }
    }
}

#[cfg(not(feature = "https"))]
fn log_tls_reload_note(_config: &MechanicsWorkerConfig) {}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use super::{MechanicsWorkerConfig, start_server};

    #[test]
    fn bind_h3_without_tls_errors_before_binding_http() {
        let mut config = MechanicsWorkerConfig {
            bind: SocketAddr::from(([127, 0, 0, 1], 0)),
            ..MechanicsWorkerConfig::default()
        };
        config.bind_h3 = Some(SocketAddr::from(([127, 0, 0, 1], 0)));
        let server =
            philharmonic::mechanics::MechanicsServer::new(Default::default()).expect("server");

        let error = start_server(&server, &config).expect_err("HTTP/3 without TLS must fail");

        assert_eq!(error, "HTTP/3 requires TLS; configure `[tls]`");
    }
}
