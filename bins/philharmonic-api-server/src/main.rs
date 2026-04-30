use std::collections::HashMap;
use std::path::Path;
use std::process;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, Response, Uri};
use axum::routing::any;
use axum::{Router, extract::State};
use clap::Parser;
use philharmonic::api::{
    PhilharmonicApiBuilder, RateLimitBucketConfig, RateLimitConfig, StubExecutor, StubLowerer,
};
use philharmonic::connector_client::LowererSigningKey;
use philharmonic::connector_common::{MLKEM768_PUBLIC_KEY_LEN, RealmId, RealmPublicKey};
use philharmonic::connector_router::{
    DispatchConfig, HyperForwarder, RouterState, router as connector_router,
};
use philharmonic::policy::{
    ApiSigningKey, ApiVerifyingKeyEntry, ApiVerifyingKeyRegistry, Sck, VerifyingKey,
};
use philharmonic::server::cli::{BaseArgs, BaseCommand, resolve_config_paths};
use philharmonic::server::config::{ConfigError, load_config};
use philharmonic::server::install::{self, InstallPlan};
use philharmonic::server::reload::ReloadHandle;
use philharmonic::store_sqlx_mysql::{SinglePool, SqlStore, migrate};
use philharmonic::workflow::{ConfigLowerer, StepExecutor};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tower::ServiceExt;
use zeroize::Zeroizing;

mod config;
mod executor;
mod lowerer;
mod scope;
mod security_headers;

use config::ApiConfig;
use executor::HttpStepExecutor;
use lowerer::ConnectorConfigLowerer;
use scope::HeaderBasedScopeResolver;

const ED25519_KEY_BYTES: usize = 32;
const SCK_KEY_BYTES: usize = 32;
const MLKEM_PUBLIC_KEY_BYTES: usize = MLKEM768_PUBLIC_KEY_LEN;
const X25519_PUBLIC_KEY_BYTES: usize = 32;
const DEFAULT_CONFIG: &str = r#"bind = "127.0.0.1:3000"
database_url = "mysql://philharmonic@localhost/philharmonic"
issuer = "philharmonic"
"#;

#[derive(Parser)]
#[command(
    name = "philharmonic-api",
    version,
    about = "Philharmonic public API server"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<BaseCommand>,
}

#[derive(Clone)]
struct DynamicRouter {
    router: Arc<RwLock<Router>>,
}

#[derive(Clone)]
struct LongLivedState {
    pool: SinglePool,
    signing_key: ApiSigningKey,
    sck_bytes: Option<Zeroizing<[u8; SCK_KEY_BYTES]>>,
}

#[derive(Clone, Copy)]
struct RuntimeCounts {
    verifying_keys: usize,
    connector_realms: usize,
}

struct Runtime {
    router: Router,
    counts: RuntimeCounts,
}

