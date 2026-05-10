#![forbid(unsafe_code)]
//! detect-virt - portable `systemd-detect-virt(1)`-style probe.
//!
//! Probe order and most Linux heuristics follow systemd `src/basic/virt.c`.
//! This xtask version keeps Linux-only `/proc` and `/sys` signals behind an
//! injected filesystem so tests can use small fixture trees, and keeps CPUID
//! probing available on x86/x86_64 UNIX targets.

use std::io;

use clap::{ArgGroup, Parser};

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
enum Mode {
    Any,
    VmOnly,
    ContainerOnly,
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

    fn detected(self) -> bool {
        !matches!(self, Self::None)
    }
}

#[derive(Parser)]
#[command(
    name = "detect-virt",
    about = "Detect VM or container virtualization using systemd-detect-virt-compatible ids.",
    group(ArgGroup::new("mode").args(["vm", "container"]).multiple(false))
)]
struct Args {
    /// Only check full-machine hypervisors.
    #[arg(long, group = "mode")]
    vm: bool,

    /// Only check containers.
    #[arg(long, group = "mode")]
    container: bool,

    /// Suppress stdout and set only the exit code.
    #[arg(short, long)]
    quiet: bool,

    /// Print every id this tool can report.
    #[arg(long)]
    list: bool,

    /// Log every probe and result to stderr before final stdout.
    #[arg(long)]
    debug: bool,
}

