//! vendor-upstream — vendor selected crates.io release files into
//! in-tree workspace members.
//!
//! Usage:
//!   ./scripts/xtask.sh vendor-upstream
//!   ./scripts/xtask.sh vendor-upstream -- --entry h3-quinn
//!   ./scripts/xtask.sh vendor-upstream -- --check
//!
//! Reads `vendor/vendor.toml` from the workspace root. Each
//! `[[entry]]` names a crates.io crate/version, a target path,
//! and sync globs. The tool fetches crates.io sparse-index
//! metadata from `https://index.crates.io`, refuses releases
//! younger than the three-day cooldown, downloads the `.crate`
//! tarball from `https://static.crates.io`, verifies SHA-256
//! against the index `cksum`, extracts into a temporary
//! directory, and syncs only matched files into the target.
//!
//! `Cargo.toml` is always protected: even if a broad sync glob
//! matches it, the hand-maintained target manifest is never
//! overwritten or deleted.
//!
//! Exit codes:
//!   0    all selected entries are up to date / written.
//!   1    input, manifest, checksum, cooldown, or check failure.
//!   2    network / HTTP failure.
//!   3    local I/O failure.

use chrono::{DateTime, SecondsFormat, Utc};
use clap::Parser;
use flate2::read::GzDecoder;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use tar::Archive;
use xtask::http::{self, HttpError};

const MANIFEST_PATH: &str = "vendor/vendor.toml";
const STAMP_NAME: &str = ".vendor-stamp.toml";
const COOLDOWN_DAYS: i64 = 3;
const COOLDOWN_SECONDS: i64 = COOLDOWN_DAYS * 86_400;

#[derive(Parser)]
#[command(
    name = "vendor-upstream",
    about = "Vendor selected crates.io release files into in-tree workspace members from vendor/vendor.toml.",
    after_help = "Examples:\n  ./scripts/xtask.sh vendor-upstream\n  ./scripts/xtask.sh vendor-upstream -- --entry h3-quinn\n  ./scripts/xtask.sh vendor-upstream -- --check"
)]
struct Args {
    /// Process only the entry whose upstream_name or target_path matches this value.
    #[arg(long)]
    entry: Option<String>,

    /// Read-only mode: verify selected entries are already in sync.
    #[arg(long)]
    check: bool,
}

#[derive(Debug, Deserialize)]
struct VendorManifest {
    entry: Vec<VendorEntry>,
}

