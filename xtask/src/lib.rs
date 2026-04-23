//! Shared helpers for in-workspace xtask bins.
//!
//! Right now: a single `http` module so every xtask bin that
//! talks to crates.io (`crates-io-versions` today, whatever
//! future bins need the same UA + rustls discipline) goes
//! through one transport path. The streaming-body case (writing
//! large responses to stdout/file) stays inline in
//! `src/bin/web-fetch.rs` for now — refactor into this helper
//! when a second streaming consumer appears.

pub mod http;
