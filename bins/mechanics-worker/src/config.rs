use std::net::SocketAddr;

#[derive(Debug, serde::Deserialize)]
#[serde(default)]
pub(crate) struct MechanicsWorkerConfig {
    pub(crate) bind: SocketAddr,
    pub(crate) tokens: Vec<String>,
    pub(crate) pool: PoolConfig,
    #[cfg(feature = "https")]
    pub(crate) tls: Option<TlsFileConfig>,
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
pub(crate) struct PoolConfig {
    pub(crate) execution_timeout_secs: u64,
    pub(crate) run_timeout_secs: u64,
    pub(crate) default_http_timeout_ms: u64,
    pub(crate) max_memory: usize,
    pub(crate) max_stack: usize,
    pub(crate) max_output: usize,
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
pub(crate) struct TlsFileConfig {
    pub(crate) cert_path: std::path::PathBuf,
    pub(crate) key_path: std::path::PathBuf,
}
