//! DNS resolution helpers shared by Mechanics runtime crates.
//!
//! [`Resolver`] loads the system resolver configuration when available.
//! If that load fails because `/etc/resolv.conf` is missing, it falls
//! back to the Cloudflare resolver set documented by Philharmonic's DNS
//! connector design. Other system-configuration errors remain visible to
//! the caller during construction.

#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::time::Instant;

use hickory_resolver::config::{
    NameServerConfig, ResolverConfig, ResolverOpts, ServerOrderingStrategy,
};
use hickory_resolver::net::proto::rr::rdata::svcb::SvcParamValue;
use hickory_resolver::net::proto::rr::{RData, Record};
use hickory_resolver::net::runtime::TokioRuntimeProvider;
use hickory_resolver::net::{DnsError, NetError};
use hickory_resolver::{Resolver as HickoryResolver, TokioResolver};
use thiserror::Error;

pub use hickory_resolver::net::proto::op::ResponseCode;
pub use hickory_resolver::net::proto::rr::RecordType;

/// Cloudflare recursive resolver set used when `/etc/resolv.conf` is missing.
///
/// The order matches `docs/design/08-connector-architecture.md`.
pub const CLOUDFLARE_FALLBACK_RESOLVERS: [IpAddr; 4] = [
    IpAddr::V6(Ipv6Addr::new(0x2606, 0x4700, 0x4700, 0, 0, 0, 0, 0x1111)),
    IpAddr::V6(Ipv6Addr::new(0x2606, 0x4700, 0x4700, 0, 0, 0, 0, 0x1001)),
    IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)),
    IpAddr::V4(Ipv4Addr::new(1, 0, 0, 1)),
];

/// Crate result type.
pub type Result<T> = std::result::Result<T, Error>;

/// DNS resolver errors.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// The host resolver configuration could not be loaded.
    #[error("failed to load DNS system configuration: {0}")]
    SystemConfig(String),
    /// A resolver could not be constructed from an already-selected config.
    #[error("failed to initialise DNS resolver: {0}")]
    ResolverInit(String),
    /// A record type string could not be parsed.
    #[error("unsupported DNS record type `{0}`")]
    InvalidRecordType(String),
    /// A DNS lookup failed.
    #[error("DNS {record_type} lookup for `{name}` failed: {message}")]
    Lookup {
        /// Queried host name.
        name: String,
        /// DNS record type being queried.
        record_type: String,
        /// DNS response code when the resolver surfaced one.
        response_code: Option<ResponseCode>,
        /// Underlying resolver error.
        message: String,
    },
}

/// Reusable DNS resolver.
#[derive(Clone)]
pub struct Resolver {
    inner: Arc<TokioResolver>,
}

impl std::fmt::Debug for Resolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Resolver").finish_non_exhaustive()
    }
}

impl Resolver {
    /// Build a resolver from the host configuration, falling back to
    /// [`CLOUDFLARE_FALLBACK_RESOLVERS`] only when the configuration file
    /// is missing.
    pub fn new() -> Result<Self> {
        match TokioResolver::builder_tokio() {
            Ok(mut builder) => {
                normalise_options(builder.options_mut());
                build_resolver(builder)
            }
            Err(error) if missing_system_resolver_config(&error) => Self::cloudflare_fallback(),
            Err(error) => Err(Error::SystemConfig(error.to_string())),
        }
    }

    /// Build a resolver using the Cloudflare fallback resolver set.
    pub fn cloudflare_fallback() -> Result<Self> {
        let mut builder = HickoryResolver::builder_with_config(
            cloudflare_resolver_config(),
            TokioRuntimeProvider::default(),
        );
        normalise_options(builder.options_mut());
        build_resolver(builder)
    }

    /// Resolve A records for `host`.
    pub async fn lookup_a(&self, host: &str) -> Result<Vec<Ipv4Addr>> {
        if let Ok(address) = host.parse::<Ipv4Addr>() {
            return Ok(vec![address]);
        }

        let lookup = match self.inner.lookup(host, RecordType::A).await {
            Ok(lookup) => lookup,
            Err(error) if error.is_no_records_found() => return Ok(Vec::new()),
            Err(error) => return Err(lookup_error(host, "A", error)),
        };
        Ok(lookup
            .answers()
            .iter()
            .filter_map(|record| match &record.data {
                RData::A(address) => Some(address.0),
                _ => None,
            })
            .collect())
    }