#[derive(Debug, Deserialize)]
struct VendorEntry {
    upstream_name: String,
    upstream_version: String,
    target_path: PathBuf,
    sync: Vec<String>,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct IndexEntry {
    name: String,
    vers: String,
    cksum: String,
    yanked: bool,
    #[serde(default)]
    pubtime: Option<String>,
}

struct EntryPlan {
    target_path: PathBuf,
    upstream_sha256: String,
    matched_files: BTreeMap<PathBuf, PathBuf>,
    stale_files: BTreeSet<PathBuf>,
}

#[derive(Default)]
struct SyncStats {
    copied: usize,
    unchanged: usize,
    removed: usize,
    bytes: u64,
    changed: bool,
}

fn main() -> ExitCode {
    match run(Args::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(AppError::Input(msg)) => {
            eprintln!("!!! vendor-upstream: {msg}");
            ExitCode::from(1)
        }
        Err(AppError::Network(msg)) => {
            eprintln!("!!! vendor-upstream: {msg}");
            ExitCode::from(2)
        }
        Err(AppError::Io(msg)) => {
            eprintln!("!!! vendor-upstream: {msg}");
            ExitCode::from(3)
        }
    }
}

#[derive(Debug)]
enum AppError {
    Input(String),
    Network(String),
    Io(String),
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

fn run(args: Args) -> Result<(), AppError> {
    let workspace = std::env::current_dir()
        .map_err(|e| AppError::Io(format!("cannot determine current directory: {e}")))?;
    let manifest = read_manifest(&workspace.join(MANIFEST_PATH))?;

    let selected = select_entries(&manifest.entry, args.entry.as_deref())?;
    let mut check_failed = false;

    for entry in selected {
        let index = fetch_index_entry(&entry.upstream_name, &entry.upstream_version)?;
        enforce_cooldown(&index, Utc::now())?;
        let tarball = fetch_tarball(&entry.upstream_name, &entry.upstream_version)?;
        let digest = sha256_hex(&tarball);
        if digest != index.cksum {
            return Err(AppError::Input(format!(
                "{} {} checksum mismatch: index {}, downloaded {}",
                entry.upstream_name, entry.upstream_version, index.cksum, digest
            )));
        }

        let tmp = TempDir::new("vendor-upstream")?;
        extract_crate(&tarball, tmp.path())?;
        let source_root = tmp.path().join(format!(
            "{}-{}",
            entry.upstream_name, entry.upstream_version
        ));
        if !source_root.is_dir() {
            return Err(AppError::Input(format!(
                "tarball did not contain expected root directory {}",
                source_root.display()
            )));
        }

        let target_path = workspace.join(&entry.target_path);
        if !target_path.is_dir() {
            return Err(AppError::Input(format!(
                "target_path {} does not exist or is not a directory",
                entry.target_path.display()
            )));
        }

        let plan = build_plan(entry, &source_root, &target_path, digest)?;
        let stats = if args.check {
            check_plan(entry, &plan)?
        } else {
            apply_plan(entry, &plan)?
        };

        let mode = if args.check { "checked" } else { "vendored" };
        eprintln!(
            "vendor-upstream: {mode} {} {} -> {} (copied {}, unchanged {}, removed {}, {} bytes)",
            entry.upstream_name,
            entry.upstream_version,
            entry.target_path.display(),
            stats.copied,
            stats.unchanged,
            stats.removed,
            stats.bytes
        );

        if args.check && stats.changed {
            check_failed = true;
        }
    }

    if check_failed {
        return Err(AppError::Input(
            "one or more entries are not up to date".to_owned(),
        ));
    }

    Ok(())
}

fn read_manifest(path: &Path) -> Result<VendorManifest, AppError> {
    let raw = fs::read_to_string(path)
        .map_err(|e| AppError::Input(format!("cannot read {}: {e}", path.display())))?;
    parse_manifest(&raw)
}

fn parse_manifest(raw: &str) -> Result<VendorManifest, AppError> {
    let manifest: VendorManifest = toml::from_str(raw)
        .map_err(|e| AppError::Input(format!("invalid vendor manifest TOML: {e}")))?;
    if manifest.entry.is_empty() {
        return Err(AppError::Input(
            "vendor manifest must contain at least one [[entry]]".to_owned(),
        ));
    }
    for entry in &manifest.entry {
        validate_entry(entry)?;
    }
    Ok(manifest)
}

fn validate_entry(entry: &VendorEntry) -> Result<(), AppError> {
    if entry.upstream_name.trim().is_empty() {
        return Err(AppError::Input("entry upstream_name is empty".to_owned()));
    }
    if entry.upstream_version.trim().is_empty() {
        return Err(AppError::Input(format!(
            "entry {} has empty upstream_version",
            entry.upstream_name
        )));
    }
    if entry.target_path.is_absolute()
        || entry
            .target_path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(AppError::Input(format!(
            "entry {} target_path must be workspace-relative and cannot contain ..",
            entry.upstream_name
        )));
    }
    if entry.sync.is_empty() {
        return Err(AppError::Input(format!(
            "entry {} must contain at least one sync glob",
            entry.upstream_name
        )));
    }
    for pattern in &entry.sync {
        validate_sync_pattern(pattern).map_err(|msg| {
            AppError::Input(format!(
                "entry {} invalid sync glob: {msg}",
                entry.upstream_name
            ))
        })?;
    }
    Ok(())
}

fn validate_sync_pattern(pattern: &str) -> Result<(), String> {
    if pattern.trim().is_empty() {
        return Err("empty pattern".to_owned());
    }
    let path = Path::new(pattern);
    if path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(format!(
            "{pattern:?} must be relative and cannot contain .."
        ));
    }
    Ok(())
}

fn select_entries<'a>(
    entries: &'a [VendorEntry],
    selected: Option<&str>,
) -> Result<Vec<&'a VendorEntry>, AppError> {
    let Some(selected) = selected else {
        return Ok(entries.iter().collect());
    };
    let matched: Vec<&VendorEntry> = entries
        .iter()
        .filter(|e| e.upstream_name == selected || e.target_path == Path::new(selected))
        .collect();
    if matched.is_empty() {
        return Err(AppError::Input(format!(
            "no vendor entry matches --entry {selected}"
        )));
    }
    Ok(matched)
}

