//! Philharmonic connector service — per-realm connector with payload decryption.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use clap::Parser;
use philharmonic::connector_common::RealmId;
use philharmonic::connector_impl_api::{Implementation, ImplementationError, JsonValue};
use philharmonic::connector_service::{
    MintingKeyEntry, MintingKeyRegistry, RealmPrivateKeyEntry, RealmPrivateKeyRegistry, UnixMillis,
    VerifyingKey, verify_and_decrypt,
};
use philharmonic::server::cli::{
    BaseArgs, BaseCommand, default_serve_command, resolve_config_paths,
};
use philharmonic::server::config::load_config_defaulting_missing;
#[cfg(feature = "https")]
use philharmonic::server::https::{start_tls_axum_server, validate_tls_server_files};
use philharmonic::server::install::{self, InstallPlan};
use philharmonic::server::key_material::{read_fixed_key_file, read_key_file};
use philharmonic::server::reload::ReloadHandle;
use serde::Serialize;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use x25519_dalek::StaticSecret;
use zeroize::Zeroizing;

mod config;
use config::ConnectorConfig;

const ENCRYPTED_PAYLOAD_HEADER: &str = "x-encrypted-payload";
const ED25519_VERIFYING_KEY_BYTES: usize = 32;
const MLKEM_SECRET_KEY_BYTES: usize = 2400;
const X25519_SECRET_KEY_BYTES: usize = 32;
const COMBINED_REALM_PRIVATE_KEY_BYTES: usize = MLKEM_SECRET_KEY_BYTES + X25519_SECRET_KEY_BYTES;
const DEFAULT_CONFIG: &str = r#"bind = "127.0.0.1:3002"
realm_id = "default"
"#;

type ImplementationRegistry = HashMap<String, Box<dyn Implementation>>;

#[derive(Parser)]
#[command(
    name = "philharmonic-connector",
    version,
    about = "Philharmonic per-realm connector service"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<BaseCommand>,
}

#[derive(Clone)]
struct AppState {
    runtime: Arc<RwLock<RuntimeState>>,
    implementations: Arc<ImplementationRegistry>,
}

struct RuntimeState {
    service_realm: String,
    minting_registry: MintingKeyRegistry,
    realm_registry: RealmPrivateKeyRegistry,
}

#[derive(Clone, Copy)]
struct RuntimeCounts {
    minting_keys: usize,
    realm_keys: usize,
}

struct Runtime {
    state: RuntimeState,
    counts: RuntimeCounts,
}

#[derive(serde::Deserialize)]
struct DecryptedPayload {
    realm: String,
    #[serde(rename = "impl")]
    implementation: String,
    config: JsonValue,
}

#[derive(Serialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Serialize)]
struct ErrorBody {
    kind: &'static str,
    message: String,
}