#[tokio::main]
async fn main() {
    if let Err(error) = run(Cli::parse()).await {
        eprintln!("philharmonic-api: {error}");
        process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<(), String> {
    match cli.command.unwrap_or_else(default_command) {
        BaseCommand::Version => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        BaseCommand::Serve(args) => serve(args).await,
        BaseCommand::Install(args) => install::execute_install(&InstallPlan {
            service_name: "philharmonic-api".to_string(),
            binary_name: "philharmonic-api".to_string(),
            description: "Philharmonic public API server".to_string(),
            config_file_name: "api.toml".to_string(),
            default_config_content: DEFAULT_CONFIG.to_string(),
            args,
        }),
    }
}

fn default_command() -> BaseCommand {
    BaseCommand::Serve(BaseArgs {
        config: None,
        config_dir: None,
        bind: None,
    })
}

async fn serve(args: BaseArgs) -> Result<(), String> {
    let (primary, drop_in) = resolve_config_paths("api", &args);
    let mut config = load_api_config(&primary, &drop_in, &args)?;
    if let Some(bind) = args.bind {
        config.bind = bind;
    }

    let pool = SinglePool::connect(&config.database_url)
        .await
        .map_err(|error| format!("database connection failed: {error}"))?;
    eprintln!("philharmonic-api: running schema migration");
    migrate(pool.pool())
        .await
        .map_err(|error| format!("schema migration failed: {error}"))?;
    let signing_key = load_signing_key(&config)?;
    let sck_bytes = load_sck_bytes(&config)?;
    let state = LongLivedState {
        pool,
        signing_key,
        sck_bytes,
    };

    let runtime = build_runtime(&config, &state)?;
    let dynamic = DynamicRouter {
        router: Arc::new(RwLock::new(runtime.router)),
    };
    let app =
        dynamic_router(dynamic.clone()).layer(axum::middleware::from_fn(security_headers::inject));

    let bind = config.bind;
    let protocol = start_server(app, &config).await?;
    let mut counts = runtime.counts;
    eprintln!("philharmonic-api listening on {bind} ({protocol})");
    log_loaded_counts(counts);

    let reload_handle = ReloadHandle::new()
        .map_err(|error| format!("failed to install SIGHUP reload handler: {error}"))?;

    loop {
        reload_handle.notified().await;
        match load_api_config(&primary, &drop_in, &args) {
            Ok(mut reloaded) => {
                if let Some(bind) = args.bind {
                    reloaded.bind = bind;
                }
                match build_runtime(&reloaded, &state) {
                    Ok(runtime) => {
                        log_tls_reload_note(&reloaded);
                        *dynamic.router.write().await = runtime.router;
                        log_reload(counts, runtime.counts);
                        counts = runtime.counts;
                    }
                    Err(error) => {
                        eprintln!("philharmonic-api reload failed: {error}");
                    }
                }
            }
            Err(error) => {
                eprintln!("philharmonic-api reload failed: {error}");
            }
        }
    }
}

fn load_api_config(primary: &Path, drop_in: &Path, args: &BaseArgs) -> Result<ApiConfig, String> {
    match load_config::<ApiConfig>(primary, drop_in) {
        Ok(config) => Ok(config),
        Err(ConfigError::Io(error))
            if error.kind() == std::io::ErrorKind::NotFound && args.config.is_none() =>
        {
            eprintln!(
                "philharmonic-api config {} not found; using built-in defaults",
                primary.display()
            );
            Ok(ApiConfig::default())
        }
        Err(error) => Err(error.to_string()),
    }
}

fn build_runtime(config: &ApiConfig, state: &LongLivedState) -> Result<Runtime, String> {
    let registry = build_verifying_key_registry(&config.verifying_keys)?;
    let connector = build_connector_routes(config)?;
    let rate_limit = build_rate_limit_config(config.rate_limit.as_ref());
    let step_executor = build_step_executor(config)?;
    let config_lowerer = build_config_lowerer(config)?;
    let store = SqlStore::new(state.pool.clone());

    let mut builder = PhilharmonicApiBuilder::new()
        .request_scope_resolver(Arc::new(HeaderBasedScopeResolver))
        .store(Arc::new(store))
        .api_verifying_key_registry(registry)
        .api_signing_key(state.signing_key.clone())
        .issuer(config.issuer.clone())
        .step_executor(step_executor)
        .config_lowerer(config_lowerer)
        .key_version(config.sck_key_version)
        .rate_limit_config(rate_limit);
    if let Some(sck_bytes) = &state.sck_bytes {
        builder = builder.sck(Sck::from_bytes(**sck_bytes));
    }
    if let Some(connector) = connector {
        builder = builder.extra_routes(connector);
    }

    let api = builder
        .build()
        .map_err(|error| format!("failed to build API router: {error}"))?;
    Ok(Runtime {
        router: api.into_router(),
        counts: RuntimeCounts {
            verifying_keys: config.verifying_keys.len(),
            connector_realms: config.connector_dispatch.len(),
        },
    })
}

fn build_verifying_key_registry(
    entries: &[config::VerifyingKeyConfig],
) -> Result<ApiVerifyingKeyRegistry, String> {
    let mut registry = ApiVerifyingKeyRegistry::new();
    for entry in entries {
        let key_bytes = read_fixed_key_file::<ED25519_KEY_BYTES>(
            &entry.public_key_path,
            "Ed25519 verifying key",
        )?;
        let vk = VerifyingKey::from_bytes(&key_bytes).map_err(|error| {
            format!(
                "failed to parse Ed25519 verifying key {}: {error}",
                entry.public_key_path.display()
            )
        })?;
        registry
            .insert(
                entry.kid.clone(),
                ApiVerifyingKeyEntry {
                    vk,
                    issuer: entry.issuer.clone(),
                    not_before: entry.not_before,
                    not_after: entry.not_after,
                },
            )
            .map_err(|error| {
                format!(
                    "failed to register API verifying key '{}': {error}",
                    entry.kid
                )
            })?;
    }
    Ok(registry)
}

fn build_config_lowerer(config: &ApiConfig) -> Result<Arc<dyn ConfigLowerer>, String> {
    let has_path = config.lowerer_signing_key_path.is_some();
    let has_kid = config.lowerer_signing_key_kid.is_some();
    let has_realm_keys = !config.realm_public_keys.is_empty();

    if !(has_path && has_kid && has_realm_keys) {
        eprintln!("philharmonic-api: lowerer not fully configured, using stub lowerer");
        return Ok(Arc::new(StubLowerer));
    }

    let path = config
        .lowerer_signing_key_path
        .as_deref()
        .ok_or_else(|| "lowerer_signing_key_path is required".to_string())?;
    let kid = config
        .lowerer_signing_key_kid
        .clone()
        .ok_or_else(|| "lowerer_signing_key_kid is required".to_string())?;
    let seed = read_fixed_secret_file::<ED25519_KEY_BYTES>(path, "lowerer Ed25519 signing seed")?;
    let signing_key = LowererSigningKey::from_seed(seed, kid);
    let realm_keys = build_realm_public_keys(&config.realm_public_keys)?;
    let issuer = config
        .lowerer_issuer
        .clone()
        .unwrap_or_else(|| config.issuer.clone());

    Ok(Arc::new(ConnectorConfigLowerer::new(
        signing_key,
        realm_keys,
        issuer,
        config.lowerer_token_lifetime_ms,
    )))
}

fn build_step_executor(config: &ApiConfig) -> Result<Arc<dyn StepExecutor>, String> {
    let Some(connector_url) = &config.connector_service_url else {
        eprintln!("philharmonic-api: connector service URL not configured, using stub executor");
        return Ok(Arc::new(StubExecutor));
    };

    Ok(Arc::new(
        HttpStepExecutor::new(connector_url.clone())
            .map_err(|error| format!("invalid connector_service_url: {error}"))?,
    ))
}

fn build_realm_public_keys(
    entries: &[config::RealmPublicKeyConfig],
) -> Result<HashMap<String, RealmPublicKey>, String> {
    let mut keys = HashMap::with_capacity(entries.len());
    for entry in entries {
        let key = read_realm_public_key(entry)?;
        if keys.insert(entry.realm_id.clone(), key).is_some() {
            return Err(format!(
                "duplicate realm public key configured for realm '{}'",
                entry.realm_id
            ));
        }
    }
    Ok(keys)
}

fn read_realm_public_key(entry: &config::RealmPublicKeyConfig) -> Result<RealmPublicKey, String> {
    let mlkem_public = read_fixed_key_file::<MLKEM_PUBLIC_KEY_BYTES>(
        &entry.mlkem_public_key_path,
        "ML-KEM-768 public key",
    )?;
    let x25519_public = read_fixed_key_file::<X25519_PUBLIC_KEY_BYTES>(
        &entry.x25519_public_key_path,
        "X25519 public key",
    )?;
    RealmPublicKey::new(
        entry.kid.clone(),
        RealmId::new(entry.realm_id.clone()),
        mlkem_public.to_vec(),
        x25519_public,
        entry.not_before,
        entry.not_after,
    )
    .map_err(|error| format!("failed to build realm public key '{}': {error}", entry.kid))
}

fn build_connector_routes(config: &ApiConfig) -> Result<Option<Router>, String> {
    if config.connector_dispatch.is_empty() {
        return Ok(None);
    }

    let mut dispatch = DispatchConfig::new(config.connector_domain_suffix.clone())
        .map_err(|error| format!("invalid connector dispatch config: {error}"))?;
    for (realm, upstream) in &config.connector_dispatch {
        let mut uris = Vec::with_capacity(upstream.upstreams.len());
        for uri in &upstream.upstreams {
            uris.push(uri.parse::<Uri>().map_err(|error| {
                format!("invalid connector upstream URI for realm '{realm}': {error}")
            })?);
        }
        dispatch
            .insert_realm(realm.clone(), uris)
            .map_err(|error| format!("invalid connector realm '{realm}': {error}"))?;
    }

    Ok(Some(Router::new().nest(
        "/connector",
        connector_router(RouterState::new(dispatch, Arc::new(HyperForwarder::new()))),
    )))
}

fn build_rate_limit_config(overrides: Option<&config::RateLimitOverrides>) -> RateLimitConfig {
    let mut config = RateLimitConfig::default();
    let Some(overrides) = overrides else {
        return config;
    };
    if let Some(override_config) = overrides.workflow {
        config.workflow = apply_rate_limit_override(config.workflow, override_config);
    }
    if let Some(override_config) = overrides.credential {
        config.credential = apply_rate_limit_override(config.credential, override_config);
    }
    if let Some(override_config) = overrides.minting {
        config.minting = apply_rate_limit_override(config.minting, override_config);
    }
    if let Some(override_config) = overrides.audit {
        config.audit = apply_rate_limit_override(config.audit, override_config);
    }
    if let Some(override_config) = overrides.admin {
        config.admin = apply_rate_limit_override(config.admin, override_config);
    }
    config
}

fn apply_rate_limit_override(
    current: RateLimitBucketConfig,
    override_config: config::RateLimitBucketOverride,
) -> RateLimitBucketConfig {
    RateLimitBucketConfig::new(
        override_config.capacity.unwrap_or(current.capacity),
        override_config
            .refill_per_second
            .unwrap_or(current.refill_per_second),
    )
}

fn load_signing_key(config: &ApiConfig) -> Result<ApiSigningKey, String> {
    let path = config
        .signing_key_path
        .as_deref()
        .ok_or_else(|| "signing_key_path is required".to_string())?;
    let kid = config
        .signing_key_kid
        .clone()
        .ok_or_else(|| "signing_key_kid is required".to_string())?;
    let seed = read_fixed_secret_file::<ED25519_KEY_BYTES>(path, "Ed25519 signing seed")?;
    Ok(ApiSigningKey::from_seed(seed, kid))
}

fn load_sck_bytes(config: &ApiConfig) -> Result<Option<Zeroizing<[u8; SCK_KEY_BYTES]>>, String> {
    config
        .sck_path
        .as_deref()
        .map(|path| read_fixed_secret_file::<SCK_KEY_BYTES>(path, "SCK"))
        .transpose()
}

fn dynamic_router(state: DynamicRouter) -> Router {
    Router::new()
        .fallback(any(dispatch_dynamic))
        .with_state(state)
}

async fn dispatch_dynamic(
    State(state): State<DynamicRouter>,
    request: Request<Body>,
) -> Response<Body> {
    let is_api_path =
        request.uri().path().starts_with("/v1/") || request.uri().path().starts_with("/connector/");

    if is_api_path {
        let router = state.router.read().await.clone();
        return match router.oneshot(request).await {
            Ok(response) => response,
            Err(error) => match error {},
        };
    }

    philharmonic::webui::webui_fallback(request).await
}

async fn start_server(app: Router, config: &ApiConfig) -> Result<&'static str, String> {
    #[cfg(feature = "https")]
    if let Some(tls) = &config.tls {
        start_tls_server(app, config.bind, tls).await?;
        return Ok("https");
    }

    let listener = TcpListener::bind(config.bind)
        .await
        .map_err(|error| format!("failed to bind API HTTP listener: {error}"))?;
    tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, app).await {
            eprintln!("philharmonic-api HTTP server stopped: {error}");
        }
    });
    Ok("http")
}

