# philharmonic-virt-detect

Portable `systemd-detect-virt(1)`-style virtualization /
container probe.

Single public function: `detect_virtualization()` returns the
detected ID (`kvm`, `docker`, `wsl`, etc.) for the current
host, or `"none"` if no virtualization is detected or any
internal error occurs.

## Never-fail contract

The public function MUST NOT return `Result`, MUST NOT panic
on reachable paths, and MUST return `"none"` for every
internal-error path. Callers can rely on the function being
safe to call from startup paths before logging is wired up.

## Why a separate crate

The probe logic was originally in the in-tree
`xtask/src/bin/detect-virt.rs` xtask binary. The API-server
deployment binary needs the same answer at startup (exposed
via `/v1/_meta/version` and rendered on the Dashboard's API
status panel), so the logic now lives in this shared library
and the xtask CLI becomes a thin wrapper.

## License

Dual-licensed under Apache-2.0 OR MPL-2.0.
