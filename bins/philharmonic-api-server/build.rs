use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

const GIT_COMMIT_SHA_ENV: &str = "PHILHARMONIC_API_GIT_COMMIT_SHA";

fn main() {
    let manifest_dir = match env::var_os("CARGO_MANIFEST_DIR") {
        Some(value) => PathBuf::from(value),
        None => return,
    };
    let workspace_dir = workspace_dir(&manifest_dir);

    emit_git_rerun_hints(&workspace_dir);
    if let Some(sha) = git_commit_sha(&workspace_dir) {
        println!("cargo:rustc-env={GIT_COMMIT_SHA_ENV}={sha}");
    }
}

fn workspace_dir(manifest_dir: &Path) -> PathBuf {
    manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .map(|path| path.to_path_buf())
        .unwrap_or_else(|| manifest_dir.to_path_buf())
}

fn emit_git_rerun_hints(workspace_dir: &Path) {
    if let Some(head_path) = git_path(workspace_dir, "HEAD") {
        println!("cargo:rerun-if-changed={}", head_path.display());
        if let Ok(head) = fs::read_to_string(&head_path)
            && let Some(ref_name) = head.strip_prefix("ref: ")
            && let Some(ref_path) = git_path(workspace_dir, ref_name.trim())
        {
            println!("cargo:rerun-if-changed={}", ref_path.display());
        }
    }

    if let Some(packed_refs_path) = git_path(workspace_dir, "packed-refs") {
        println!("cargo:rerun-if-changed={}", packed_refs_path.display());
    }
}

fn git_path(workspace_dir: &Path, logical_path: &str) -> Option<PathBuf> {
    let output = Command::new("git")
        .arg("-C")
        .arg(workspace_dir)
        .arg("rev-parse")
        .arg("--git-path")
        .arg(logical_path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let path = String::from_utf8(output.stdout).ok()?;
    let path = path.trim();
    if path.is_empty() {
        return None;
    }

    let path = PathBuf::from(path);
    if path.is_absolute() {
        Some(path)
    } else {
        Some(workspace_dir.join(path))
    }
}

fn git_commit_sha(workspace_dir: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(workspace_dir)
        .arg("rev-parse")
        .arg("--verify")
        .arg("HEAD")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let sha = String::from_utf8(output.stdout).ok()?;
    let sha = sha.trim();
    if (sha.len() == 40 || sha.len() == 64) && sha.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(sha.to_string())
    } else {
        None
    }
}
