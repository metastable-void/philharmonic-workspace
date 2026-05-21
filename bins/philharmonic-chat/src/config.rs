use std::{net::SocketAddr, path::PathBuf};

use serde::Deserialize;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Config {
    pub(crate) chat: ChatConfig,
    pub(crate) tls: Option<TlsConfig>,
}

impl Config {
    pub(crate) fn load(path: &PathBuf) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        toml::from_str(&text).map_err(|error| format!("cannot parse {}: {error}", path.display()))
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        self.chat.validate()
    }
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ChatConfig {
    pub(crate) bind: SocketAddr,
    pub(crate) bind_h3: Option<SocketAddr>,
    pub(crate) api_url: String,
    pub(crate) service_token: String,
    pub(crate) agent_token: String,
    pub(crate) minting_token: String,
    pub(crate) chat_uuid: Uuid,
    pub(crate) notify_instance_uuid: Uuid,
}

impl ChatConfig {
    fn validate(&self) -> Result<(), String> {
        let api_url = self.api_url.trim();
        if !(api_url.starts_with("https://") || api_url.starts_with("http://")) {
            return Err("chat.api_url must start with http:// or https://".to_string());
        }
        for (name, value) in [
            ("chat.service_token", self.service_token.as_str()),
            ("chat.agent_token", self.agent_token.as_str()),
            ("chat.minting_token", self.minting_token.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(format!("{name} must not be empty"));
            }
        }
        Ok(())
    }

    pub(crate) fn api_endpoint(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.api_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct TlsConfig {
    pub(crate) cert_path: PathBuf,
    pub(crate) key_path: PathBuf,
}