fn fetch_index_entry(name: &str, version: &str) -> Result<IndexEntry, AppError> {
    let url = format!("https://index.crates.io/{}", sparse_index_path(name));
    let body = match http::fetch_text(&url) {
        Ok(body) => body,
        Err(HttpError::StatusCode { code: 404 }) => {
            return Err(AppError::Input(format!(
                "crate {name} not found in crates.io sparse index"
            )));
        }
        Err(HttpError::StatusCode { code }) => {
            return Err(AppError::Network(format!(
                "HTTP {code} fetching sparse-index metadata for {name}"
            )));
        }
        Err(e) => {
            return Err(AppError::Network(format!(
                "fetch sparse-index metadata for {name}: {e}"
            )));
        }
    };

    for line in body.lines().filter(|line| !line.trim().is_empty()) {
        let entry: IndexEntry = serde_json::from_str(line)
            .map_err(|e| AppError::Input(format!("malformed sparse-index line for {name}: {e}")))?;
        if entry.vers == version {
            if entry.name != name {
                return Err(AppError::Input(format!(
                    "sparse-index entry name mismatch for {name} {version}: {}",
                    entry.name
                )));
            }
            if entry.yanked {
                return Err(AppError::Input(format!("{name} {version} is yanked")));
            }
            return Ok(entry);
        }
    }

    Err(AppError::Input(format!(
        "{name} {version} not found in crates.io sparse index"
    )))
}

fn sparse_index_path(name: &str) -> String {
    let name = name.to_lowercase();
    match name.len() {
        1 => format!("1/{name}"),
        2 => format!("2/{name}"),
        3 => format!("3/{}/{}", &name[..1], name),
        _ => format!("{}/{}/{}", &name[..2], &name[2..4], name),
    }
}

fn enforce_cooldown(entry: &IndexEntry, now: DateTime<Utc>) -> Result<(), AppError> {
    let published = parse_pubtime(entry)?;
    let age = now.signed_duration_since(published).num_seconds();
    if age < COOLDOWN_SECONDS {
        return Err(AppError::Input(format!(
            "{} {} is younger than the {COOLDOWN_DAYS}-day cooldown (published {})",
            entry.name,
            entry.vers,
            published.to_rfc3339_opts(SecondsFormat::Secs, true)
        )));
    }
    Ok(())
}

fn parse_pubtime(entry: &IndexEntry) -> Result<DateTime<Utc>, AppError> {
    let raw = entry.pubtime.as_deref().ok_or_else(|| {
        AppError::Input(format!(
            "{} {} sparse-index entry has no pubtime; cannot enforce cooldown",
            entry.name, entry.vers
        ))
    })?;
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| {
            AppError::Input(format!(
                "{} {} has malformed pubtime {raw:?}: {e}",
                entry.name, entry.vers
            ))
        })
}

