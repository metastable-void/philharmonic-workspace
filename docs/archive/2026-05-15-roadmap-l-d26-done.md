# 2026-05-15 ROADMAP §3.L trim — D26 (`mechanics-dns`) done

Pre-trim verbatim §3.L as it stood after D26 landed. Trimmed
because the body re-stated the pre-implementation problem
analysis and the dispatch spec, neither of which is current
state once the dispatch has shipped. The live ROADMAP carries
a one-paragraph done-pointer at the same location.

Prior trim archive:
[`2026-05-15-roadmap-audit-refactor-slices-1-2.md`](2026-05-15-roadmap-audit-refactor-slices-1-2.md).

---

## Verbatim §3.L — `mechanics-dns` extraction + mhc resolver migration (1 dispatch) — DONE

DONE 2026-05-15. D26 scaffolded the in-tree `mechanics-dns`
crate and migrated `mechanics-http-client`'s HTTPS-RR lookup,
HTTP/3 socket-address fallback, and hyper-util h1/h2/HTTPS
connector resolver onto it. `mechanics-dns` exposes IN-class
generic DNS query, HTTPS RR, A, AAAA, combined IP, and
socket-address lookup helpers. It loads the host resolver
configuration when present,
falls back to the documented Cloudflare resolver set only on
missing `/etc/resolv.conf`, and surfaces other system-config
failures during resolver construction. See
[`docs/codex-reports/2026-05-15-0005-mechanics-dns-extraction.md`](../codex-reports/2026-05-15-0005-mechanics-dns-extraction.md)
for implementation notes.

The current `mechanics-http-client` is fragile on hosts
without `/etc/resolv.conf` (typical in distroless / scratch
container images): its HTTPS-RR lookup uses
`hickory_resolver::TokioResolver` which errors on
`read_system_conf` ENOENT, and its h1/h2/HTTPS dial path
uses `tokio::net::lookup_host` (libc `getaddrinfo`) which
fails the same way. The D19 DNS connector
([§3.B](../ROADMAP.md#b-phase-7-tier-23-connector-implementations-4-dispatches))
is spec'd to fall back to Cloudflare resolvers on ENOENT;
mhc currently isn't. Both need the same behaviour, and the
list shouldn't live in two places.

- **D26** (`mechanics-dns`, new in-tree non-submodule
  crate). Scaffold a new workspace member at
  `./mechanics-dns/` (same shape as
  `mechanics-h3-quinn`: in-tree, no git submodule, but
  published to crates.io so external consumers of mhc /
  the D19 connector resolve it normally). Library API:
  - Generic DNS query support for connector-impl-dns
  - HTTPS-RR lookup
  - A / AAAA lookup
  - `IN`-class only
  - On `/etc/resolv.conf` ENOENT, fall back to the
    [Cloudflare fallback resolver set](../design/08-connector-architecture.md#cloudflare-fallback-resolver-set);
    any other read error surfaces as a startup-level
    failure rather than silently falling back.

  Migrate `mechanics-http-client`'s HTTPS-RR call site
  and its `tokio::net::lookup_host` callers (HTTP/3
  `first_socket_addr`, h1/h2/HTTPS hyper-util connector)
  to use `mechanics-dns`. Behaviour on hosts with
  `/etc/resolv.conf` is unchanged; behaviour on ENOENT
  hosts becomes "use the Cloudflare set" instead of
  "fail."

  Dispatches independently of the Tier-2 batch. D19 now
  consumes `mechanics-dns` directly rather than
  re-implementing the fallback.
