//! hf-fetch-embed-model — fetch an embedding model's ONNX +
//! tokenizer bundle from HuggingFace into a local directory,
//! pinned to a revision commit SHA and recorded in a
//! per-file-SHA256 `manifest.json`.
//!
//! **Used at deployment build time** by operators preparing
//! their `philharmonic-connector-impl-embed`-using connector-
//! service binary. The downloaded bundle becomes the
//! `include_bytes!` inputs to `Embed::new_from_bytes(...)` at
//! runtime.
//!
//! **Never invoked at runtime.** Philharmonic connector
//! services are constrained-network-friendly by design —
//! `philharmonic-connector-impl-embed` itself has no network
//! code. This xtask exists as a one-shot tool for the build
//! pipeline; after it runs once, the weights are local and the
//! connector-service binary embeds them.
//!
//! Files fetched (fastembed-compatible bundle):
//!   - ONNX model at `--onnx-path` (default `onnx/model.onnx`).
//!   - `tokenizer.json`
//!   - `tokenizer_config.json`
//!   - `config.json`
//!   - `special_tokens_map.json`
//!
//! Usage:
//!
//!   ./scripts/xtask.sh hf-fetch-embed-model -- \
//!       --model sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2 \
//!       --revision <pinned-git-sha> \
//!       --out /path/to/deployment/assets/
//!
//! Output:
//!
//!   <out>/<sanitized-model>/
//!   ├── manifest.json           — model, revision, timestamp, sha256 per file
//!   ├── model.onnx
//!   ├── tokenizer.json
//!   ├── tokenizer_config.json
//!   ├── config.json
//!   └── special_tokens_map.json
//!
//! `<sanitized-model>` is `--model` with `/` replaced by `__`.
//!
//! Idempotency: a second run against the same `<out>` with
//! matching `--model` + `--revision` re-downloads every file,
//! SHA256-compares against the existing manifest, and exits 0
//! with "up-to-date" if every byte matches. If anything
//! differs (revision upgrade, repo mutated at the same
//! revision, etc.), the bin refuses to overwrite unless
//! `--force` is passed. Safe to invoke from build scripts.
//!
//! Exit codes:
//!   0    bundle present and up-to-date (fresh write or
//!        matching re-verification).
//!   1    input error (bad flags, malformed model id, output
//!        dir misconfigured, existing content differs without
//!        `--force`).
//!   2    network / HTTP / decode failure talking to HuggingFace.
//!   3    local I/O failure writing the bundle.

use chrono::SecondsFormat;
use clap::Parser;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use xtask::http::{HttpError, fetch_bytes};

const HF_BASE: &str = "https://huggingface.co";

/// Required tokenizer files alongside the ONNX model. The set
/// is driven by what fastembed's
/// `TextEmbedding::try_new_from_user_defined(...)` expects in
/// its `TokenizerFiles` bundle.
const TOKENIZER_FILES: &[&str] = &[
    "tokenizer.json",
    "tokenizer_config.json",
    "config.json",
    "special_tokens_map.json",
];

const MANIFEST_NAME: &str = "manifest.json";

#[derive(Parser)]
#[command(
    name = "hf-fetch-embed-model",
    about = "Fetch an embedding model's ONNX + tokenizer bundle from HuggingFace into a local directory, pinned by revision and recorded in a per-file SHA256 manifest. Used at deployment build time."
)]
struct Args {
    /// HuggingFace repo id (e.g., `sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2`).
    #[arg(long)]
    model: String,
    /// Git revision to pin — a commit SHA is the reproducible
    /// choice; a branch name like `main` works but drifts
    /// every time the branch moves.
    #[arg(long)]
    revision: String,
    /// Output directory. A subdirectory named after the
    /// sanitized model id will be created inside it.
    #[arg(long)]
    out: PathBuf,
    /// Path to the ONNX file inside the HF repo. Varies by
    /// repo — some use `onnx/model.onnx`, some have it at
    /// the top level, some ship a quantized variant like
    /// `onnx/model_quantized.onnx`.
    #[arg(long, default_value = "onnx/model.onnx")]
    onnx_path: String,
    /// Overwrite the bundle if its existing content differs
    /// from what would be downloaded this run.
    #[arg(long)]
    force: bool,
}