fn fetch_tarball(name: &str, version: &str) -> Result<Vec<u8>, AppError> {
    let url = format!("https://static.crates.io/crates/{name}/{name}-{version}.crate");
    match http::fetch_bytes(&url) {
        Ok(bytes) => Ok(bytes),
        Err(HttpError::StatusCode { code }) => Err(AppError::Network(format!(
            "HTTP {code} downloading {name} {version} tarball"
        ))),
        Err(e) => Err(AppError::Network(format!(
            "download {name} {version} tarball: {e}"
        ))),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn extract_crate(bytes: &[u8], dest: &Path) -> Result<(), AppError> {
    let decoder = GzDecoder::new(Cursor::new(bytes));
    let mut archive = Archive::new(decoder);
    archive.unpack(dest).map_err(|e| {
        AppError::Io(format!(
            "extract crate tarball into {}: {e}",
            dest.display()
        ))
    })
}

fn build_plan(
    entry: &VendorEntry,
    source_root: &Path,
    target_path: &Path,
    upstream_sha256: String,
) -> Result<EntryPlan, AppError> {
    let mut matched_files = BTreeMap::new();
    for rel in list_files(source_root)? {
        if is_protected_path(&rel) {
            continue;
        }
        if matches_any(&rel, &entry.sync) {
            matched_files.insert(rel.clone(), source_root.join(&rel));
        }
    }

    let mut stale_files = BTreeSet::new();
    for rel in list_files(target_path)? {
        if is_protected_path(&rel) || rel == Path::new(STAMP_NAME) {
            continue;
        }
        if matches_any(&rel, &entry.sync) && !matched_files.contains_key(&rel) {
            stale_files.insert(rel);
        }
    }

    Ok(EntryPlan {
        target_path: target_path.to_owned(),
        upstream_sha256,
        matched_files,
        stale_files,
    })
}

fn apply_plan(entry: &VendorEntry, plan: &EntryPlan) -> Result<SyncStats, AppError> {
    let mut stats = SyncStats::default();
    for (rel, source) in &plan.matched_files {
        let dest = plan.target_path.join(rel);
        let source_bytes = fs::read(source)
            .map_err(|e| AppError::Io(format!("read {}: {e}", source.display())))?;
        stats.bytes = stats
            .bytes
            .checked_add(source_bytes.len() as u64)
            .ok_or_else(|| AppError::Input("synced byte count overflowed u64".to_owned()))?;

        if file_matches(&dest, &source_bytes)? {
            stats.unchanged += 1;
            continue;
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| AppError::Io(format!("create {}: {e}", parent.display())))?;
        }
        fs::write(&dest, &source_bytes)
            .map_err(|e| AppError::Io(format!("write {}: {e}", dest.display())))?;
        stats.copied += 1;
        stats.changed = true;
    }

    for rel in &plan.stale_files {
        let path = plan.target_path.join(rel);
        fs::remove_file(&path)
            .map_err(|e| AppError::Io(format!("remove stale {}: {e}", path.display())))?;
        stats.removed += 1;
        stats.changed = true;
    }
    remove_empty_dirs(&plan.target_path)?;

    let stamp = stamp_text(entry, &plan.upstream_sha256, Utc::now());
    let stamp_path = plan.target_path.join(STAMP_NAME);
    if !file_matches(&stamp_path, stamp.as_bytes())? {
        fs::write(&stamp_path, stamp)
            .map_err(|e| AppError::Io(format!("write {}: {e}", stamp_path.display())))?;
        stats.changed = true;
    }

    Ok(stats)
}

fn check_plan(entry: &VendorEntry, plan: &EntryPlan) -> Result<SyncStats, AppError> {
    let mut stats = SyncStats::default();
    for (rel, source) in &plan.matched_files {
        let source_bytes = fs::read(source)
            .map_err(|e| AppError::Io(format!("read {}: {e}", source.display())))?;
        stats.bytes = stats
            .bytes
            .checked_add(source_bytes.len() as u64)
            .ok_or_else(|| AppError::Input("synced byte count overflowed u64".to_owned()))?;

        let dest = plan.target_path.join(rel);
        if file_matches(&dest, &source_bytes)? {
            stats.unchanged += 1;
        } else {
            eprintln!("!! vendor-upstream: would update {}", dest.display());
            stats.copied += 1;
            stats.changed = true;
        }
    }

    for rel in &plan.stale_files {
        eprintln!(
            "!! vendor-upstream: would remove stale {}",
            plan.target_path.join(rel).display()
        );
        stats.removed += 1;
        stats.changed = true;
    }

    let stamp_path = plan.target_path.join(STAMP_NAME);
    if !stamp_matches(&stamp_path, entry, &plan.upstream_sha256)? {
        eprintln!("!! vendor-upstream: stamp is missing or stale");
        stats.changed = true;
    }

    Ok(stats)
}

fn file_matches(path: &Path, expected: &[u8]) -> Result<bool, AppError> {
    match fs::read(path) {
        Ok(existing) => Ok(existing == expected),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(AppError::Io(format!("read {}: {e}", path.display()))),
    }
}

fn stamp_matches(path: &Path, entry: &VendorEntry, sha256: &str) -> Result<bool, AppError> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(AppError::Io(format!("read {}: {e}", path.display()))),
    };
    let value: toml::Value = toml::from_str(&raw)
        .map_err(|e| AppError::Input(format!("invalid {}: {e}", path.display())))?;
    Ok(value
        .get("upstream_name")
        .and_then(toml::Value::as_str)
        .is_some_and(|v| v == entry.upstream_name)
        && value
            .get("upstream_version")
            .and_then(toml::Value::as_str)
            .is_some_and(|v| v == entry.upstream_version)
        && value
            .get("upstream_sha256")
            .and_then(toml::Value::as_str)
            .is_some_and(|v| v == sha256)
        && value
            .get("vendor_tool")
            .and_then(toml::Value::as_str)
            .is_some_and(|v| v == "vendor-upstream"))
}

