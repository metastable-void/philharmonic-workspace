use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;

use philharmonic::types::UnixMillis;

#[derive(Debug, serde::Deserialize)]
#[serde(default)]
pub(crate) struct ApiConfig {
    pub(crate) bind: SocketAddr,
    pub(crate) database_url: String,
    pub(crate) signing_key_path: Option<PathBuf>,
    pub(crate) signing_key_kid: Option<String>,
    pub(crate) issuer: String,
    pub(crate) lowerer_signing_key_path: Option<PathBuf>,
    pub(crate) lowerer_signing_key_kid: Option<String>,
    pub(crate) lowerer_issuer: Option<String>,
    #[serde(default = "default_lowerer_token_lifetime_ms")]
    pub(crate) lowerer_token_lifetime_ms: u64,
    pub(crate) realm_public_keys: Vec<RealmPublicKeyConfig>,
    pub(crate) connector_service_url: Option<String>,
    pub(crate) verifying_keys: Vec<VerifyingKeyConfig>,
    pub(crate) sck_path: Option<PathBuf>,
    pub(crate) sck_key_version: i64,
    pub(crate) connector_dispatch: HashMap<String, UpstreamConfig>,
    pub(crate) connector_domain_suffix: String,
    pub(crate) rate_limit: Option<RateLimitOverrides>,
    #[serde(default = "default_brand_name")]
    pub(crate) webui_brand_name: String,
    #[cfg(feature = "https")]
    pub(crate) tls: Option<TlsFileConfig>,
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
            webui_brand_name: default_brand_name(),
            #[cfg(feature = "https")]
            tls: None,
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct RealmPublicKeyConfig {
    pub(crate) kid: String,
    pub(crate) realm_id: String,
    pub(crate) mlkem_public_key_path: PathBuf,
    pub(crate) x25519_public_key_path: PathBuf,
    #[serde(default = "default_not_before")]
    pub(crate) not_before: UnixMillis,
    #[serde(default = "default_not_after")]
    pub(crate) not_after: UnixMillis,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct VerifyingKeyConfig {
    pub(crate) kid: String,
    pub(crate) public_key_path: PathBuf,
    pub(crate) issuer: String,
    #[serde(default = "default_not_before")]
    pub(crate) not_before: UnixMillis,
    #[serde(default = "default_not_after")]
    pub(crate) not_after: UnixMillis,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct UpstreamConfig {
    pub(crate) upstreams: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct RateLimitOverrides {
    pub(crate) workflow: Option<RateLimitBucketOverride>,
    pub(crate) credential: Option<RateLimitBucketOverride>,
    pub(crate) minting: Option<RateLimitBucketOverride>,
    pub(crate) audit: Option<RateLimitBucketOverride>,
    pub(crate) admin: Option<RateLimitBucketOverride>,
}

#[derive(Clone, Copy, Debug, serde::Deserialize)]
pub(crate) struct RateLimitBucketOverride {
    pub(crate) capacity: Option<u32>,
    pub(crate) refill_per_second: Option<u32>,
}

#[cfg(feature = "https")]
#[derive(Debug, serde::Deserialize)]
pub(crate) struct TlsFileConfig {
    pub(crate) cert_path: PathBuf,
    pub(crate) key_path: PathBuf,
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

fn default_brand_name() -> String {
    "Philharmonic".to_string()
}
