//! systemd-detect-virt(1)-style virtualization / container probe.
//!
//! Public surface: a single never-fail function
//! [`detect_virtualization`] that returns the
//! systemd-detect-virt-style identifier (`kvm`, `docker`,
//! `wsl`, etc.) for the current host, or `"none"` if no
//! virtualization is detected or the probe encounters any
//! internal error.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
// On non-Linux targets the Linux-only `/proc` / `/sys`
// probe helpers (`ProcFs` trait + `RealFs` impl,
// `read_text` / `read_bytes` / `path_exists`, `DmiFields`,
// `match_dmi_fields`, `any_contains`, `contains_ci`,
// the `ReadResult` / `PathStatus` enums) and a handful of
// `VmId` variants only reachable via DMI / sysinfo are
// genuinely unreached at runtime — the non-Linux fallback
// shims for each Linux-only probe function return
// `Ok(None)` / `Ok(Virt::None)` directly, and the only
// remaining live probe is CPUID on x86 / x86_64. The Linux
// target still gets full `dead_code` warnings; the gate
// here is precise to non-Linux.
#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

use std::io;
use std::panic::{self, RefUnwindSafe};

const IDS: &[&str] = &[
    "kvm",
    "qemu",
    "bochs",
    "xen",
    "uml",
    "vmware",
    "oracle",
    "microsoft",
    "zvm",
    "parallels",
    "bhyve",
    "qnx",
    "acrn",
    "powervm",
    "apple",
    "sre",
    "google",
    "amazon",
    "lxc",
    "lxc-libvirt",
    "systemd-nspawn",
    "docker",
    "podman",
    "rkt",
    "wsl",
    "proot",
    "pouch",
    "openvz",
    "none",
];

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
    detect_with_fs(&RealFs)
}

fn detect_with_fs<F>(fs: &F) -> &'static str
where
    F: ProcFs + RefUnwindSafe,
{
    // catch_unwind here is diagnostic / defense-in-depth for
    // unwinding (test / debug) builds. Production builds use
    // `panic = "abort"`, where catch_unwind is a no-op — a
    // panic aborts the process before this match runs. We
    // MUST NOT rely on it for the crate's never-fail contract;
    // the real guarantee comes from `detect(...)` being
    // structured to avoid panic-prone paths. The wrapper stays
    // because it costs nothing in release and keeps test-time
    // panics from propagating out of the crate boundary.
    // See workspace `CONTRIBUTING.md` §10.16.
    match panic::catch_unwind(|| detect(Mode::Any, fs)) {
        Ok(Ok(virt)) => documented_id(virt.id()),
        Ok(Err(_)) | Err(_) => "none",
    }
}

