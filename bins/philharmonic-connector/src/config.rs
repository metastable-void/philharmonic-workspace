use std::net::SocketAddr;
use std::path::PathBuf;

use philharmonic::connector_service::UnixMillis;

#[derive(Debug, serde::Deserialize)]
#[serde(default)]
pub(crate) struct ConnectorConfig {
    pub(crate) bind: SocketAddr,
    pub(crate) realm_id: String,
    pub(crate) minting_keys: Vec<MintingKeyConfig>,
    pub(crate) realm_keys: Vec<RealmKeyConfig>,
    #[cfg(feature = "https")]
    pub(crate) tls: Option<TlsFileConfig>,
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
pub(crate) struct MintingKeyConfig {
    pub(crate) kid: String,
    pub(crate) public_key_path: PathBuf,
    pub(crate) not_before: UnixMillis,
    pub(crate) not_after: UnixMillis,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct RealmKeyConfig {
    pub(crate) kid: String,
    pub(crate) realm_id: String,
    pub(crate) private_key_path: PathBuf,
    pub(crate) x25519_private_key_path: Option<PathBuf>,
    pub(crate) not_before: UnixMillis,
    pub(crate) not_after: UnixMillis,
}

#[cfg(feature = "https")]
#[derive(Debug, serde::Deserialize)]
pub(crate) struct TlsFileConfig {
    pub(crate) cert_path: PathBuf,
    pub(crate) key_path: PathBuf,
}
