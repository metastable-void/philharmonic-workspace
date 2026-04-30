use std::net::SocketAddr;
use std::path::PathBuf;

use philharmonic::connector_service::UnixMillis;

#[derive(Debug, serde::Deserialize)]
#[serde(default)]
pub struct ConnectorConfig {
    pub bind: SocketAddr,
    pub realm_id: String,
    pub minting_keys: Vec<MintingKeyConfig>,
    pub realm_keys: Vec<RealmKeyConfig>,
    #[cfg(feature = "https")]
    pub tls: Option<TlsFileConfig>,
}

impl Default for ConnectorConfig {
    fn default() -> Self {
        Self {
            bind: SocketAddr::from(([127, 0, 0, 1], 3002)),
            realm_id: "default".to_string(),
            minting_keys: Vec::new(),
            realm_keys: Vec::new(),
            #[cfg(feature = "https")]
            tls: None,
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct MintingKeyConfig {
    pub kid: String,
    pub public_key_path: PathBuf,
    pub not_before: UnixMillis,
    pub not_after: UnixMillis,
}

#[derive(Debug, serde::Deserialize)]
pub struct RealmKeyConfig {
    pub kid: String,
    pub realm_id: String,
    pub private_key_path: PathBuf,
    pub x25519_private_key_path: Option<PathBuf>,
    pub not_before: UnixMillis,
    pub not_after: UnixMillis,
}

#[cfg(feature = "https")]
#[derive(Debug, serde::Deserialize)]
pub struct TlsFileConfig {
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
}
