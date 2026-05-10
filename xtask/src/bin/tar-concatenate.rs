//! tar-concatenate — concatenate multiple tar archives into one,
//! optionally compressing the output with gzip or zstd.
//!
//! Usage:
//!   ./scripts/xtask.sh tar-concatenate -- -o out.tar a.tar b.tar
//!   ./scripts/xtask.sh tar-concatenate -- --gzip -o out.tar.gz a.tar b.tar
//!   ./scripts/xtask.sh tar-concatenate -- --zstd -o out.tar.zst a.tar b.tar
//!
//! Inputs are read in argument order; their entries are appended
//! to the output in that order. The output path is overwritten if
//! it exists; this bin never reads from it.

use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;

use clap::Parser;

#[derive(Clone, Copy)]
enum Compression {
    None,
    Gzip,
    Zstd,
}

#[derive(Parser)]
#[command(
    name = "tar-concatenate",
    about = "Concatenate multiple tar archives into a single output \
             archive, optionally compressing the output with gzip \
             or zstd. Overwrites the output path; never reads it."
)]
struct Args {
    /// Output file path (overwritten if it exists).
    #[arg(short, long)]
    output: PathBuf,

    /// Compress output with gzip.
    #[arg(long, group = "comp")]
    gzip: bool,

    /// Compress output with zstd.
    #[arg(long, group = "comp")]
    zstd: bool,

    /// Input tar archives, in the order their entries should
    /// appear in the output.
    #[arg(required = true)]
    tar_files: Vec<PathBuf>,
}

fn run(args: Args) -> anyhow::Result<()> {
    let compression = if args.zstd {
        Compression::Zstd
    } else if args.gzip {
        Compression::Gzip
    } else {
        Compression::None
    };

    let out_file = File::create(&args.output)?;
    let buf = BufWriter::new(out_file);

    match compression {
        Compression::None => {
            let mut archive = tar::Builder::new(buf);
            concatenate(&mut archive, &args.tar_files)?;
            let mut inner = archive.into_inner()?;
            inner.flush()?;
        }
        Compression::Gzip => {
            let encoder = flate2::write::GzEncoder::new(buf, flate2::Compression::default());
            let mut archive = tar::Builder::new(encoder);
            concatenate(&mut archive, &args.tar_files)?;
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
            concatenate(&mut archive, &args.tar_files)?;
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

fn concatenate<W: Write>(
    archive: &mut tar::Builder<W>,
    tar_files: &[PathBuf],
) -> anyhow::Result<()> {
    for path in tar_files {
        let f = File::open(path).map_err(|e| anyhow::anyhow!("open {}: {e}", path.display()))?;
        let mut input = tar::Archive::new(BufReader::new(f));
        for entry in input.entries()? {
            let mut entry = entry?;
            let mut header = entry.header().clone();
            let entry_path = entry.path()?.into_owned();
            archive.append_data(&mut header, &entry_path, &mut entry)?;
        }
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