#[derive(Serialize, Deserialize)]
struct Manifest {
    model: String,
    revision: String,
    onnx_path: String,
    fetched_at: String,
    files: BTreeMap<String, String>,
}

fn main() -> ExitCode {
    let args = Args::parse();

    let sanitized = match sanitize_model_id(&args.model) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("!!! hf-fetch-embed-model: {e}");
            return ExitCode::from(1);
        }
    };

    let target_dir = args.out.join(&sanitized);

    // Gather the set of files to fetch. The ONNX is stored
    // under its basename (no nested directory in the output),
    // matching what the deployment's build pipeline will
    // `include_bytes!` by flat filename.
    let onnx_basename = match Path::new(&args.onnx_path)
        .file_name()
        .and_then(|os| os.to_str())
        .map(|s| s.to_owned())
    {
        Some(name) if !name.is_empty() => name,
        _ => {
            eprintln!(
                "!!! hf-fetch-embed-model: --onnx-path '{}' has no resolvable basename",
                args.onnx_path
            );
            return ExitCode::from(1);
        }
    };

    // URL / local-name pairs in deterministic order.
    let mut plan: Vec<(String, String)> = Vec::with_capacity(1 + TOKENIZER_FILES.len());
    plan.push((args.onnx_path.clone(), onnx_basename));
    for tok in TOKENIZER_FILES {
        plan.push(((*tok).to_owned(), (*tok).to_owned()));
    }

    // Fetch every file into memory and hash each.
    let mut fetched: Vec<(String, Vec<u8>, String)> = Vec::with_capacity(plan.len());
    for (remote_path, local_name) in &plan {
        let url = format!(
            "{HF_BASE}/{}/resolve/{}/{}",
            args.model, args.revision, remote_path
        );
        eprintln!("hf-fetch-embed-model: GET {url}");
        let bytes = match fetch_bytes(&url) {
            Ok(b) => b,
            Err(HttpError::StatusCode { code }) => {
                eprintln!(
                    "!!! hf-fetch-embed-model: HTTP {code} fetching {remote_path} from {}",
                    args.model
                );
                return ExitCode::from(2);
            }
            Err(e) => {
                eprintln!("!!! hf-fetch-embed-model: transport error fetching {remote_path}: {e}");
                return ExitCode::from(2);
            }
        };
        let digest = sha256_hex(&bytes);
        fetched.push((local_name.clone(), bytes, digest));
    }

    // Check existing manifest (if any) to decide overwrite vs. skip.
    let manifest_path = target_dir.join(MANIFEST_NAME);
    if manifest_path.is_file() {
        let existing: Manifest = match std::fs::read_to_string(&manifest_path)
            .map_err(|e| format!("read {}: {e}", manifest_path.display()))
            .and_then(|s| {
                serde_json::from_str(&s)
                    .map_err(|e| format!("parse {}: {e}", manifest_path.display()))
            }) {
            Ok(m) => m,
            Err(e) => {
                eprintln!(
                    "!!! hf-fetch-embed-model: cannot read existing manifest; use --force to overwrite: {e}"
                );
                return ExitCode::from(1);
            }
        };

        let up_to_date = existing.model == args.model
            && existing.revision == args.revision
            && existing.onnx_path == args.onnx_path
            && fetched.iter().all(|(name, _bytes, digest)| {
                existing.files.get(name).is_some_and(|have| have == digest)
            })
            && fetched.len() == existing.files.len();

        if up_to_date {
            eprintln!(
                "hf-fetch-embed-model: bundle at {} is up-to-date (model={}, revision={})",
                target_dir.display(),
                args.model,
                args.revision
            );
            return ExitCode::SUCCESS;
        }

        if !args.force {
            eprintln!(
                "!!! hf-fetch-embed-model: existing bundle at {} differs from requested \
                 model/revision/content; re-run with --force to overwrite",
                target_dir.display()
            );
            return ExitCode::from(1);
        }
        // --force: fall through; we'll overwrite files below.
    }

    // Write (or overwrite) the bundle.
    if let Err(e) = std::fs::create_dir_all(&target_dir) {
        eprintln!(
            "!!! hf-fetch-embed-model: cannot create {}: {e}",
            target_dir.display()
        );
        return ExitCode::from(3);
    }

    for (local_name, bytes, _digest) in &fetched {
        let dest = target_dir.join(local_name);
        if let Err(e) = std::fs::write(&dest, bytes) {
            eprintln!(
                "!!! hf-fetch-embed-model: cannot write {}: {e}",
                dest.display()
            );
            return ExitCode::from(3);
        }
    }

    let manifest = Manifest {
        model: args.model.clone(),
        revision: args.revision.clone(),
        onnx_path: args.onnx_path.clone(),
        fetched_at: chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        files: fetched
            .iter()
            .map(|(name, _, digest)| (name.clone(), digest.clone()))
            .collect(),
    };

    let manifest_json = match serde_json::to_string_pretty(&manifest) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("!!! hf-fetch-embed-model: cannot encode manifest: {e}");
            return ExitCode::from(3);
        }
    };

    let manifest_path = target_dir.join(MANIFEST_NAME);
    if let Err(e) = std::fs::write(&manifest_path, format!("{manifest_json}\n")) {
        eprintln!(
            "!!! hf-fetch-embed-model: cannot write {}: {e}",
            manifest_path.display()
        );
        return ExitCode::from(3);
    }

    eprintln!(
        "hf-fetch-embed-model: wrote {} files + manifest.json at {}",
        fetched.len(),
        target_dir.display()
    );
    ExitCode::SUCCESS
}