#[tokio::main]
async fn main() {
    if let Err(error) = run(Cli::parse()).await {
        eprintln!("philharmonic-connector: {error}");
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
            service_name: "philharmonic-connector".to_string(),
            binary_name: "philharmonic-connector".to_string(),
            description: "Philharmonic per-realm connector service".to_string(),
            config_file_name: "connector.toml".to_string(),
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
    let (primary, drop_in) = resolve_config_paths("connector", &args);
    let mut config = load_connector_config(&primary, &drop_in, &args)?;
    if let Some(bind) = args.bind {
        config.bind = bind;
    }
    if let Some(bind_h3) = args.bind_h3 {
        config.bind_h3 = Some(bind_h3);
    }

    let runtime = build_runtime(&config)?;
    let implementations = Arc::new(build_implementation_registry()?);
    let implementation_count = implementations.len();
    let app_state = AppState {
        runtime: Arc::new(RwLock::new(runtime.state)),
        implementations,
    };
    let app = router(app_state.clone());

    let bind = config.bind;
    let bind_h3 = config.bind_h3;
    let protocol = start_server(app, &config).await?;
    let mut counts = runtime.counts;
    match bind_h3 {
        Some(addr) => {
            eprintln!("philharmonic-connector listening on {bind} ({protocol}, h3 {addr})")
        }
        None => eprintln!("philharmonic-connector listening on {bind} ({protocol})"),
    }
    eprintln!("service realm: {}", config.realm_id);
    log_loaded_counts(counts, implementation_count);

    let reload_handle = ReloadHandle::new()
        .map_err(|error| format!("failed to install SIGHUP reload handler: {error}"))?;

    loop {
        reload_handle.notified().await;
        match load_connector_config(&primary, &drop_in, &args) {
            Ok(mut reloaded) => {
                if let Some(bind) = args.bind {
                    reloaded.bind = bind;
                }
                if let Some(bind_h3) = args.bind_h3 {
                    reloaded.bind_h3 = Some(bind_h3);
                }
                match build_runtime(&reloaded) {
                    Ok(runtime) => {
                        log_tls_reload_note(&reloaded);
                        *app_state.runtime.write().await = runtime.state;
                        log_reload(counts, runtime.counts);
                        counts = runtime.counts;
                    }
                    Err(error) => {
                        eprintln!("philharmonic-connector reload failed: {error}");
                    }
                }
            }
            Err(error) => {
                eprintln!("philharmonic-connector reload failed: {error}");
            }
        }
    }
}

fn load_connector_config(
    primary: &Path,
    drop_in: &Path,
    args: &BaseArgs,
) -> Result<ConnectorConfig, String> {
    let (config, defaulted) =
        load_config_defaulting_missing::<ConnectorConfig>(primary, drop_in, args.config.is_none())
            .map_err(|error| error.to_string())?;
    if defaulted {
        eprintln!(
            "philharmonic-connector config {} not found; using built-in defaults",
            primary.display()
        );
    }
    Ok(config)
}

fn build_runtime(config: &ConnectorConfig) -> Result<Runtime, String> {
    let minting_registry = build_minting_key_registry(&config.minting_keys)?;
    let realm_registry = build_realm_private_key_registry(&config.realm_keys)?;
    let counts = RuntimeCounts {
        minting_keys: count_unique_minting_keys(&minting_registry, &config.minting_keys),
        realm_keys: count_unique_realm_keys(&realm_registry, &config.realm_keys),
    };

    Ok(Runtime {
        state: RuntimeState {
            service_realm: config.realm_id.clone(),
            minting_registry,
            realm_registry,
        },
        counts,
    })
}

fn build_minting_key_registry(
    entries: &[config::MintingKeyConfig],
) -> Result<MintingKeyRegistry, String> {
    let mut registry = MintingKeyRegistry::new();
    for entry in entries {
        let key_bytes = read_fixed_key_file::<ED25519_VERIFYING_KEY_BYTES>(
            &entry.public_key_path,
            "Ed25519 verifying key",
        )?;
        let vk = VerifyingKey::from_bytes(&key_bytes).map_err(|error| {
            format!(
                "failed to parse Ed25519 verifying key {}: {error}",
                entry.public_key_path.display()
            )
        })?;
        registry.insert(
            entry.kid.clone(),
            MintingKeyEntry {
                vk,
                not_before: entry.not_before,
                not_after: entry.not_after,
            },
        );
    }
    Ok(registry)
}

fn build_realm_private_key_registry(
    entries: &[config::RealmKeyConfig],
) -> Result<RealmPrivateKeyRegistry, String> {
    let mut registry = RealmPrivateKeyRegistry::new();
    for entry in entries {
        let (kem_sk, x25519_sk) = read_realm_private_key(entry)?;
        registry.insert(
            entry.kid.clone(),
            RealmPrivateKeyEntry {
                kem_sk: Zeroizing::new(kem_sk),
                ecdh_sk: Zeroizing::new(StaticSecret::from(x25519_sk)),
                realm: RealmId::new(entry.realm_id.clone()),
                not_before: entry.not_before,
                not_after: entry.not_after,
            },
        );
    }
    Ok(registry)
}

fn read_realm_private_key(
    entry: &config::RealmKeyConfig,
) -> Result<([u8; MLKEM_SECRET_KEY_BYTES], [u8; X25519_SECRET_KEY_BYTES]), String> {
    if let Some(x25519_private_key_path) = &entry.x25519_private_key_path {
        let kem_sk = read_fixed_key_file::<MLKEM_SECRET_KEY_BYTES>(
            &entry.private_key_path,
            "ML-KEM-768 secret key",
        )?;
        let x25519_sk = read_fixed_key_file::<X25519_SECRET_KEY_BYTES>(
            x25519_private_key_path,
            "X25519 static secret key",
        )?;
        return Ok((kem_sk, x25519_sk));
    }

    let combined = read_key_file(&entry.private_key_path, COMBINED_REALM_PRIVATE_KEY_BYTES)?;
    let (kem_slice, x25519_slice) = combined.as_slice().split_at(MLKEM_SECRET_KEY_BYTES);
    let kem_sk = <[u8; MLKEM_SECRET_KEY_BYTES]>::try_from(kem_slice).map_err(|_| {
        format!(
            "failed to split combined realm private key {}",
            entry.private_key_path.display()
        )
    })?;
    let x25519_sk = <[u8; X25519_SECRET_KEY_BYTES]>::try_from(x25519_slice).map_err(|_| {
        format!(
            "failed to split combined realm private key {}",
            entry.private_key_path.display()
        )
    })?;
    Ok((kem_sk, x25519_sk))
}

fn build_implementation_registry() -> Result<ImplementationRegistry, String> {
    let mut registry = ImplementationRegistry::new();

    register_implementation(
        &mut registry,
        philharmonic::connector_impl_http_forward::HttpForward::new()
            .map_err(|error| format!("failed to build http_forward implementation: {error}"))?,
    )?;

    register_implementation(
        &mut registry,
        philharmonic::connector_impl_llm_openai_compat::LlmOpenaiCompat::new().map_err(
            |error| format!("failed to build llm_openai_compat implementation: {error}"),
        )?,
    )?;

    register_implementation(
        &mut registry,
        philharmonic::connector_impl_sql_postgres::SqlPostgres::new(),
    )?;

    register_implementation(
        &mut registry,
        philharmonic::connector_impl_sql_mysql::SqlMysql::new(),
    )?;

    register_implementation(
        &mut registry,
        philharmonic::connector_impl_embed::Embed::new_default()
            .map_err(|error| format!("failed to build embed implementation: {error}"))?,
    )?;

    register_implementation(
        &mut registry,
        philharmonic::connector_impl_vector_search::VectorSearch::new(),
    )?;

    Ok(registry)
}

fn register_implementation<I>(
    registry: &mut ImplementationRegistry,
    implementation: I,
) -> Result<(), String>
where
    I: Implementation + 'static,
{
    let name = implementation.name().to_string();
    if registry
        .insert(name.clone(), Box::new(implementation))
        .is_some()
    {
        return Err(format!("duplicate connector implementation name '{name}'"));
    }
    Ok(())
}

/// Maximum request-body size accepted by the connector binary.
///
/// Raised from axum's 2 MiB default so that workflows passing large
/// request bodies (notably `vector_search` corpora — a 1024-dim f32
/// CorpusItem JSON-encodes to ~10-12 KiB, putting the practical limit
/// around 170 items at the 2 MiB default) can land. The connector
/// service still enforces logical caps inside each implementation;
/// this is just the HTTP envelope ceiling.
const MAX_REQUEST_BODY_BYTES: usize = 32 * 1024 * 1024;

fn router(state: AppState) -> Router {
    Router::new()
        .route("/", post(handle_connector_request))
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BODY_BYTES))
        .with_state(state)
}