#[cfg(feature = "https")]
async fn start_tls_server(
    app: Router,
    bind: std::net::SocketAddr,
    tls: &config::TlsFileConfig,
) -> Result<(), String> {
    use hyper_util::rt::{TokioExecutor, TokioIo};
    use hyper_util::server::conn::auto::Builder;
    use hyper_util::service::TowerToHyperService;
    use tokio_rustls::TlsAcceptor;

    let tls_config = read_tls_server_config(tls)?;
    let acceptor = TlsAcceptor::from(Arc::new(tls_config));
    let listener = TcpListener::bind(bind)
        .await
        .map_err(|error| format!("failed to bind API HTTPS listener: {error}"))?;

    tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(accepted) => accepted,
                Err(error) => {
                    eprintln!("philharmonic-api HTTPS accept failed: {error}");
                    continue;
                }
            };
            let acceptor = acceptor.clone();
            let app = app.clone();
            tokio::spawn(async move {
                let tls_stream = match acceptor.accept(stream).await {
                    Ok(stream) => stream,
                    Err(error) => {
                        eprintln!("philharmonic-api TLS handshake failed: {error}");
                        return;
                    }
                };
                let service = TowerToHyperService::new(app);
                let builder = Builder::new(TokioExecutor::new());
                if let Err(error) = builder
                    .serve_connection_with_upgrades(TokioIo::new(tls_stream), service)
                    .await
                {
                    eprintln!("philharmonic-api HTTPS connection failed: {error}");
                }
            });
        }
    });

    Ok(())
}

