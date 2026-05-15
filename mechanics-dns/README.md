# mechanics-dns

Reusable DNS resolution helpers for Mechanics runtime crates.

The crate wraps `hickory-resolver` with the workspace resolver policy:
load the host resolver configuration when `/etc/resolv.conf` is
available, fall back to the documented Cloudflare resolver set only when
that file is missing, and surface any other system-configuration error to
the caller during resolver construction.

## Surface

- A and AAAA lookups.
- Combined IP and socket-address lookups for runtime clients.
- HTTPS RR lookup with ALPN, port, and address-hint extraction.
- Generic IN-class DNS queries returning presentation-format records
  for connector implementations.
- IN-class DNS only.

## Licensing

Licensed under either `Apache-2.0` or `MPL-2.0`, at your option.