async fn handle_connector_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    match handle_connector_request_inner(state, headers, body).await {
        Ok(response) => Json(response).into_response(),
        Err(error) => error.into_response(),
    }
}

async fn handle_connector_request_inner(
    state: AppState,
    headers: HeaderMap,
    body: Bytes,
) -> Result<JsonValue, ServiceError> {
    let token_cose_bytes = bearer_token_bytes(&headers)?;
    let encrypted_payload_bytes = encrypted_payload_bytes(&headers)?;
    let request = serde_json::from_slice::<JsonValue>(&body).map_err(|error| {
        ServiceError::bad_request(format!("request body must be valid JSON: {error}"))
    })?;

    let runtime = state.runtime.read().await;
    let service_realm = runtime.service_realm.clone();
    let verified = verify_and_decrypt(
        &token_cose_bytes,
        &encrypted_payload_bytes,
        &service_realm,
        &runtime.minting_registry,
        &runtime.realm_registry,
        UnixMillis::now(),
    )
    .map_err(|error| ServiceError::unauthorized(format!("token verification failed: {error}")))?;
    drop(runtime);

    let payload =
        serde_json::from_slice::<DecryptedPayload>(&verified.plaintext).map_err(|error| {
            ServiceError::bad_request(format!("decrypted connector payload is invalid: {error}"))
        })?;
    if payload.realm != service_realm {
        return Err(ServiceError::unauthorized(format!(
            "decrypted connector payload realm '{}' does not match service realm '{service_realm}'",
            payload.realm
        )));
    }
    let implementation = state
        .implementations
        .get(&payload.implementation)
        .ok_or_else(|| {
            ServiceError::not_found(format!(
                "unknown connector implementation '{}'",
                payload.implementation
            ))
        })?;

    implementation
        .execute(&payload.config, &request, &verified.context)
        .await
        .map_err(ServiceError::implementation)
}

