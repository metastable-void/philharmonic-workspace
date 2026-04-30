use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;

use philharmonic::types::UnixMillis;

#[derive(Debug, serde::Deserialize)]
#[serde(default)]
pub struct ApiConfig {
    pub bind: SocketAddr,
    pub database_url: String,
    pub signing_key_path: Option<PathBuf>,
    pub signing_key_kid: Option<String>,
    pub issuer: String,
    pub lowerer_signing_key_path: Option<PathBuf>,
    pub lowerer_signing_key_kid: Option<String>,
    pub lowerer_issuer: Option<String>,
    #[serde(default = "default_lowerer_token_lifetime_ms")]
    pub lowerer_token_lifetime_ms: u64,
    pub realm_public_keys: Vec<RealmPublicKeyConfig>,
    pub connector_service_url: Option<String>,
    pub verifying_keys: Vec<VerifyingKeyConfig>,
    pub sck_path: Option<PathBuf>,
    pub sck_key_version: i64,
    pub connector_dispatch: HashMap<String, UpstreamConfig>,
    pub connector_domain_suffix: String,
    pub rate_limit: Option<RateLimitOverrides>,
    #[cfg(feature = "https")]
    pub tls: Option<TlsFileConfig>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            bind: SocketAddr::from(([127, 0, 0, 1], 3000)),
            database_url: "mysql://root@localhost/philharmonic".to_string(),
            signing_key_path: None,
            signing_key_kid: None,
            issuer: "philharmonic".to_string(),
            lowerer_signing_key_path: None,
            lowerer_signing_key_kid: None,
            lowerer_issuer: None,
            lowerer_token_lifetime_ms: default_lowerer_token_lifetime_ms(),
            realm_public_keys: Vec::new(),
            connector_service_url: None,
            verifying_keys: Vec::new(),
            sck_path: None,
            sck_key_version: 1,
            connector_dispatch: HashMap::new(),
            connector_domain_suffix: "localhost".to_string(),
            rate_limit: None,
            #[cfg(feature = "https")]
            tls: None,
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct RealmPublicKeyConfig {
    pub kid: String,
    pub realm_id: String,
    pub mlkem_public_key_path: PathBuf,
    pub x25519_public_key_path: PathBuf,
    #[serde(default = "default_not_before")]
    pub not_before: UnixMillis,
    #[serde(default = "default_not_after")]
    pub not_after: UnixMillis,
}

#[derive(Debug, serde::Deserialize)]
pub struct VerifyingKeyConfig {
    pub kid: String,
    pub public_key_path: PathBuf,
    pub issuer: String,
    #[serde(default = "default_not_before")]
    pub not_before: UnixMillis,
    #[serde(default = "default_not_after")]
    pub not_after: UnixMillis,
}

#[derive(Debug, serde::Deserialize)]
pub struct UpstreamConfig {
    pub upstreams: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct RateLimitOverrides {
    pub workflow: Option<RateLimitBucketOverride>,
    pub credential: Option<RateLimitBucketOverride>,
    pub minting: Option<RateLimitBucketOverride>,
    pub audit: Option<RateLimitBucketOverride>,
    pub admin: Option<RateLimitBucketOverride>,
}

#[derive(Clone, Copy, Debug, serde::Deserialize)]
pub struct RateLimitBucketOverride {
    pub capacity: Option<u32>,
    pub refill_per_second: Option<u32>,
}

#[cfg(feature = "https")]
#[derive(Debug, serde::Deserialize)]
pub struct TlsFileConfig {
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
}

const fn default_not_before() -> UnixMillis {
    UnixMillis(0)
}

const fn default_not_after() -> UnixMillis {
    UnixMillis(i64::MAX)
}

const fn default_lowerer_token_lifetime_ms() -> u64 {
    600_000
}