#[cfg(feature = "https")]
fn read_tls_server_config(
    tls: &config::TlsFileConfig,
) -> Result<tokio_rustls::rustls::ServerConfig, String> {
    use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};

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

    let cert_chain: Vec<CertificateDer<'static>> = CertificateDer::pem_slice_iter(&cert_bytes)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("failed to parse TLS certificate chain: {error}"))?;
    if cert_chain.is_empty() {
        return Err("failed to parse TLS certificate chain: no certificates found".to_string());
    }

    let private_key = PrivateKeyDer::from_pem_slice(&key_bytes)
        .map_err(|error| format!("failed to parse TLS private key: {error}"))?;

    let mut config = tokio_rustls::rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, private_key)
        .map_err(|error| format!("failed to build TLS server config: {error}"))?;
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(config)
}

fn read_fixed_key_file<const N: usize>(path: &Path, label: &str) -> Result<[u8; N], String> {
    let bytes = read_key_file(path, N)?;
    <[u8; N]>::try_from(bytes.as_slice()).map_err(|_| {
        format!(
            "{label} file {} did not contain exactly {N} decoded bytes",
            path.display()
        )
    })
}

fn read_fixed_secret_file<const N: usize>(
    path: &Path,
    label: &str,
) -> Result<Zeroizing<[u8; N]>, String> {
    let bytes = read_key_file(path, N)?;
    let fixed = <[u8; N]>::try_from(bytes.as_slice()).map_err(|_| {
        format!(
            "{label} file {} did not contain exactly {N} decoded bytes",
            path.display()
        )
    })?;
    Ok(Zeroizing::new(fixed))
}

