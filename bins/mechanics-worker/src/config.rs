use std::net::SocketAddr;

#[derive(Debug, serde::Deserialize)]
#[serde(default)]
pub struct MechanicsWorkerConfig {
    pub bind: SocketAddr,
    pub tokens: Vec<String>,
    pub pool: PoolConfig,
    #[cfg(feature = "https")]
    pub tls: Option<TlsFileConfig>,
}

impl Default for MechanicsWorkerConfig {
    fn default() -> Self {
        Self {
            bind: SocketAddr::from(([127, 0, 0, 1], 3001)),
            tokens: Vec::new(),
            pool: PoolConfig::default(),
            #[cfg(feature = "https")]
            tls: None,
        }
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(default)]
pub struct PoolConfig {
    pub execution_timeout_secs: u64,
    pub run_timeout_secs: u64,
    pub default_http_timeout_ms: u64,
    pub max_memory: usize,
    pub max_stack: usize,
    pub max_output: usize,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            execution_timeout_secs: 3600,
            run_timeout_secs: 3600,
            default_http_timeout_ms: 300_000,
            max_memory: 65536,
            max_stack: 65536,
            max_output: 131072,
        }
    }
}

#[cfg(feature = "https")]
#[derive(Debug, serde::Deserialize)]
pub struct TlsFileConfig {
    pub cert_path: std::path::PathBuf,
    pub key_path: std::path::PathBuf,
}
