//! tar-archive — fast tar+gzip/zstd archiver for release artifacts.
//!
//! Usage:
//!   ./scripts/xtask.sh tar-archive -- -o out.tar.gz --gzip file1 file2 ...
//!   ./scripts/xtask.sh tar-archive -- -o out.tar.zst --zstd file1 file2 ...
//!
//! Files are stored in the archive under their basename (no
//! directory prefix). Duplicate basenames are an error.

use std::collections::HashSet;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use clap::Parser;

#[derive(Clone, Copy)]
enum Compression {
    Gzip,
    Zstd,
}

#[derive(Parser)]
#[command(
    name = "tar-archive",
    about = "Create a compressed tar archive from the listed files. \
             Replaces shell `tar | gzip` with a single Rust binary \
             for speed (parallel zstd) and portability."
)]
struct Args {
    /// Output file path.
    #[arg(short, long)]
    output: PathBuf,

    /// Use gzip compression (default if neither flag given).
    #[arg(long, group = "comp")]
    gzip: bool,

    /// Use zstd compression.
    #[arg(long, group = "comp")]
    zstd: bool,

    /// Files to include in the archive (stored under their basename).
    #[arg(required = true)]
    files: Vec<PathBuf>,
}

fn run(args: Args) -> anyhow::Result<()> {
    let compression = if args.zstd {
        Compression::Zstd
    } else {
        Compression::Gzip
    };

    let mut seen = HashSet::new();
    for path in &args.files {
        let name = path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("path has no filename: {}", path.display()))?
            .to_string_lossy()
            .into_owned();
        if !seen.insert(name.clone()) {
            anyhow::bail!("duplicate basename in archive: {name}");
        }
    }

    let out_file = File::create(&args.output)?;
    let buf = BufWriter::new(out_file);

    match compression {
        Compression::Gzip => {
            let encoder = flate2::write::GzEncoder::new(buf, flate2::Compression::default());
            let mut archive = tar::Builder::new(encoder);
            append_files(&mut archive, &args.files)?;
            archive.into_inner()?.finish()?;
        }
        Compression::Zstd => {
            let mut raw = zstd::stream::raw::Encoder::new(0)?;
            let workers = std::thread::available_parallelism()
                .map(|n| n.get() as u32)
                .unwrap_or(1);
            raw.set_parameter(zstd::stream::raw::CParameter::NbWorkers(workers))?;
            let encoder = zstd::Encoder::with_encoder(buf, raw);
            let mut archive = tar::Builder::new(encoder);
            append_files(&mut archive, &args.files)?;
            archive.into_inner()?.finish()?;
        }
    }

    let meta = std::fs::metadata(&args.output)?;
    eprintln!(
        "{} ({}) written",
        args.output.display(),
        human_size(meta.len()),
    );
    Ok(())
}

fn append_files<W: Write>(archive: &mut tar::Builder<W>, files: &[PathBuf]) -> anyhow::Result<()> {
    for path in files {
        let name = path.file_name().unwrap().to_string_lossy();
        archive.append_path_with_name(path, name.as_ref())?;
    }
    Ok(())
}

fn human_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

fn main() {
    let args = Args::parse();
    if let Err(e) = run(args) {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}