fn bearer_token_bytes(headers: &HeaderMap) -> Result<Vec<u8>, ServiceError> {
    let value = headers
        .get(header::AUTHORIZATION)
        .ok_or_else(|| ServiceError::unauthorized("missing Authorization bearer token"))?
        .to_str()
        .map_err(|_| ServiceError::unauthorized("Authorization header is not valid ASCII"))?;
    let Some(token) = value
        .strip_prefix("Bearer ")
        .or_else(|| value.strip_prefix("bearer "))
    else {
        return Err(ServiceError::unauthorized(
            "Authorization header must use Bearer scheme",
        ));
    };
    decode_hex_header(token, "Authorization bearer token").map_err(ServiceError::unauthorized)
}

fn encrypted_payload_bytes(headers: &HeaderMap) -> Result<Vec<u8>, ServiceError> {
    let value = headers
        .get(ENCRYPTED_PAYLOAD_HEADER)
        .ok_or_else(|| ServiceError::bad_request("missing X-Encrypted-Payload header"))?
        .to_str()
        .map_err(|_| ServiceError::bad_request("X-Encrypted-Payload header is not valid ASCII"))?;
    decode_hex_header(value, "X-Encrypted-Payload header").map_err(ServiceError::bad_request)
}

fn decode_hex_header(value: &str, label: &str) -> Result<Vec<u8>, String> {
    let compact = value
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    if compact.is_empty() {
        return Err(format!("{label} is empty"));
    }
    hex::decode(&compact)
        .map_err(|error| format!("{label} must be hex-encoded COSE bytes: {error}"))
}

async fn start_server(app: Router, config: &ConnectorConfig) -> Result<&'static str, String> {
    #[cfg(feature = "https")]
    if let Some(tls) = &config.tls {
        start_tls_axum_server(
            app,
            config.bind,
            config.bind_h3,
            &tls.cert_path,
            &tls.key_path,
            "philharmonic-connector",
            "connector",
        )
        .await?;
        return Ok(if config.bind_h3.is_some() {
            "https+h3"
        } else {
            "https"
        });
    }

    if config.bind_h3.is_some() {
        return Err("HTTP/3 requires TLS; configure `[tls]`".to_string());
    }

    let listener = TcpListener::bind(config.bind)
        .await
        .map_err(|error| format!("failed to bind connector HTTP listener: {error}"))?;
    tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, app).await {
            eprintln!("philharmonic-connector HTTP server stopped: {error}");
        }
    });
    Ok("http")
}

fn count_unique_minting_keys(
    registry: &MintingKeyRegistry,
    entries: &[config::MintingKeyConfig],
) -> usize {
    entries
        .iter()
        .map(|entry| entry.kid.as_str())
        .filter(|kid| registry.lookup(kid).is_some())
        .collect::<HashSet<_>>()
        .len()
}