fn documented_id(id: &'static str) -> &'static str {
    if IDS.contains(&id) { id } else { "none" }
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    Any,
    VmOnly,
    ContainerOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VmId {
    Kvm,
    Qemu,
    Bochs,
    Xen,
    Uml,
    Vmware,
    Oracle,
    Microsoft,
    Zvm,
    Parallels,
    Bhyve,
    Qnx,
    Acrn,
    Powervm,
    Apple,
    Sre,
    Google,
    Amazon,
}

impl VmId {
    fn id(self) -> &'static str {
        match self {
            Self::Kvm => "kvm",
            Self::Qemu => "qemu",
            Self::Bochs => "bochs",
            Self::Xen => "xen",
            Self::Uml => "uml",
            Self::Vmware => "vmware",
            Self::Oracle => "oracle",
            Self::Microsoft => "microsoft",
            Self::Zvm => "zvm",
            Self::Parallels => "parallels",
            Self::Bhyve => "bhyve",
            Self::Qnx => "qnx",
            Self::Acrn => "acrn",
            Self::Powervm => "powervm",
            Self::Apple => "apple",
            Self::Sre => "sre",
            Self::Google => "google",
            Self::Amazon => "amazon",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ContainerId {
    Lxc,
    LxcLibvirt,
    SystemdNspawn,
    Docker,
    Podman,
    Rkt,
    Wsl,
    Proot,
    Pouch,
    Openvz,
}

impl ContainerId {
    fn id(self) -> &'static str {
        match self {
            Self::Lxc => "lxc",
            Self::LxcLibvirt => "lxc-libvirt",
            Self::SystemdNspawn => "systemd-nspawn",
            Self::Docker => "docker",
            Self::Podman => "podman",
            Self::Rkt => "rkt",
            Self::Wsl => "wsl",
            Self::Proot => "proot",
            Self::Pouch => "pouch",
            Self::Openvz => "openvz",
        }
    }

    fn from_id(id: &str) -> Option<Self> {
        match id.trim() {
            "lxc" => Some(Self::Lxc),
            "lxc-libvirt" => Some(Self::LxcLibvirt),
            "systemd-nspawn" => Some(Self::SystemdNspawn),
            "docker" => Some(Self::Docker),
            "podman" => Some(Self::Podman),
            "rkt" => Some(Self::Rkt),
            "wsl" => Some(Self::Wsl),
            "proot" => Some(Self::Proot),
            "pouch" => Some(Self::Pouch),
            "openvz" => Some(Self::Openvz),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Virt {
    None,
    Vm(VmId),
    Container(ContainerId),
}

impl Virt {
    fn id(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Vm(id) => id.id(),
            Self::Container(id) => id.id(),
        }
    }
}

fn detect(fs_mode: Mode, fs: &impl ProcFs) -> io::Result<Virt> {
    match fs_mode {
        Mode::Any => {
            let container = detect_container(fs)?;
            if container != Virt::None {
                return Ok(container);
            }
            detect_vm(fs)
        }
        Mode::VmOnly => detect_vm(fs),
        Mode::ContainerOnly => detect_container(fs),
    }
}

#[derive(Debug, Eq, PartialEq)]
enum ReadResult {
    Data(Vec<u8>),
    Absent,
    Denied,
}

#[derive(Debug, Eq, PartialEq)]
enum PathStatus {
    Exists,
    Absent,
    Denied,
}

trait ProcFs {
    fn read_file(&self, path: &str) -> io::Result<ReadResult>;
    fn path_status(&self, path: &str) -> io::Result<PathStatus>;
}

struct RealFs;

impl ProcFs for RealFs {
    fn read_file(&self, path: &str) -> io::Result<ReadResult> {
        match std::fs::read(path) {
            Ok(data) => Ok(ReadResult::Data(data)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(ReadResult::Absent),
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => Ok(ReadResult::Denied),
            Err(error) => Err(error),
        }
    }

    fn path_status(&self, path: &str) -> io::Result<PathStatus> {
        match std::fs::metadata(path) {
            Ok(_) => Ok(PathStatus::Exists),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(PathStatus::Absent),
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => Ok(PathStatus::Denied),
            Err(error) => Err(error),
        }
    }
}

fn read_text(fs: &impl ProcFs, path: &'static str) -> io::Result<Option<String>> {
    match fs.read_file(path)? {
        ReadResult::Data(data) => Ok(Some(String::from_utf8_lossy(&data).trim().to_owned())),
        ReadResult::Absent | ReadResult::Denied => Ok(None),
    }
}

fn read_bytes(fs: &impl ProcFs, path: &'static str) -> io::Result<Option<Vec<u8>>> {
    match fs.read_file(path)? {
        ReadResult::Data(data) => Ok(Some(data)),
        ReadResult::Absent | ReadResult::Denied => Ok(None),
    }
}

fn path_exists(fs: &impl ProcFs, path: &'static str) -> io::Result<bool> {
    match fs.path_status(path)? {
        PathStatus::Exists => Ok(true),
        PathStatus::Absent | PathStatus::Denied => Ok(false),
    }
}

fn detect_container(fs: &impl ProcFs) -> io::Result<Virt> {
    detect_container_impl(fs)
}

#[cfg(not(target_os = "linux"))]
fn detect_container_impl(_fs: &impl ProcFs) -> io::Result<Virt> {
    Ok(Virt::None)
}

#[cfg(target_os = "linux")]
fn detect_container_impl(fs: &impl ProcFs) -> io::Result<Virt> {
    if let Some(data) = read_bytes(fs, "/proc/1/environ")?
        && let Some(id) = parse_environ(&data)
    {
        return Ok(Virt::Container(id));
    }

    for path in ["/run/systemd/container", "/run/host/container-manager"] {
        if let Some(text) = read_text(fs, path)?
            && let Some(id) = id_from_manager_value(&text)
        {
            return Ok(Virt::Container(id));
        }
    }

    if path_exists(fs, "/.dockerenv")? {
        return Ok(Virt::Container(ContainerId::Docker));
    }
    if path_exists(fs, "/run/.containerenv")? {
        return Ok(Virt::Container(ContainerId::Podman));
    }

    if let Some(text) = read_text(fs, "/proc/sys/kernel/osrelease")? {
        let lower = text.to_ascii_lowercase();
        if lower.contains("microsoft") || lower.contains("wsl") {
            return Ok(Virt::Container(ContainerId::Wsl));
        }
    }

    let proc_vz = path_exists(fs, "/proc/vz")?;
    let proc_bc = path_exists(fs, "/proc/bc")?;
    if proc_vz && !proc_bc {
        return Ok(Virt::Container(ContainerId::Openvz));
    }

    Ok(Virt::None)
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn parse_environ(data: &[u8]) -> Option<ContainerId> {
    data.split(|byte| *byte == 0).find_map(|entry| {
        entry
            .strip_prefix(b"container=")
            .and_then(|value| std::str::from_utf8(value).ok())
            .and_then(id_from_manager_value)
    })
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn id_from_manager_value(value: &str) -> Option<ContainerId> {
    let trimmed = value.trim();
    if trimmed == "oci" {
        return None;
    }
    ContainerId::from_id(trimmed)
}

fn detect_vm(fs: &impl ProcFs) -> io::Result<Virt> {
    if let Some(id) = detect_dmi(fs)? {
        return Ok(Virt::Vm(id));
    }
    if let Some(id) = xen_capabilities(fs)? {
        return Ok(Virt::Vm(id));
    }
    if let Some(id) = hypervisor_type(fs)? {
        return Ok(Virt::Vm(id));
    }
    if let Some(id) = detect_cpuid() {
        return Ok(Virt::Vm(id));
    }
    if let Some(id) = device_tree(fs)? {
        return Ok(Virt::Vm(id));
    }
    if let Some(id) = sysinfo(fs)? {
        return Ok(Virt::Vm(id));
    }
    if let Some(id) = cpuinfo_uml(fs)? {
        return Ok(Virt::Vm(id));
    }
    Ok(Virt::None)
}

#[cfg(not(target_os = "linux"))]
fn xen_capabilities(_fs: &impl ProcFs) -> io::Result<Option<VmId>> {
    Ok(None)
}

#[cfg(target_os = "linux")]
fn xen_capabilities(fs: &impl ProcFs) -> io::Result<Option<VmId>> {
    if let Some(text) = read_text(fs, "/proc/xen/capabilities")? {
        if text.split(',').any(|part| part.trim() == "control_d") {
            return Ok(None);
        }
        return Ok(Some(VmId::Xen));
    }
    Ok(None)
}

#[cfg(not(target_os = "linux"))]
fn hypervisor_type(_fs: &impl ProcFs) -> io::Result<Option<VmId>> {
    Ok(None)
}

#[cfg(target_os = "linux")]
fn hypervisor_type(fs: &impl ProcFs) -> io::Result<Option<VmId>> {
    if read_text(fs, "/sys/hypervisor/type")?.as_deref() == Some("xen") {
        return Ok(Some(VmId::Xen));
    }
    Ok(None)
}

#[cfg(not(target_os = "linux"))]
fn device_tree(_fs: &impl ProcFs) -> io::Result<Option<VmId>> {
    Ok(None)
}

#[cfg(target_os = "linux")]
fn device_tree(fs: &impl ProcFs) -> io::Result<Option<VmId>> {
    if let Some(text) = read_text(fs, "/proc/device-tree/hypervisor/compatible")? {
        let lower = text.to_ascii_lowercase();
        if lower.contains("linux,kvm") {
            return Ok(Some(VmId::Kvm));
        }
        if lower.contains("xen") {
            return Ok(Some(VmId::Xen));
        }
        if lower.contains("vmware") {
            return Ok(Some(VmId::Vmware));
        }
    }

    let ibm_partition = path_exists(fs, "/proc/device-tree/ibm,partition-name")?;
    let hmc_managed = path_exists(fs, "/proc/device-tree/hmc-managed?")?;
    let qemu_graphic = path_exists(fs, "/proc/device-tree/chosen/qemu,graphic-width")?;
    if ibm_partition && hmc_managed && !qemu_graphic {
        return Ok(Some(VmId::Powervm));
    }

    if let Some(text) = read_text(fs, "/proc/device-tree/compatible")?
        && text.to_ascii_lowercase().contains("qemu,pseries")
    {
        return Ok(Some(VmId::Qemu));
    }
    Ok(None)
}

#[cfg(not(target_os = "linux"))]
fn sysinfo(_fs: &impl ProcFs) -> io::Result<Option<VmId>> {
    Ok(None)
}

#[cfg(target_os = "linux")]
fn sysinfo(fs: &impl ProcFs) -> io::Result<Option<VmId>> {
    if let Some(text) = read_text(fs, "/proc/sysinfo")? {
        for line in text.lines() {
            if let Some(value) = line
                .split_once(':')
                .and_then(|(key, value)| (key.trim() == "VM00 Control Program").then_some(value))
            {
                return if value.trim() == "z/VM" {
                    Ok(Some(VmId::Zvm))
                } else {
                    Ok(Some(VmId::Kvm))
                };
            }
        }
    }
    Ok(None)
}

#[cfg(not(target_os = "linux"))]
fn cpuinfo_uml(_fs: &impl ProcFs) -> io::Result<Option<VmId>> {
    Ok(None)
}

#[cfg(target_os = "linux")]
fn cpuinfo_uml(fs: &impl ProcFs) -> io::Result<Option<VmId>> {
    if let Some(text) = read_text(fs, "/proc/cpuinfo")? {
        for line in text.lines() {
            if let Some((key, value)) = line.split_once(':')
                && key.trim() == "vendor_id"
                && value.trim().starts_with("User Mode Linux")
            {
                return Ok(Some(VmId::Uml));
            }
        }
    }
    Ok(None)
}

const DMI_PATHS: &[&str] = &[
    "/sys/class/dmi/id/sys_vendor",
    "/sys/class/dmi/id/product_name",
    "/sys/class/dmi/id/bios_vendor",
    "/sys/class/dmi/id/chassis_vendor",
    "/sys/class/dmi/id/chassis_asset_tag",
];

#[derive(Default)]
struct DmiFields {
    sys_vendor: Option<String>,
    product_name: Option<String>,
    bios_vendor: Option<String>,
    chassis_vendor: Option<String>,
    chassis_asset_tag: Option<String>,
}

fn detect_dmi(fs: &impl ProcFs) -> io::Result<Option<VmId>> {
    detect_dmi_impl(fs)
}

#[cfg(not(target_os = "linux"))]
fn detect_dmi_impl(_fs: &impl ProcFs) -> io::Result<Option<VmId>> {
    Ok(None)
}

#[cfg(target_os = "linux")]
fn detect_dmi_impl(fs: &impl ProcFs) -> io::Result<Option<VmId>> {
    let mut fields = DmiFields::default();
    for path in DMI_PATHS {
        if let Some(text) = read_text(fs, path)? {
            match *path {
                "/sys/class/dmi/id/sys_vendor" => fields.sys_vendor = Some(text),
                "/sys/class/dmi/id/product_name" => fields.product_name = Some(text),
                "/sys/class/dmi/id/bios_vendor" => fields.bios_vendor = Some(text),
                "/sys/class/dmi/id/chassis_vendor" => fields.chassis_vendor = Some(text),
                "/sys/class/dmi/id/chassis_asset_tag" => fields.chassis_asset_tag = Some(text),
                _ => {}
            }
        }
    }
    Ok(match_dmi_fields(&fields))
}

fn match_dmi_fields(fields: &DmiFields) -> Option<VmId> {
    let values = [
        fields.sys_vendor.as_deref(),
        fields.product_name.as_deref(),
        fields.bios_vendor.as_deref(),
        fields.chassis_vendor.as_deref(),
        fields.chassis_asset_tag.as_deref(),
    ];

    if any_contains(&values, "KVM") {
        return Some(VmId::Kvm);
    }
    if any_contains(&values, "Amazon EC2") {
        return Some(VmId::Amazon);
    }
    if any_contains(&values, "Google") {
        return Some(VmId::Google);
    }
    if any_contains(&values, "VMware") || any_contains(&values, "VMW") {
        return Some(VmId::Vmware);
    }
    if any_contains(&values, "innotek GmbH") || any_contains(&values, "VirtualBox") {
        return Some(VmId::Oracle);
    }
    if fields
        .sys_vendor
        .as_deref()
        .is_some_and(|value| contains_ci(value, "Oracle Corporation"))
        && fields
            .chassis_vendor
            .as_deref()
            .is_some_and(|value| contains_ci(value, "Oracle"))
    {
        return Some(VmId::Oracle);
    }
    if any_contains(&values, "Xen") {
        return Some(VmId::Xen);
    }
    if any_contains(&values, "Bochs") {
        return Some(VmId::Bochs);
    }
    if any_contains(&values, "Parallels") {
        return Some(VmId::Parallels);
    }
    if any_contains(&values, "BHYVE") {
        return Some(VmId::Bhyve);
    }
    let microsoft_vendor = fields
        .sys_vendor
        .as_deref()
        .is_some_and(|value| contains_ci(value, "Microsoft Corporation"));
    let microsoft_product = fields
        .product_name
        .as_deref()
        .is_some_and(|value| contains_ci(value, "Virtual Machine"));
    if microsoft_vendor && microsoft_product {
        return Some(VmId::Microsoft);
    }
    if any_contains(&values, "Apple Virtualization") || any_contains(&values, "Apple VZ") {
        return Some(VmId::Apple);
    }
    if any_contains(&values, "QEMU") {
        return Some(VmId::Qemu);
    }
    None
}

fn any_contains(values: &[Option<&str>], needle: &str) -> bool {
    values
        .iter()
        .flatten()
        .any(|value| contains_ci(value, needle))
}

fn contains_ci(haystack: &str, needle: &str) -> bool {
    haystack
        .to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
fn detect_cpuid() -> Option<VmId> {
    None
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn detect_cpuid() -> Option<VmId> {
    let cpuid = raw_cpuid::CpuId::new();
    let features = cpuid.get_feature_info()?;
    if !features.has_hypervisor() {
        return None;
    }
    let info = cpuid.get_hypervisor_info()?;

    let raw_cpuid::Hypervisor::Unknown(ebx, ecx, edx) = info.identify() else {
        return match info.identify() {
            raw_cpuid::Hypervisor::Xen => Some(VmId::Xen),
            raw_cpuid::Hypervisor::VMware => Some(VmId::Vmware),
            raw_cpuid::Hypervisor::HyperV => Some(VmId::Microsoft),
            raw_cpuid::Hypervisor::KVM => Some(VmId::Kvm),
            raw_cpuid::Hypervisor::QEMU => Some(VmId::Qemu),
            raw_cpuid::Hypervisor::Bhyve => Some(VmId::Bhyve),
            raw_cpuid::Hypervisor::QNX => Some(VmId::Qnx),
            raw_cpuid::Hypervisor::ACRN => Some(VmId::Acrn),
            raw_cpuid::Hypervisor::Unknown(_, _, _) => None,
        };
    };

    match vendor_string(ebx, ecx, edx).trim_end_matches('\0') {
        "Linux KVM Hv" => Some(VmId::Kvm),
        "VBoxVBoxVBox" => Some(VmId::Oracle),
        "prl hyperv " | " lrpepyh vr" => Some(VmId::Parallels),
        "SRESRESRESRE" => Some(VmId::Sre),
        "Apple VZ" => Some(VmId::Apple),
        _ => None,
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn vendor_string(ebx: u32, ecx: u32, edx: u32) -> String {
    let mut bytes = Vec::with_capacity(12);
    bytes.extend_from_slice(&ebx.to_le_bytes());
    bytes.extend_from_slice(&ecx.to_le_bytes());
    bytes.extend_from_slice(&edx.to_le_bytes());
    String::from_utf8_lossy(&bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    struct FixtureFs {
        root: PathBuf,
    }

    impl FixtureFs {
        fn new(root: impl Into<PathBuf>) -> Self {
            Self { root: root.into() }
        }

        fn full_path(&self, path: &str) -> PathBuf {
            self.root.join(path.trim_start_matches('/'))
        }
    }

    impl ProcFs for FixtureFs {
        fn read_file(&self, path: &str) -> io::Result<ReadResult> {
            match std::fs::read(self.full_path(path)) {
                Ok(data) => Ok(ReadResult::Data(data)),
                Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(ReadResult::Absent),
                Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
                    Ok(ReadResult::Denied)
                }
                Err(error) => Err(error),
            }
        }

        fn path_status(&self, path: &str) -> io::Result<PathStatus> {
            match std::fs::metadata(self.full_path(path)) {
                Ok(_) => Ok(PathStatus::Exists),
                Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(PathStatus::Absent),
                Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
                    Ok(PathStatus::Denied)
                }
                Err(error) => Err(error),
            }
        }
    }

    struct PanicFs;

    impl ProcFs for PanicFs {
        fn read_file(&self, _path: &str) -> io::Result<ReadResult> {
            panic!("forced read panic")
        }

        fn path_status(&self, _path: &str) -> io::Result<PathStatus> {
            panic!("forced path panic")
        }
    }

    struct ErrorFs;

    impl ProcFs for ErrorFs {
        fn read_file(&self, _path: &str) -> io::Result<ReadResult> {
            Err(io::Error::other("forced read error"))
        }

        fn path_status(&self, _path: &str) -> io::Result<PathStatus> {
            Err(io::Error::other("forced path error"))
        }
    }

    fn fixture(name: &str) -> FixtureFs {
        FixtureFs::new(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../xtask/tests/fixtures/detect-virt")
                .join(name),
        )
    }

    fn detect_fixture(name: &str, mode: Mode) -> Virt {
        detect(mode, &fixture(name)).unwrap()
    }

    #[test]
    fn ids_include_none_last() {
        assert_eq!(IDS.last(), Some(&"none"));
    }

    #[test]
    fn parses_proc_1_environ_container_value() {
        let data = b"PATH=/usr/bin\0container=podman\0TERM=xterm\0";
        assert_eq!(parse_environ(data), Some(ContainerId::Podman));
    }

    #[test]
    fn unsupported_oci_environ_value_does_not_invent_an_id() {
        assert_eq!(parse_environ(b"container=oci\0"), None);
    }

    #[test]
    fn matches_representative_dmi_values() {
        let fields = DmiFields {
            sys_vendor: Some("Microsoft Corporation".to_owned()),
            product_name: Some("Virtual Machine".to_owned()),
            ..DmiFields::default()
        };
        assert_eq!(match_dmi_fields(&fields), Some(VmId::Microsoft));

        let fields = DmiFields {
            sys_vendor: Some("QEMU".to_owned()),
            product_name: Some("VirtualBox".to_owned()),
            ..DmiFields::default()
        };
        assert_eq!(match_dmi_fields(&fields), Some(VmId::Oracle));

        let fields = DmiFields {
            sys_vendor: Some("Amazon EC2".to_owned()),
            ..DmiFields::default()
        };
        assert_eq!(match_dmi_fields(&fields), Some(VmId::Amazon));
    }

    #[test]
    fn fixture_docker_on_kvm_is_innermost_by_default() {
        assert_eq!(
            detect_fixture("docker-on-kvm", Mode::Any),
            Virt::Container(ContainerId::Docker)
        );
        assert_eq!(
            detect_fixture("docker-on-kvm", Mode::VmOnly),
            Virt::Vm(VmId::Kvm)
        );
    }

    #[test]
    fn fixture_amazon_dmi_matches_vm() {
        assert_eq!(
            detect_fixture("kvm-on-amazon-ec2", Mode::VmOnly),
            Virt::Vm(VmId::Amazon)
        );
    }

    #[test]
    fn fixture_vanilla_bare_metal_matches_none() {
        assert_eq!(detect_fixture("vanilla-bare-metal", Mode::Any), Virt::None);
    }

    #[test]
    fn fixture_xen_dom0_is_ignored() {
        assert_eq!(detect_fixture("xen-dom0", Mode::VmOnly), Virt::None);
    }

    #[test]
    fn fixture_wsl_matches_container() {
        assert_eq!(
            detect_fixture("wsl", Mode::ContainerOnly),
            Virt::Container(ContainerId::Wsl)
        );
    }

    #[test]
    fn panic_path_returns_none() {
        assert_eq!(detect_with_fs(&PanicFs), "none");
    }

    #[test]
    fn io_error_path_returns_none() {
        assert_eq!(detect_with_fs(&ErrorFs), "none");
    }

    #[test]
    #[ignore = "host-dependent smoke test"]
    fn real_fs_smoke_result_is_documented() {
        assert!(IDS.contains(&detect_virtualization()));
    }
}