fn stamp_text(entry: &VendorEntry, sha256: &str, now: DateTime<Utc>) -> String {
    let mut out = String::new();
    out.push_str(&format!("upstream_name = {:?}\n", entry.upstream_name));
    out.push_str(&format!(
        "upstream_version = {:?}\n",
        entry.upstream_version
    ));
    out.push_str(&format!("upstream_sha256 = {:?}\n", sha256));
    out.push_str(&format!(
        "vendored_at = {:?}\n",
        now.to_rfc3339_opts(SecondsFormat::Secs, true)
    ));
    out.push_str("vendor_tool = \"vendor-upstream\"\n");
    if let Some(reason) = &entry.reason {
        out.push_str(&format!("reason = {:?}\n", reason));
    }
    out
}

fn list_files(root: &Path) -> Result<Vec<PathBuf>, AppError> {
    let mut out = Vec::new();
    list_files_inner(root, root, &mut out)?;
    out.sort();
    Ok(out)
}

fn list_files_inner(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), AppError> {
    for entry in
        fs::read_dir(dir).map_err(|e| AppError::Io(format!("read {}: {e}", dir.display())))?
    {
        let entry = entry.map_err(|e| AppError::Io(format!("read {}: {e}", dir.display())))?;
        let path = entry.path();
        let ty = entry
            .file_type()
            .map_err(|e| AppError::Io(format!("stat {}: {e}", path.display())))?;
        if ty.is_dir() {
            list_files_inner(root, &path, out)?;
        } else if ty.is_file() {
            let rel = path
                .strip_prefix(root)
                .map_err(|e| AppError::Io(format!("strip {}: {e}", path.display())))?;
            out.push(rel.to_owned());
        }
    }
    Ok(())
}

fn remove_empty_dirs(root: &Path) -> Result<(), AppError> {
    let mut dirs = list_dirs(root)?;
    dirs.sort_by_key(|b| std::cmp::Reverse(b.components().count()));
    for dir in dirs {
        let path = root.join(dir);
        match fs::remove_dir(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::DirectoryNotEmpty => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(AppError::Io(format!(
                    "remove empty {}: {e}",
                    path.display()
                )));
            }
        }
    }
    Ok(())
}

fn list_dirs(root: &Path) -> Result<Vec<PathBuf>, AppError> {
    let mut out = Vec::new();
    list_dirs_inner(root, root, &mut out)?;
    Ok(out)
}

fn list_dirs_inner(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), AppError> {
    for entry in
        fs::read_dir(dir).map_err(|e| AppError::Io(format!("read {}: {e}", dir.display())))?
    {
        let entry = entry.map_err(|e| AppError::Io(format!("read {}: {e}", dir.display())))?;
        let path = entry.path();
        let ty = entry
            .file_type()
            .map_err(|e| AppError::Io(format!("stat {}: {e}", path.display())))?;
        if ty.is_dir() {
            let rel = path
                .strip_prefix(root)
                .map_err(|e| AppError::Io(format!("strip {}: {e}", path.display())))?;
            out.push(rel.to_owned());
            list_dirs_inner(root, &path, out)?;
        }
    }
    Ok(())
}

fn is_protected_path(path: &Path) -> bool {
    path == Path::new("Cargo.toml")
}

fn matches_any(path: &Path, patterns: &[String]) -> bool {
    let text = path.to_string_lossy().replace('\\', "/");
    patterns.iter().any(|pattern| glob_match(pattern, &text))
}

fn glob_match(pattern: &str, text: &str) -> bool {
    let pattern_segments: Vec<&str> = pattern.split('/').collect();
    let text_segments: Vec<&str> = text.split('/').collect();
    match_segments(&pattern_segments, &text_segments)
}

fn match_segments(patterns: &[&str], texts: &[&str]) -> bool {
    if patterns.is_empty() {
        return texts.is_empty();
    }
    if patterns[0] == "**" {
        for i in 0..=texts.len() {
            if match_segments(&patterns[1..], &texts[i..]) {
                return true;
            }
        }
        return false;
    }
    let Some((text, rest_texts)) = texts.split_first() else {
        return false;
    };
    match_component(patterns[0], text) && match_segments(&patterns[1..], rest_texts)
}

fn match_component(pattern: &str, text: &str) -> bool {
    match_component_bytes(pattern.as_bytes(), text.as_bytes())
}

