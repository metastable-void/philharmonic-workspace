# Mechanics DNS extraction

**Date:** 2026-05-15
**Prompt:** direct Codex dispatch from `docs/ROADMAP.md` §3.L

The D26 extraction now lives in the in-tree `mechanics-dns` crate.
The crate owns resolver construction, the Cloudflare fallback list,
generic IN-class DNS query support, and parsed HTTPS RR records. Its
public surface exposes A, AAAA, combined IP, socket-address, HTTPS RR,
and generic presentation-format record lookups.

Two policy choices are intentionally centralised there:

- Fallback to Cloudflare is only triggered when hickory reports an
  `io::ErrorKind::NotFound` while loading system resolver
  configuration. Permission errors, malformed resolver config, and
  other I/O failures remain `Resolver::new()` errors.
- The hickory response cache is disabled (`cache_size = 0`) and
  nameservers use `ServerOrderingStrategy::UserProvidedOrder`. This
  keeps the new long-lived resolver from adding process-level DNS
  response caching or resolver reordering on top of the host's
  configured policy.

`mechanics-http-client` now constructs one `mechanics_dns::Resolver`
per client build. The hyper-util TCP/TLS connector receives a clone
through a small local `Service<Name>` adapter, and the HTTP/3 path
uses the same resolver for HTTPS RR lookup and fallback A/AAAA socket
resolution. The old `tokio::net::lookup_host` and inline
`hickory_resolver::TokioResolver` call sites are gone from mhc.

The future DNS connector can use `Resolver::query()` plus the public
`RecordType` and `ResponseCode` re-exports for canonical RR-type parsing
and DNS RCODE mapping, while still emitting its own workflow response
shape from the lightweight `DnsRecord` values. `NOERROR` responses with
no matching records return an empty vector; `NXDOMAIN`, `SERVFAIL`,
`NOTIMP`, `REFUSED`, and other DNS errors remain `Error::Lookup`
values carrying the surfaced `ResponseCode`.

Follow-up for release orchestration: `mechanics-dns` is scaffolded and
versioned as `0.1.0`, but this task did not publish or reserve the
crate on crates.io.