fn sanitize_model_id(model: &str) -> Result<String, String> {
    if model.is_empty() {
        return Err("--model is empty".to_owned());
    }
    for ch in model.chars() {
        let ok = ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '/');
        if !ok {
            return Err(format!(
                "--model contains unsupported character {ch:?}; HF repo ids are ASCII alphanumerics with `-` `_` `.` `/`"
            ));
        }
    }
    if model.starts_with('/') || model.ends_with('/') {
        return Err("--model must not start or end with '/'".to_owned());
    }
    Ok(model.replace('/', "__"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let out = hasher.finalize();
    let mut hex = String::with_capacity(out.len() * 2);
    for byte in out {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_model_id_replaces_slashes() {
        assert_eq!(
            sanitize_model_id("sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2")
                .unwrap(),
            "sentence-transformers__paraphrase-multilingual-MiniLM-L12-v2"
        );
    }

    #[test]
    fn sanitize_model_id_allows_bare_name() {
        assert_eq!(
            sanitize_model_id("bge-small-en-v1.5").unwrap(),
            "bge-small-en-v1.5"
        );
    }

    #[test]
    fn sanitize_model_id_rejects_empty() {
        assert!(sanitize_model_id("").is_err());
    }

    #[test]
    fn sanitize_model_id_rejects_leading_or_trailing_slash() {
        assert!(sanitize_model_id("/foo/bar").is_err());
        assert!(sanitize_model_id("foo/bar/").is_err());
    }

    #[test]
    fn sanitize_model_id_rejects_unsupported_chars() {
        assert!(sanitize_model_id("foo bar").is_err());
        assert!(sanitize_model_id("foo:bar").is_err());
        assert!(sanitize_model_id("foo@bar").is_err());
    }

    #[test]
    fn sha256_hex_matches_known_vectors() {
        // NIST test vector: SHA-256("") =
        //   e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        // SHA-256("abc") =
        //   ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn manifest_roundtrip() {
        let mut files = BTreeMap::new();
        files.insert("model.onnx".to_owned(), "aa".repeat(32));
        files.insert("tokenizer.json".to_owned(), "bb".repeat(32));
        let m = Manifest {
            model: "ns/name".to_owned(),
            revision: "cc".repeat(20),
            onnx_path: "onnx/model.onnx".to_owned(),
            fetched_at: "2026-04-24T11:12:13Z".to_owned(),
            files,
        };
        let json = serde_json::to_string(&m).unwrap();
        let back: Manifest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.model, m.model);
        assert_eq!(back.revision, m.revision);
        assert_eq!(back.onnx_path, m.onnx_path);
        assert_eq!(back.fetched_at, m.fetched_at);
        assert_eq!(back.files, m.files);
    }
}