    /// Resolve AAAA records for `host`.
    pub async fn lookup_aaaa(&self, host: &str) -> Result<Vec<Ipv6Addr>> {
        if let Ok(address) = host.parse::<Ipv6Addr>() {
            return Ok(vec![address]);
        }

        let lookup = match self.inner.lookup(host, RecordType::AAAA).await {
            Ok(lookup) => lookup,
            Err(error) if error.is_no_records_found() => return Ok(Vec::new()),
            Err(error) => return Err(lookup_error(host, "AAAA", error)),
        };
        Ok(lookup
            .answers()
            .iter()
            .filter_map(|record| match &record.data {
                RData::AAAA(address) => Some(address.0),
                _ => None,
            })
            .collect())
    }

    /// Resolve IP addresses for `host` using the resolver's configured
    /// dual-stack strategy.
    pub async fn lookup_ip(&self, host: &str) -> Result<Vec<IpAddr>> {
        if let Ok(address) = host.parse::<IpAddr>() {
            return Ok(vec![address]);
        }

        let lookup = match self.inner.lookup_ip(host).await {
            Ok(lookup) => lookup,
            Err(error) if error.is_no_records_found() => return Ok(Vec::new()),
            Err(error) => return Err(lookup_error(host, "A/AAAA", error)),
        };
        Ok(lookup.iter().collect())
    }

    /// Resolve `host` and attach `port` to every returned IP address.
    pub async fn lookup_socket_addrs(&self, host: &str, port: u16) -> Result<Vec<SocketAddr>> {
        Ok(self
            .lookup_ip(host)
            .await?
            .into_iter()
            .map(|address| SocketAddr::new(address, port))
            .collect())
    }

    /// Run a generic IN-class DNS query and return presentation-format records.
    pub async fn query(&self, name: &str, record_type: RecordType) -> Result<Vec<DnsRecord>> {
        let lookup = match self.inner.lookup(name, record_type).await {
            Ok(lookup) => lookup,
            Err(error) if no_data_response(&error) => return Ok(Vec::new()),
            Err(error) => return Err(lookup_error(name, record_type, error)),
        };
        Ok(lookup
            .answers()
            .iter()
            .map(DnsRecord::from_hickory)
            .collect())
    }

    /// Resolve HTTPS resource records for `host`.
    pub async fn lookup_https(&self, host: &str) -> Result<Vec<HttpsRecord>> {
        let lookup = match self.inner.lookup(host, RecordType::HTTPS).await {
            Ok(lookup) => lookup,
            Err(error) if error.is_no_records_found() => return Ok(Vec::new()),
            Err(error) => return Err(lookup_error(host, "HTTPS", error)),
        };
        let expires_at = lookup.valid_until();
        Ok(lookup
            .answers()
            .iter()
            .filter_map(|record| match &record.data {
                RData::HTTPS(https) => Some(https_record_from_svcb(&https.0, expires_at)),
                _ => None,
            })
            .collect())
    }
}

/// Parse a DNS record type string using canonical IANA names.
///
/// Input is matched case-insensitively after trimming ASCII whitespace.
pub fn parse_record_type(record_type: &str) -> Result<RecordType> {
    let canonical = record_type.trim().to_ascii_uppercase();
    canonical
        .parse()
        .map_err(|_| Error::InvalidRecordType(record_type.to_owned()))
}

/// Generic DNS record in presentation form.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DnsRecord {
    /// Owner name rendered in DNS presentation form.
    pub name: String,
    /// DNS record type.
    pub record_type: RecordType,
    /// Time-to-live in seconds.
    pub ttl: u32,
    /// RDATA rendered in presentation form.
    pub data: String,
}

impl DnsRecord {
    fn from_hickory(record: &Record) -> Self {
        Self {
            name: record.name.to_string(),
            record_type: record.record_type(),
            ttl: record.ttl,
            data: record.data.to_string(),
        }
    }

