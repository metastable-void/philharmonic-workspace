# Changelog

All notable changes to this crate are documented in this file.

The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this crate adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-05-15

### Added
- Initial resolver API for A, AAAA, combined IP, socket-address,
  HTTPS RR, and generic DNS queries.
- System resolver initialisation with Cloudflare fallback only when
  `/etc/resolv.conf` is missing; other system-configuration errors
  remain startup-level failures.
