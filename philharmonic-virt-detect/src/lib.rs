//! systemd-detect-virt(1)-style virtualization / container probe.
//!
//! Public surface: a single never-fail function
//! [`detect_virtualization`] that returns the
//! systemd-detect-virt-style identifier (`kvm`, `docker`,
//! `wsl`, etc.) for the current host, or `"none"` if no
//! virtualization is detected or the probe encounters any
//! internal error.
//!
//! ## Why a separate crate
//!
//! The probe logic was originally in the in-tree
//! `xtask/src/bin/detect-virt.rs` xtask binary, where it could
//! only be reached as a developer-side CLI. The
//! API-server deployment binary needs the same answer at
//! startup (exposed via `/v1/_meta/version` and rendered on
//! the Dashboard's API status panel), so the logic now lives
//! in this shared library and the xtask CLI becomes a thin
//! wrapper.
//!
//! ## Never-fail contract
//!
//! The public function MUST NOT return `Result`, MUST NOT
//! panic on reachable paths, and MUST return `"none"` for
//! every internal-error path (I/O, unexpected fixture
//! contents, CPUID failure on unsupported targets, etc.).
//! Callers can rely on the function being safe to call from
//! startup paths before logging is wired up.
//!
//! ## Current state
//!
//! This module currently ships a stub that always returns
//! `"none"`. The substantive probe implementation is being
//! moved over from `xtask/src/bin/detect-virt.rs` in a
//! follow-up round.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Probe for the current virtualization / container environment.
///
/// Returns one of the documented systemd-detect-virt
/// identifiers (`kvm`, `qemu`, `bochs`, `xen`, `uml`,
/// `vmware`, `oracle`, `microsoft`, `zvm`, `parallels`,
/// `bhyve`, `qnx`, `acrn`, `powervm`, `apple`, `sre`,
/// `google`, `amazon`, `lxc`, `lxc-libvirt`,
/// `systemd-nspawn`, `docker`, `podman`, `rkt`, `wsl`,
/// `proot`, `pouch`, `openvz`) as a `'static` string, or
/// `"none"` if no virtualization is detected.
///
/// Never panics on reachable paths; any internal failure
/// converts to `"none"`. Safe to call from startup paths
/// before logging is wired up.
pub fn detect_virtualization() -> &'static str {
    // Stub. The substantive probe logic will be moved over
    // from `xtask/src/bin/detect-virt.rs` in a follow-up
    // round; until then the stub satisfies the never-fail
    // contract with a conservative "none" answer.
    "none"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_returns_none() {
        assert_eq!(detect_virtualization(), "none");
    }

    #[test]
    fn return_value_is_static() {
        // Compile-time check that the return type is &'static str.
        let _bound: &'static str = detect_virtualization();
    }
}