    /// DNS record type rendered as its canonical IANA name.
    pub fn record_type_name(&self) -> String {
        self.record_type.to_string()
    }
}

/// Parsed HTTPS resource record data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpsRecord {
    /// SVCB priority.
    pub priority: u16,
    /// SVCB target name rendered in presentation form.
    pub target_name: String,
    /// ALPN protocol identifiers advertised by the record.
    pub alpns: Vec<String>,
    /// Optional port override.
    pub port: Option<u16>,
    /// IPv4 address hints.
    pub ipv4_hints: Vec<Ipv4Addr>,
    /// IPv6 address hints.
    pub ipv6_hints: Vec<Ipv6Addr>,
    /// Expiry derived from the RRSet TTL.
    pub expires_at: Instant,
}

impl HttpsRecord {
    /// Return true when the record advertises `alpn`.
    pub fn has_alpn(&self, alpn: &str) -> bool {
        self.alpns.iter().any(|candidate| candidate == alpn)
    }

    /// Iterate all IPv4 and IPv6 address hints as [`IpAddr`] values.
    pub fn address_hints(&self) -> impl Iterator<Item = IpAddr> + '_ {
        self.ipv4_hints
            .iter()
            .copied()
            .map(IpAddr::V4)
            .chain(self.ipv6_hints.iter().copied().map(IpAddr::V6))
    }
}

fn build_resolver(
    builder: hickory_resolver::ResolverBuilder<TokioRuntimeProvider>,
) -> Result<Resolver> {
    builder
        .build()
        .map(|resolver| Resolver {
            inner: Arc::new(resolver),
        })
        .map_err(|error| Error::ResolverInit(error.to_string()))
}

fn cloudflare_resolver_config() -> ResolverConfig {
    ResolverConfig::from_parts(
        None,
        Vec::new(),
        CLOUDFLARE_FALLBACK_RESOLVERS
            .iter()
            .copied()
            .map(NameServerConfig::udp_and_tcp)
            .collect(),
    )
}

fn normalise_options(options: &mut ResolverOpts) {
    options.cache_size = 0;
    options.server_ordering_strategy = ServerOrderingStrategy::UserProvidedOrder;
}

fn missing_system_resolver_config(error: &NetError) -> bool {
    matches!(error, NetError::Io(io) if io.kind() == io::ErrorKind::NotFound)
}

fn lookup_error(name: &str, record_type: impl std::fmt::Display, error: NetError) -> Error {
    let response_code = response_code(&error);
    Error::Lookup {
        name: name.to_owned(),
        record_type: record_type.to_string(),
        response_code,
        message: error.to_string(),
    }
}

fn response_code(error: &NetError) -> Option<ResponseCode> {
    match error {
        NetError::Dns(DnsError::NoRecordsFound(no_records)) => Some(no_records.response_code),
        NetError::Dns(DnsError::ResponseCode(response_code)) => Some(*response_code),
        _ => None,
    }
}

fn no_data_response(error: &NetError) -> bool {
    matches!(
        error,
        NetError::Dns(DnsError::NoRecordsFound(no_records))
            if no_records.response_code == ResponseCode::NoError
    )
}