fn read_key_file(path: &Path, expected_len: usize) -> Result<Zeroizing<Vec<u8>>, String> {
    let bytes = Zeroizing::new(
        std::fs::read(path)
            .map_err(|error| format!("failed to read key file {}: {error}", path.display()))?,
    );
    if bytes.len() == expected_len {
        return Ok(bytes);
    }

    let hex_len = expected_len
        .checked_mul(2)
        .ok_or_else(|| "expected key length overflowed usize".to_string())?;
    let Ok(text) = std::str::from_utf8(bytes.as_slice()) else {
        return Err(format!(
            "key file {} has {} bytes; expected {expected_len} raw bytes",
            path.display(),
            bytes.len()
        ));
    };
    let compact = text
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    if compact.len() == hex_len
        && compact
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return hex::decode(&compact).map(Zeroizing::new).map_err(|error| {
            format!(
                "failed to decode hex key file {} as {expected_len} bytes: {error}",
                path.display()
            )
        });
    }

    Err(format!(
        "key file {} has {} raw bytes and {} hex characters; expected {expected_len} raw bytes or {hex_len} hex characters",
        path.display(),
        bytes.len(),
        compact.len()
    ))
}

fn log_loaded_counts(counts: RuntimeCounts) {
    eprintln!(
        "loaded {} API verifying key(s), {} connector realm(s)",
        counts.verifying_keys, counts.connector_realms
    );
}

fn log_reload(old: RuntimeCounts, new: RuntimeCounts) {
    eprintln!(
        "philharmonic-api reloaded config; API verifying keys {} -> {}, connector realms {} -> {}",
        old.verifying_keys, new.verifying_keys, old.connector_realms, new.connector_realms
    );
}

#[cfg(feature = "https")]
fn log_tls_reload_note(config: &ApiConfig) {
    if let Some(tls) = &config.tls {
        match read_tls_server_config(tls) {
            Ok(_) => eprintln!(
                "philharmonic-api re-read TLS certificate/key; restart required to apply TLS changes"
            ),
            Err(error) => eprintln!("philharmonic-api TLS reload check failed: {error}"),
        }
    }
}

#[cfg(not(feature = "https"))]
fn log_tls_reload_note(_config: &ApiConfig) {}
