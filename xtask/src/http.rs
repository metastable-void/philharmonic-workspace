//! Shared HTTP transport for xtask bins that talk to crates.io
//! (and anywhere else that benefits from a single ureq+rustls+UA
//! path).
//!
//! Mirrors the defaults used by `src/bin/web-fetch.rs`:
//!   - User-Agent: `$WEB_FETCH_UA` or `philharmonic-dev-agent/1.0`.
//!   - TLS: ureq 3's default (rustls).
//!   - HTTP 4xx/5xx: returned as `HttpError::StatusCode`; the
//!     caller decides whether to recover (e.g., 404 on a missing
//!     crate is normal) or surface the failure.
//!
//! Only text-body responses for now. Add a streaming variant
//! when a second non-text consumer appears; `web-fetch` stays
//! inline until then.

use std::io::Read;

const DEFAULT_UA: &str = "philharmonic-dev-agent/1.0";

#[derive(Debug)]
pub enum HttpError {
    /// Upstream returned a 4xx/5xx status.
    StatusCode { code: u16 },
    /// Transport failure (DNS, TLS, connection, timeout, …).
    Transport(String),
    /// Body was received but reading/decoding it to UTF-8 failed.
    Read(String),
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StatusCode { code } => write!(f, "HTTP {code}"),
            Self::Transport(m) => write!(f, "transport error: {m}"),
            Self::Read(m) => write!(f, "read error: {m}"),
        }
    }
}

impl std::error::Error for HttpError {}

/// GET `url` and return the body as UTF-8 text.
///
/// Sets `User-Agent` from `WEB_FETCH_UA` (or the workspace
/// default). All outbound HTTP from xtask bins that want the
/// web-fetch discipline should go through this.
pub fn fetch_text(url: &str) -> Result<String, HttpError> {
    let ua = std::env::var("WEB_FETCH_UA").unwrap_or_else(|_| DEFAULT_UA.to_owned());

    let response = match ureq::get(url).header("User-Agent", &ua).call() {
        Ok(r) => r,
        Err(ureq::Error::StatusCode(code)) => return Err(HttpError::StatusCode { code }),
        Err(e) => return Err(HttpError::Transport(e.to_string())),
    };

    let mut reader = response.into_body().into_reader();
    let mut buf = String::new();
    reader
        .read_to_string(&mut buf)
        .map_err(|e| HttpError::Read(e.to_string()))?;
    Ok(buf)
}