fn https_record_from_svcb(
    svcb: &hickory_resolver::net::proto::rr::rdata::SVCB,
    expires_at: Instant,
) -> HttpsRecord {
    let mut alpns = Vec::new();
    let mut port = None;
    let mut ipv4_hints = Vec::new();
    let mut ipv6_hints = Vec::new();

    for (_key, value) in &svcb.svc_params {
        match value {
            SvcParamValue::Alpn(value) => {
                alpns.extend(value.0.iter().cloned());
            }
            SvcParamValue::Port(value) => {
                port = Some(*value);
            }
            SvcParamValue::Ipv4Hint(value) => {
                ipv4_hints.extend(value.0.iter().map(|address| address.0));
            }
            SvcParamValue::Ipv6Hint(value) => {
                ipv6_hints.extend(value.0.iter().map(|address| address.0));
            }
            _ => {}
        }
    }

    HttpsRecord {
        priority: svcb.svc_priority,
        target_name: svcb.target_name.to_string(),
        alpns,
        port,
        ipv4_hints,
        ipv6_hints,
        expires_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hickory_resolver::net::NoRecords;
    use hickory_resolver::net::proto::op::Query;
    use hickory_resolver::net::proto::rr::Name;
    use hickory_resolver::net::proto::rr::Record;
    use hickory_resolver::net::proto::rr::rdata::svcb::{
        Alpn, IpHint, SVCB, SvcParamKey, SvcParamValue,
    };
    use hickory_resolver::net::proto::rr::rdata::{A, AAAA};

    #[test]
    fn cloudflare_order_matches_design_doc() {
        let resolvers = CLOUDFLARE_FALLBACK_RESOLVERS.map(|address| address.to_string());
        assert_eq!(
            resolvers,
            [
                "2606:4700:4700::1111",
                "2606:4700:4700::1001",
                "1.1.1.1",
                "1.0.0.1"
            ]
        );
    }

    #[test]
    fn only_not_found_errors_trigger_system_config_fallback() {
        let missing: NetError = io::Error::from(io::ErrorKind::NotFound).into();
        assert!(missing_system_resolver_config(&missing));

        let denied: NetError = io::Error::from(io::ErrorKind::PermissionDenied).into();
        assert!(!missing_system_resolver_config(&denied));
    }

    #[test]
    fn record_type_parse_is_case_insensitive() {
        assert_eq!(parse_record_type(" mx ").unwrap(), RecordType::MX);
    }

    #[test]
    fn generic_record_uses_presentation_data() {
        let record = Record::from_rdata(
            Name::from_ascii("www.example.com.").unwrap(),
            300,
            RData::A(A::new(192, 0, 2, 1)),
        );

        let record = DnsRecord::from_hickory(&record);

        assert_eq!(record.name, "www.example.com.");
        assert_eq!(record.record_type, RecordType::A);
        assert_eq!(record.record_type_name(), "A");
        assert_eq!(record.ttl, 300);
        assert_eq!(record.data, "192.0.2.1");
    }

    #[test]
    fn no_data_is_distinct_from_nxdomain() {
        assert!(no_data_response(&no_records_error(ResponseCode::NoError)));
        assert!(!no_data_response(&no_records_error(ResponseCode::NXDomain)));
    }

    #[test]
    fn lookup_error_preserves_response_code() {
        let error = lookup_error(
            "example.com",
            RecordType::MX,
            NetError::Dns(DnsError::ResponseCode(ResponseCode::Refused)),
        );

        let Error::Lookup { response_code, .. } = error else {
            panic!("expected lookup error");
        };
        assert_eq!(response_code, Some(ResponseCode::Refused));
    }

    #[test]
    fn parses_https_record_alpn_port_and_hints() {
        let expires_at = Instant::now();
        let svcb = SVCB::new(
            1,
            Name::root(),
            vec![
                (
                    SvcParamKey::Alpn,
                    SvcParamValue::Alpn(Alpn(vec!["h2".to_owned(), "h3".to_owned()])),
                ),
                (SvcParamKey::Port, SvcParamValue::Port(8443)),
                (
                    SvcParamKey::Ipv4Hint,
                    SvcParamValue::Ipv4Hint(IpHint(vec![A::new(192, 0, 2, 1)])),
                ),
                (
                    SvcParamKey::Ipv6Hint,
                    SvcParamValue::Ipv6Hint(IpHint(vec![AAAA::new(
                        0x2001, 0xdb8, 0, 0, 0, 0, 0, 1,
                    )])),
                ),
            ],
        );

        let record = https_record_from_svcb(&svcb, expires_at);

        assert_eq!(record.priority, 1);
        assert!(record.has_alpn("h3"));
        assert_eq!(record.port, Some(8443));
        assert_eq!(record.ipv4_hints, vec![Ipv4Addr::new(192, 0, 2, 1)]);
        assert_eq!(
            record.ipv6_hints,
            vec![Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)]
        );
        assert_eq!(record.expires_at, expires_at);
    }

    fn no_records_error(response_code: ResponseCode) -> NetError {
        let query = Query::query(Name::from_ascii("example.com.").unwrap(), RecordType::TXT);
        NetError::Dns(DnsError::NoRecordsFound(NoRecords::new(
            query,
            response_code,
        )))
    }
}
