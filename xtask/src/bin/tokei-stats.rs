//! Per-language file-size distribution statistics using tokei.
//!
//! For each language detected in the current directory, prints:
//!   N (file count), min, Q1, Q2 (median), Q3, max, avg, stddev
//! where line counts are total lines (code + comments + blanks).
//!
//! Tokei's default exclusions apply (target/, node_modules/, etc.).
//!
//! Run: `./scripts/xtask.sh tokei-stats`

use std::process::ExitCode;

fn main() -> ExitCode {
    let mut langs = tokei::Languages::new();
    let config = tokei::Config::default();
    langs.get_statistics(&["."], &[], &config);

    let mut rows: Vec<(String, Vec<usize>)> = Vec::new();
    for (lang_type, lang) in &langs {
        let mut lines: Vec<usize> = lang
            .reports
            .iter()
            .map(|r| r.stats.lines())
            .collect();
        if lines.is_empty() {
            continue;
        }
        lines.sort_unstable();
        rows.push((lang_type.to_string(), lines));
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0));

    if rows.is_empty() {
        eprintln!("No files found.");
        return ExitCode::from(1);
    }

    println!(
        "{:<20} {:>5} {:>6} {:>6} {:>6} {:>6} {:>6} {:>8} {:>8}",
        "Language", "N", "min", "Q1", "Q2", "Q3", "max", "avg", "stddev"
    );
    println!("{}", "-".repeat(82));

    for (name, lines) in &rows {
        let n = lines.len();
        let min = lines[0];
        let max = lines[n - 1];
        let q1 = percentile(lines, 25);
        let q2 = percentile(lines, 50);
        let q3 = percentile(lines, 75);
        let avg = mean(lines);
        let sd = stddev(lines, avg);

        println!(
            "{:<20} {:>5} {:>6} {:>6} {:>6} {:>6} {:>6} {:>8.1} {:>8.1}",
            name, n, min, q1, q2, q3, max, avg, sd,
        );
    }

    ExitCode::SUCCESS
}

fn percentile(sorted: &[usize], p: usize) -> usize {
    let n = sorted.len();
    if n == 1 {
        return sorted[0];
    }
    let rank = (p as f64 / 100.0) * (n - 1) as f64;
    let lo = rank as usize;
    let hi = if lo + 1 < n { lo + 1 } else { lo };
    let frac = rank - lo as f64;
    (sorted[lo] as f64 + frac * (sorted[hi] as f64 - sorted[lo] as f64)).round() as usize
}

fn mean(vals: &[usize]) -> f64 {
    vals.iter().sum::<usize>() as f64 / vals.len() as f64
}

fn stddev(vals: &[usize], avg: f64) -> f64 {
    if vals.len() < 2 {
        return 0.0;
    }
    let variance = vals.iter().map(|&v| {
        let d = v as f64 - avg;
        d * d
    }).sum::<f64>() / (vals.len() - 1) as f64;
    variance.sqrt()
}