fn main() {
    let args = Args::parse();
    if args.list {
        for id in list_ids() {
            println!("{id}");
        }
        return;
    }

    let mode = if args.vm {
        Mode::VmOnly
    } else if args.container {
        Mode::ContainerOnly
    } else {
        Mode::Any
    };

    let fs = procfs::RealFs;
    let mut log = ProbeLog::new(args.debug);
    let detected = if args.debug {
        detect_with_options(mode, &fs, DetectOptions::default(), &mut log)
    } else {
        detect(mode, &fs)
    };
    match detected {
        Ok(virt) => {
            log.emit();
            if !args.quiet {
                println!("{}", virt.id());
            }
            std::process::exit(if virt.detected() { 0 } else { 1 });
        }
        Err(e) => {
            log.emit();
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}

fn list_ids() -> &'static [&'static str] {
    IDS
}

fn detect(mode: Mode, fs: &impl procfs::ProcFs) -> io::Result<Virt> {
    let mut log = ProbeLog::new(false);
    detect_with_options(mode, fs, DetectOptions::default(), &mut log)
}

#[derive(Clone, Copy)]
struct DetectOptions {
    cpuid: bool,
}

impl Default for DetectOptions {
    fn default() -> Self {
        Self { cpuid: true }
    }
}

fn detect_with_options(
    mode: Mode,
    fs: &impl procfs::ProcFs,
    options: DetectOptions,
    log: &mut ProbeLog,
) -> io::Result<Virt> {
    match mode {
        Mode::Any => {
            let container = container::detect(fs, log)?;
            if container != Virt::None {
                return Ok(container);
            }
            vm::detect(fs, options, log)
        }
        Mode::VmOnly => vm::detect(fs, options, log),
        Mode::ContainerOnly => container::detect(fs, log),
    }
}

struct ProbeLog {
    enabled: bool,
    lines: Vec<String>,
}

impl ProbeLog {
    fn new(enabled: bool) -> Self {
        Self {
            enabled,
            lines: Vec::new(),
        }
    }

    fn push(&mut self, probe: &str, path: &str, result: impl Into<String>) {
        if self.enabled {
            self.lines
                .push(format!("{probe}: {path}: {}", result.into()));
        }
    }

    fn push_probe(&mut self, probe: &str, result: impl Into<String>) {
        if self.enabled {
            self.lines.push(format!("{probe}: {}", result.into()));
        }
    }

    fn emit(&self) {
        for line in &self.lines {
            eprintln!("{line}");
        }
    }
}

mod procfs {
    use super::*;
    #[cfg(test)]
    use std::path::PathBuf;

    #[derive(Debug, Eq, PartialEq)]
    pub enum ReadResult {
        Data(Vec<u8>),
        Absent,
        Denied,
    }

    #[derive(Debug, Eq, PartialEq)]
    pub enum PathStatus {
        Exists,
        Absent,
        Denied,
    }

    pub trait ProcFs {
        fn read_file(&self, path: &str) -> io::Result<ReadResult>;
        fn path_status(&self, path: &str) -> io::Result<PathStatus>;
    }

    pub struct RealFs;

    impl ProcFs for RealFs {
        fn read_file(&self, path: &str) -> io::Result<ReadResult> {
            match std::fs::read(path) {
                Ok(data) => Ok(ReadResult::Data(data)),
                Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(ReadResult::Absent),
                Err(e) if e.kind() == io::ErrorKind::PermissionDenied => Ok(ReadResult::Denied),
                Err(e) => Err(e),
            }
        }

        fn path_status(&self, path: &str) -> io::Result<PathStatus> {
            match std::fs::metadata(path) {
                Ok(_) => Ok(PathStatus::Exists),
                Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(PathStatus::Absent),
                Err(e) if e.kind() == io::ErrorKind::PermissionDenied => Ok(PathStatus::Denied),
                Err(e) => Err(e),
            }
        }
    }

    #[cfg(test)]
    pub struct FixtureFs {
        root: PathBuf,
    }

    #[cfg(test)]
    impl FixtureFs {
        pub fn new(root: impl Into<PathBuf>) -> Self {
            Self { root: root.into() }
        }

        fn full_path(&self, path: &str) -> PathBuf {
            self.root.join(path.trim_start_matches('/'))
        }
    }

    #[cfg(test)]
    impl ProcFs for FixtureFs {
        fn read_file(&self, path: &str) -> io::Result<ReadResult> {
            match std::fs::read(self.full_path(path)) {
                Ok(data) => Ok(ReadResult::Data(data)),
                Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(ReadResult::Absent),
                Err(e) if e.kind() == io::ErrorKind::PermissionDenied => Ok(ReadResult::Denied),
                Err(e) => Err(e),
            }
        }

        fn path_status(&self, path: &str) -> io::Result<PathStatus> {
            match std::fs::metadata(self.full_path(path)) {
                Ok(_) => Ok(PathStatus::Exists),
                Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(PathStatus::Absent),
                Err(e) if e.kind() == io::ErrorKind::PermissionDenied => Ok(PathStatus::Denied),
                Err(e) => Err(e),
            }
        }
    }
}

fn read_text(
    fs: &impl procfs::ProcFs,
    path: &'static str,
    probe: &'static str,
    log: &mut ProbeLog,
) -> io::Result<Option<String>> {
    match fs.read_file(path)? {
        procfs::ReadResult::Data(data) => {
            log.push(probe, path, "read");
            Ok(Some(String::from_utf8_lossy(&data).trim().to_owned()))
        }
        procfs::ReadResult::Absent => {
            log.push(probe, path, "skipped, file absent");
            Ok(None)
        }
        procfs::ReadResult::Denied => {
            log.push(probe, path, "skipped, permission denied");
            Ok(None)
        }
    }
}

fn read_bytes(
    fs: &impl procfs::ProcFs,
    path: &'static str,
    probe: &'static str,
    log: &mut ProbeLog,
) -> io::Result<Option<Vec<u8>>> {
    match fs.read_file(path)? {
        procfs::ReadResult::Data(data) => {
            log.push(probe, path, "read");
            Ok(Some(data))
        }
        procfs::ReadResult::Absent => {
            log.push(probe, path, "skipped, file absent");
            Ok(None)
        }
        procfs::ReadResult::Denied => {
            log.push(probe, path, "skipped, permission denied");
            Ok(None)
        }
    }
}

fn path_exists(
    fs: &impl procfs::ProcFs,
    path: &'static str,
    probe: &'static str,
    log: &mut ProbeLog,
) -> io::Result<bool> {
    match fs.path_status(path)? {
        procfs::PathStatus::Exists => {
            log.push(probe, path, "exists");
            Ok(true)
        }
        procfs::PathStatus::Absent => {
            log.push(probe, path, "skipped, file absent");
            Ok(false)
        }
        procfs::PathStatus::Denied => {
            log.push(probe, path, "skipped, permission denied");
            Ok(false)
        }
    }
}

mod container {
    use super::*;

    pub fn detect(fs: &impl procfs::ProcFs, log: &mut ProbeLog) -> io::Result<Virt> {
        detect_impl(fs, log)
    }

    #[cfg(not(target_os = "linux"))]
    fn detect_impl(_fs: &impl procfs::ProcFs, log: &mut ProbeLog) -> io::Result<Virt> {
        log.push_probe("container", "skipped, non-Linux target");
        Ok(Virt::None)
    }

    #[cfg(target_os = "linux")]
    fn detect_impl(fs: &impl procfs::ProcFs, log: &mut ProbeLog) -> io::Result<Virt> {
        if let Some(data) = read_bytes(fs, "/proc/1/environ", "container:environ", log)? {
            match parse_environ(&data) {
                Some(id) => {
                    log.push_probe("container:environ", format!("matched {}", id.id()));
                    return Ok(Virt::Container(id));
                }
                None => log.push_probe("container:environ", "read, no match"),
            }
        }

        if let Some(text) = read_text(
            fs,
            "/run/systemd/container",
            "container:systemd-container",
            log,
        )? {
            if let Some(id) = id_from_manager_value(&text) {
                log.push_probe(
                    "container:systemd-container",
                    format!("matched {}", id.id()),
                );
                return Ok(Virt::Container(id));
            }
            log.push_probe("container:systemd-container", "read, no match");
        }

        if let Some(text) = read_text(
            fs,
            "/run/host/container-manager",
            "container:host-manager",
            log,
        )? {
            if let Some(id) = id_from_manager_value(&text) {
                log.push_probe("container:host-manager", format!("matched {}", id.id()));
                return Ok(Virt::Container(id));
            }
            log.push_probe("container:host-manager", "read, no match");
        }

        if path_exists(fs, "/.dockerenv", "container:dockerenv", log)? {
            log.push_probe("container:dockerenv", "matched docker");
            return Ok(Virt::Container(ContainerId::Docker));
        }

        if path_exists(fs, "/run/.containerenv", "container:containerenv", log)? {
            log.push_probe("container:containerenv", "matched podman");
            return Ok(Virt::Container(ContainerId::Podman));
        }

        if let Some(text) = read_text(fs, "/proc/sys/kernel/osrelease", "container:osrelease", log)?
        {
            let lower = text.to_ascii_lowercase();
            if lower.contains("microsoft") || lower.contains("wsl") {
                log.push_probe("container:osrelease", "matched wsl");
                return Ok(Virt::Container(ContainerId::Wsl));
            }
            log.push_probe("container:osrelease", "read, no match");
        }

        let proc_vz = path_exists(fs, "/proc/vz", "container:openvz-vz", log)?;
        let proc_bc = path_exists(fs, "/proc/bc", "container:openvz-bc", log)?;
        if proc_vz && !proc_bc {
            log.push_probe("container:openvz", "matched openvz");
            return Ok(Virt::Container(ContainerId::Openvz));
        }

        if let Some(text) = read_text(fs, "/proc/1/sched", "container:sched", log)? {
            if let Some(first) = text.lines().next() {
                let weak = !first.starts_with("init ")
                    && !first.starts_with("systemd ")
                    && !first.starts_with("launchd ");
                if weak {
                    log.push_probe("container:sched", "read, weak unknown-container hint only");
                } else {
                    log.push_probe("container:sched", "read, no match");
                }
            } else {
                log.push_probe("container:sched", "read, no match");
            }
        }

        Ok(Virt::None)
    }

    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub fn parse_environ(data: &[u8]) -> Option<ContainerId> {
        data.split(|b| *b == 0).find_map(|entry| {
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
}

mod vm {
    use super::*;

    pub fn detect(
        fs: &impl procfs::ProcFs,
        options: DetectOptions,
        log: &mut ProbeLog,
    ) -> io::Result<Virt> {
        if let Some(id) = dmi::detect(fs, log)? {
            return Ok(Virt::Vm(id));
        }
        if let Some(id) = xen_capabilities(fs, log)? {
            return Ok(Virt::Vm(id));
        }
        if let Some(id) = hypervisor_type(fs, log)? {
            return Ok(Virt::Vm(id));
        }
        if options.cpuid {
            if let Some(id) = cpuid::detect(log) {
                return Ok(Virt::Vm(id));
            }
        } else {
            log.push_probe("vm:cpuid", "skipped, disabled by test options");
        }
        if let Some(id) = device_tree(fs, log)? {
            return Ok(Virt::Vm(id));
        }
        if let Some(id) = sysinfo(fs, log)? {
            return Ok(Virt::Vm(id));
        }
        if let Some(id) = cpuinfo_uml(fs, log)? {
            return Ok(Virt::Vm(id));
        }
        Ok(Virt::None)
    }

    #[cfg(not(target_os = "linux"))]
    fn xen_capabilities(_fs: &impl procfs::ProcFs, log: &mut ProbeLog) -> io::Result<Option<VmId>> {
        log.push_probe("vm:xen-capabilities", "skipped, non-Linux target");
        Ok(None)
    }

    #[cfg(target_os = "linux")]
    fn xen_capabilities(fs: &impl procfs::ProcFs, log: &mut ProbeLog) -> io::Result<Option<VmId>> {
        if let Some(text) = read_text(fs, "/proc/xen/capabilities", "vm:xen-capabilities", log)? {
            if text.split(',').any(|part| part.trim() == "control_d") {
                log.push_probe("vm:xen-capabilities", "read, Xen dom0 ignored");
                return Ok(None);
            }
            log.push_probe("vm:xen-capabilities", "matched xen");
            return Ok(Some(VmId::Xen));
        }
        Ok(None)
    }

    #[cfg(not(target_os = "linux"))]
    fn hypervisor_type(_fs: &impl procfs::ProcFs, log: &mut ProbeLog) -> io::Result<Option<VmId>> {
        log.push_probe("vm:hypervisor-type", "skipped, non-Linux target");
        Ok(None)
    }

    #[cfg(target_os = "linux")]
    fn hypervisor_type(fs: &impl procfs::ProcFs, log: &mut ProbeLog) -> io::Result<Option<VmId>> {
        if let Some(text) = read_text(fs, "/sys/hypervisor/type", "vm:hypervisor-type", log)? {
            if text == "xen" {
                log.push_probe("vm:hypervisor-type", "matched xen");
                return Ok(Some(VmId::Xen));
            }
            log.push_probe("vm:hypervisor-type", "read, no supported id");
        }
        Ok(None)
    }

    #[cfg(not(target_os = "linux"))]
    fn device_tree(_fs: &impl procfs::ProcFs, log: &mut ProbeLog) -> io::Result<Option<VmId>> {
        log.push_probe("vm:device-tree", "skipped, non-Linux target");
        Ok(None)
    }

    #[cfg(target_os = "linux")]
    fn device_tree(fs: &impl procfs::ProcFs, log: &mut ProbeLog) -> io::Result<Option<VmId>> {
        if let Some(text) = read_text(
            fs,
            "/proc/device-tree/hypervisor/compatible",
            "vm:device-tree-hypervisor",
            log,
        )? {
            let lower = text.to_ascii_lowercase();
            let id = if lower.contains("linux,kvm") {
                Some(VmId::Kvm)
            } else if lower.contains("xen") {
                Some(VmId::Xen)
            } else if lower.contains("vmware") {
                Some(VmId::Vmware)
            } else {
                None
            };
            if let Some(id) = id {
                log.push_probe("vm:device-tree-hypervisor", format!("matched {}", id.id()));
                return Ok(Some(id));
            }
            log.push_probe("vm:device-tree-hypervisor", "read, no supported id");
        }

        let ibm_partition = path_exists(
            fs,
            "/proc/device-tree/ibm,partition-name",
            "vm:device-tree-powervm-partition",
            log,
        )?;
        let hmc_managed = path_exists(
            fs,
            "/proc/device-tree/hmc-managed?",
            "vm:device-tree-powervm-hmc",
            log,
        )?;
        let qemu_graphic = path_exists(
            fs,
            "/proc/device-tree/chosen/qemu,graphic-width",
            "vm:device-tree-qemu-graphic",
            log,
        )?;
        if ibm_partition && hmc_managed && !qemu_graphic {
            log.push_probe("vm:device-tree-powervm", "matched powervm");
            return Ok(Some(VmId::Powervm));
        }

        if let Some(text) = read_text(
            fs,
            "/proc/device-tree/compatible",
            "vm:device-tree-compatible",
            log,
        )? {
            let lower = text.to_ascii_lowercase();
            if lower.contains("qemu,pseries") {
                log.push_probe("vm:device-tree-compatible", "matched qemu");
                return Ok(Some(VmId::Qemu));
            }
            log.push_probe("vm:device-tree-compatible", "read, no match");
        }
        Ok(None)
    }

    #[cfg(not(target_os = "linux"))]
    fn sysinfo(_fs: &impl procfs::ProcFs, log: &mut ProbeLog) -> io::Result<Option<VmId>> {
        log.push_probe("vm:sysinfo", "skipped, non-Linux target");
        Ok(None)
    }

    #[cfg(target_os = "linux")]
    fn sysinfo(fs: &impl procfs::ProcFs, log: &mut ProbeLog) -> io::Result<Option<VmId>> {
        if let Some(text) = read_text(fs, "/proc/sysinfo", "vm:sysinfo", log)? {
            for line in text.lines() {
                if let Some(value) = line.split_once(':').and_then(|(key, value)| {
                    (key.trim() == "VM00 Control Program").then_some(value.trim())
                }) {
                    let id = if value == "z/VM" {
                        VmId::Zvm
                    } else {
                        VmId::Kvm
                    };
                    log.push_probe("vm:sysinfo", format!("matched {}", id.id()));
                    return Ok(Some(id));
                }
            }
            log.push_probe("vm:sysinfo", "read, no match");
        }
        Ok(None)
    }

    #[cfg(not(target_os = "linux"))]
    fn cpuinfo_uml(_fs: &impl procfs::ProcFs, log: &mut ProbeLog) -> io::Result<Option<VmId>> {
        log.push_probe("vm:cpuinfo-uml", "skipped, non-Linux target");
        Ok(None)
    }

    #[cfg(target_os = "linux")]
    fn cpuinfo_uml(fs: &impl procfs::ProcFs, log: &mut ProbeLog) -> io::Result<Option<VmId>> {
        if let Some(text) = read_text(fs, "/proc/cpuinfo", "vm:cpuinfo-uml", log)? {
            for line in text.lines() {
                if let Some((key, value)) = line.split_once(':')
                    && key.trim() == "vendor_id"
                    && value.trim().starts_with("User Mode Linux")
                {
                    log.push_probe("vm:cpuinfo-uml", "matched uml");
                    return Ok(Some(VmId::Uml));
                }
            }
            log.push_probe("vm:cpuinfo-uml", "read, no match");
        }
        Ok(None)
    }
}

mod dmi {
    use super::*;

    const DMI_PATHS: &[&str] = &[
        "/sys/class/dmi/id/sys_vendor",
        "/sys/class/dmi/id/product_name",
        "/sys/class/dmi/id/bios_vendor",
        "/sys/class/dmi/id/chassis_vendor",
        "/sys/class/dmi/id/chassis_asset_tag",
    ];

    #[derive(Default)]
    pub struct DmiFields {
        pub sys_vendor: Option<String>,
        pub product_name: Option<String>,
        pub bios_vendor: Option<String>,
        pub chassis_vendor: Option<String>,
        pub chassis_asset_tag: Option<String>,
    }

    pub fn detect(fs: &impl procfs::ProcFs, log: &mut ProbeLog) -> io::Result<Option<VmId>> {
        detect_impl(fs, log)
    }

    #[cfg(not(target_os = "linux"))]
    fn detect_impl(_fs: &impl procfs::ProcFs, log: &mut ProbeLog) -> io::Result<Option<VmId>> {
        log.push_probe("vm:dmi", "skipped, non-Linux target");
        Ok(None)
    }

    #[cfg(target_os = "linux")]
    fn detect_impl(fs: &impl procfs::ProcFs, log: &mut ProbeLog) -> io::Result<Option<VmId>> {
        let mut fields = DmiFields::default();
        for path in DMI_PATHS {
            if let Some(text) = read_text(fs, path, "vm:dmi", log)? {
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
        let id = match_fields(&fields);
        if let Some(id) = id {
            log.push_probe("vm:dmi", format!("matched {}", id.id()));
        } else {
            log.push_probe("vm:dmi", "read, no match");
        }
        Ok(id)
    }

    pub fn match_fields(fields: &DmiFields) -> Option<VmId> {
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
        if let Some(sys_vendor) = fields.sys_vendor.as_deref()
            && contains_ci(sys_vendor, "Oracle Corporation")
            && fields
                .chassis_vendor
                .as_deref()
                .is_some_and(|s| contains_ci(s, "Oracle"))
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
            .is_some_and(|s| contains_ci(s, "Microsoft Corporation"));
        let microsoft_product = fields
            .product_name
            .as_deref()
            .is_some_and(|s| contains_ci(s, "Virtual Machine"));
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
}

mod cpuid {
    use super::*;

    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    pub fn detect(log: &mut ProbeLog) -> Option<VmId> {
        log.push_probe("vm:cpuid", "skipped, non-x86 target");
        None
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub fn detect(log: &mut ProbeLog) -> Option<VmId> {
        let cpuid = raw_cpuid::CpuId::new();
        let Some(features) = cpuid.get_feature_info() else {
            log.push_probe("vm:cpuid", "read, no feature info");
            return None;
        };
        if !features.has_hypervisor() {
            log.push_probe("vm:cpuid", "read, no hypervisor bit");
            return None;
        }
        let Some(info) = cpuid.get_hypervisor_info() else {
            log.push_probe("vm:cpuid", "read, hypervisor info absent");
            return None;
        };

        let raw_cpuid::Hypervisor::Unknown(ebx, ecx, edx) = info.identify() else {
            let id = match info.identify() {
                raw_cpuid::Hypervisor::Xen => VmId::Xen,
                raw_cpuid::Hypervisor::VMware => VmId::Vmware,
                raw_cpuid::Hypervisor::HyperV => VmId::Microsoft,
                raw_cpuid::Hypervisor::KVM => VmId::Kvm,
                raw_cpuid::Hypervisor::QEMU => VmId::Qemu,
                raw_cpuid::Hypervisor::Bhyve => VmId::Bhyve,
                raw_cpuid::Hypervisor::QNX => VmId::Qnx,
                raw_cpuid::Hypervisor::ACRN => VmId::Acrn,
                raw_cpuid::Hypervisor::Unknown(_, _, _) => return None,
            };
            log.push_probe("vm:cpuid", format!("matched {}", id.id()));
            return Some(id);
        };

        let vendor = vendor_string(ebx, ecx, edx);
        let id = match vendor.trim_end_matches('\0') {
            "Linux KVM Hv" => Some(VmId::Kvm),
            "VBoxVBoxVBox" => Some(VmId::Oracle),
            "prl hyperv " | " lrpepyh vr" => Some(VmId::Parallels),
            "SRESRESRESRE" => Some(VmId::Sre),
            "Apple VZ" => Some(VmId::Apple),
            _ => None,
        };
        if let Some(id) = id {
            log.push_probe(
                "vm:cpuid",
                format!("matched {} from vendor {vendor:?}", id.id()),
            );
            Some(id)
        } else {
            log.push_probe("vm:cpuid", format!("read, no match for {vendor:?}"));
            None
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn fixture(name: &str) -> procfs::FixtureFs {
        procfs::FixtureFs::new(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/detect-virt")
                .join(name),
        )
    }

    fn detect_fixture(name: &str, mode: Mode) -> Virt {
        let fs = fixture(name);
        let mut log = ProbeLog::new(false);
        detect_with_options(mode, &fs, DetectOptions { cpuid: false }, &mut log).unwrap()
    }

    #[test]
    fn list_ids_are_exact_and_include_none_last() {
        assert_eq!(list_ids(), IDS);
        assert_eq!(list_ids().last(), Some(&"none"));
    }

    #[test]
    fn parses_proc_1_environ_container_value() {
        let data = b"PATH=/usr/bin\0container=podman\0TERM=xterm\0";
        assert_eq!(container::parse_environ(data), Some(ContainerId::Podman));
    }

    #[test]
    fn unsupported_oci_environ_value_does_not_invent_an_id() {
        assert_eq!(container::parse_environ(b"container=oci\0"), None);
    }

    #[test]
    fn matches_representative_dmi_values() {
        let fields = dmi::DmiFields {
            sys_vendor: Some("Microsoft Corporation".to_owned()),
            product_name: Some("Virtual Machine".to_owned()),
            ..dmi::DmiFields::default()
        };
        assert_eq!(dmi::match_fields(&fields), Some(VmId::Microsoft));

        let fields = dmi::DmiFields {
            sys_vendor: Some("QEMU".to_owned()),
            product_name: Some("VirtualBox".to_owned()),
            ..dmi::DmiFields::default()
        };
        assert_eq!(dmi::match_fields(&fields), Some(VmId::Oracle));

        let fields = dmi::DmiFields {
            sys_vendor: Some("Amazon EC2".to_owned()),
            ..dmi::DmiFields::default()
        };
        assert_eq!(dmi::match_fields(&fields), Some(VmId::Amazon));
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
    fn fixture_vanilla_bare_metal_matches_none_without_cpuid() {
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
    fn real_fs_smoke_test_does_not_panic() {
        let fs = procfs::RealFs;
        assert!(detect(Mode::Any, &fs).is_ok());
    }
}