fn match_component_bytes(pattern: &[u8], text: &[u8]) -> bool {
    if pattern.is_empty() {
        return text.is_empty();
    }
    match pattern[0] {
        b'*' => {
            for i in 0..=text.len() {
                if match_component_bytes(&pattern[1..], &text[i..]) {
                    return true;
                }
            }
            false
        }
        b'?' => {
            if text.is_empty() {
                false
            } else {
                match_component_bytes(&pattern[1..], &text[1..])
            }
        }
        byte => {
            if text.first().copied() == Some(byte) {
                match_component_bytes(&pattern[1..], &text[1..])
            } else {
                false
            }
        }
    }
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(prefix: &str) -> Result<Self, AppError> {
        let base = std::env::temp_dir();
        for attempt in 0..100_u32 {
            let path = base.join(format!(
                "{prefix}-{}-{}-{attempt}",
                std::process::id(),
                Utc::now().timestamp_nanos_opt().unwrap_or(0)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => {
                    return Err(AppError::Io(format!(
                        "create temporary directory {}: {e}",
                        path.display()
                    )));
                }
            }
        }
        Err(AppError::Io(
            "could not create unique temporary directory".to_owned(),
        ))
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_parsing_accepts_valid_manifest() {
        let raw = r#"
[[entry]]
upstream_name = "h3-quinn"
upstream_version = "0.0.10"
target_path = "mechanics-h3-quinn"
sync = ["src/**/*.rs", "LICENSE", "README.md"]
reason = "test"
"#;
        let manifest = parse_manifest(raw).expect("valid manifest parses");
        assert_eq!(manifest.entry.len(), 1);
        assert_eq!(manifest.entry[0].upstream_name, "h3-quinn");
    }

    #[test]
    fn manifest_parsing_rejects_invalid_toml() {
        let err = parse_manifest("[[entry]\n").expect_err("invalid TOML rejects");
        assert!(matches!(err, AppError::Input(_)));
    }

    #[test]
    fn cooldown_rejects_young_release() {
        let now = DateTime::parse_from_rfc3339("2026-05-14T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let entry = IndexEntry {
            name: "demo".to_owned(),
            vers: "1.0.0".to_owned(),
            cksum: "abc".to_owned(),
            yanked: false,
            pubtime: Some("2026-05-12T00:00:00Z".to_owned()),
        };
        assert!(enforce_cooldown(&entry, now).is_err());
    }

    #[test]
    fn cooldown_accepts_old_release() {
        let now = DateTime::parse_from_rfc3339("2026-05-14T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let entry = IndexEntry {
            name: "demo".to_owned(),
            vers: "1.0.0".to_owned(),
            cksum: "abc".to_owned(),
            yanked: false,
            pubtime: Some("2026-05-10T23:59:59Z".to_owned()),
        };
        enforce_cooldown(&entry, now).expect("old release passes");
    }

    #[test]
    fn sync_globs_match_recursive_rust_files_and_root_files() {
        let patterns = vec![
            "src/**/*.rs".to_owned(),
            "LICENSE".to_owned(),
            "README.md".to_owned(),
        ];
        assert!(matches_any(Path::new("src/lib.rs"), &patterns));
        assert!(matches_any(Path::new("src/nested/mod.rs"), &patterns));
        assert!(matches_any(Path::new("LICENSE"), &patterns));
        assert!(!matches_any(Path::new("Cargo.toml"), &patterns));
        assert!(!matches_any(Path::new("src/lib.txt"), &patterns));
    }

    #[test]
    fn cargo_toml_is_not_overwritten_even_when_glob_matches() {
        let source = TempDir::new("vendor-upstream-test-source").unwrap();
        let target = TempDir::new("vendor-upstream-test-target").unwrap();
        fs::write(source.path().join("Cargo.toml"), b"upstream").unwrap();
        fs::write(source.path().join("README.md"), b"readme").unwrap();
        fs::write(target.path().join("Cargo.toml"), b"hand-maintained").unwrap();

        let entry = VendorEntry {
            upstream_name: "demo".to_owned(),
            upstream_version: "1.0.0".to_owned(),
            target_path: PathBuf::from("target"),
            sync: vec!["*".to_owned()],
            reason: None,
        };
        let plan = build_plan(&entry, source.path(), target.path(), "abc".to_owned()).unwrap();
        apply_plan(&entry, &plan).unwrap();

        let cargo = fs::read_to_string(target.path().join("Cargo.toml")).unwrap();
        let readme = fs::read_to_string(target.path().join("README.md")).unwrap();
        assert_eq!(cargo, "hand-maintained");
        assert_eq!(readme, "readme");
    }
}