fn count_unique_realm_keys(
    registry: &RealmPrivateKeyRegistry,
    entries: &[config::RealmKeyConfig],
) -> usize {
    entries
        .iter()
        .map(|entry| entry.kid.as_str())
        .filter(|kid| registry.lookup(kid).is_some())
        .collect::<HashSet<_>>()
        .len()
}

fn log_loaded_counts(counts: RuntimeCounts, implementation_count: usize) {
    eprintln!(
        "loaded {} minting key(s), {} realm key(s), {} implementation(s)",
        counts.minting_keys, counts.realm_keys, implementation_count
    );
}

fn log_reload(old: RuntimeCounts, new: RuntimeCounts) {
    eprintln!(
        "philharmonic-connector reloaded config; minting keys {} -> {}, realm keys {} -> {}",
        old.minting_keys, new.minting_keys, old.realm_keys, new.realm_keys
    );
}

#[cfg(feature = "https")]
fn log_tls_reload_note(config: &ConnectorConfig) {
    if let Some(tls) = &config.tls {
        match validate_tls_server_files(&tls.cert_path, &tls.key_path) {
            Ok(_) => eprintln!(
                "philharmonic-connector re-read TLS certificate/key; restart required to apply TLS changes"
            ),
            Err(error) => eprintln!("philharmonic-connector TLS reload check failed: {error}"),
        }
    }
}

#[cfg(not(feature = "https"))]
fn log_tls_reload_note(_config: &ConnectorConfig) {}

struct ServiceError {
    status: StatusCode,
    kind: &'static str,
    message: String,
}

impl ServiceError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            kind: "bad_request",
            message: message.into(),
        }
    }

    fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            kind: "unauthorized",
            message: message.into(),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            kind: "not_found",
            message: message.into(),
        }
    }

    fn implementation(error: ImplementationError) -> Self {
        let status = implementation_status(&error);
        Self {
            status,
            kind: implementation_error_kind(&error),
            message: error.to_string(),
        }
    }
}

impl IntoResponse for ServiceError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorEnvelope {
                error: ErrorBody {
                    kind: self.kind,
                    message: self.message,
                },
            }),
        )
            .into_response()
    }
}

fn implementation_status(error: &ImplementationError) -> StatusCode {
    match error {
        ImplementationError::InvalidConfig { .. } | ImplementationError::InvalidRequest { .. } => {
            StatusCode::BAD_REQUEST
        }
        ImplementationError::SchemaValidationFailed { .. } => StatusCode::UNPROCESSABLE_ENTITY,
        ImplementationError::UpstreamError { .. }
        | ImplementationError::UpstreamUnreachable { .. }
        | ImplementationError::ResponseTooLarge { .. } => StatusCode::BAD_GATEWAY,
        ImplementationError::UpstreamTimeout => StatusCode::GATEWAY_TIMEOUT,
        ImplementationError::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn implementation_error_kind(error: &ImplementationError) -> &'static str {
    match error {
        ImplementationError::InvalidConfig { .. } => "invalid_config",
        ImplementationError::UpstreamError { .. } => "upstream_error",
        ImplementationError::UpstreamUnreachable { .. } => "upstream_unreachable",
        ImplementationError::UpstreamTimeout => "upstream_timeout",
        ImplementationError::SchemaValidationFailed { .. } => "schema_validation_failed",
        ImplementationError::ResponseTooLarge { .. } => "response_too_large",
        ImplementationError::InvalidRequest { .. } => "invalid_request",
        ImplementationError::Internal { .. } => "internal",
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use axum::Router;

    use super::{ConnectorConfig, start_server};

    #[tokio::test]
    async fn bind_h3_without_tls_errors_before_binding_http() {
        let mut config = ConnectorConfig {
            bind: SocketAddr::from(([127, 0, 0, 1], 0)),
            ..ConnectorConfig::default()
        };
        config.bind_h3 = Some(SocketAddr::from(([127, 0, 0, 1], 0)));

        let error = start_server(Router::new(), &config)
            .await
            .expect_err("HTTP/3 without TLS must fail");

        assert_eq!(error, "HTTP/3 requires TLS; configure `[tls]`");
    }
}
