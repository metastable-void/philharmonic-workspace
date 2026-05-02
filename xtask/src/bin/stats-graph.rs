//! stats-graph — render an SVG line graph of workspace size over
//! time from `stats-log.sh`-formatted stdin.
//!
//! Reads `./scripts/stats-log.sh`-style lines on stdin (one commit
//! per line, with the `Code-stats:` trailer parsed out), and
//! writes an SVG line chart to stdout with three series:
//!
//! - **total** — total lines (code + docs + blanks).
//! - **code** — code lines.
//! - **docs** — comments + blank lines (the `D` field).
//!
//! X axis is the commit's Unix timestamp (seconds), parsed from
//! the ISO date in the input. Y axis is line count. Commits
//! without a `Code-stats:` trailer (older than the trailer's
//! adoption) are skipped silently. The input order doesn't
//! matter — samples are sorted chronologically before plotting.
//!
//! Backed by the `poloto` crate (pure-Rust SVG plotter, no
//! browser / canvas dependency); CSS theming via the bundled
//! light theme so the SVG renders correctly when embedded in
//! Markdown viewers and on GitHub.
//!
//! **Strict parsing.** A commit is plotted only when its line
//! has all four fields parseable (ISO timestamp, total `L`,
//! code `C`, docs `D`). Commits with `-` in the stats slot
//! (older than the `Code-stats:` trailer's adoption) and
//! commits where any field fails to parse are excluded — never
//! interpolated, defaulted, or carried forward from neighbours.
//! The stderr summary on completion reports the skip count so a
//! sparse chart isn't silent about its sparseness.
//!
//! Usage:
//!   ./scripts/stats-log.sh --no-color | ./scripts/xtask.sh stats-graph
//!
//! Typically wrapped by `./scripts/update-stats-graph.sh` which
//! pipes the log into this bin and redirects the result to
//! `./docs/stats.svg`.

use std::io::{self, BufRead, Write};

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, TimeZone};
use chrono_tz::Asia::Tokyo;
use poloto::ticks::TickDistGen;

#[derive(Debug, Clone, Copy)]
struct Sample {
    t_unix: f64,
    total: f64,
    code: f64,
    docs: f64,
}

/// Format a line count with an SI suffix — `850`, `1.5K`,
/// `137K`, `2.3M`, `1.5G`. Used for Y-axis tick labels so the
/// chart shows recognisable line-count magnitudes instead of
/// poloto's default scientific notation (`1e5`).
fn si_format(v: f64) -> String {
    let abs = v.abs();
    let (scale, suffix) = if abs >= 1.0e9 {
        (1.0e9, "G")
    } else if abs >= 1.0e6 {
        (1.0e6, "M")
    } else if abs >= 1.0e3 {
        (1.0e3, "K")
    } else {
        return format!("{v:.0}");
    };
    let scaled = v / scale;
    if (scaled - scaled.round()).abs() < 1e-9 {
        format!("{scaled:.0}{suffix}")
    } else {
        format!("{scaled:.1}{suffix}")
    }
}

/// Format a Unix timestamp (f64 seconds) as an ISO date+time in
/// JST (Asia/Tokyo, UTC+09:00), e.g. `2026-05-02 15:32`. Used
/// for X-axis tick labels so the chart shows recognisable dates
/// instead of poloto's default `1.77765e9 + j*1e7` notation. JST
/// is the project's authoritative timezone for human-facing
/// time displays — see CONTRIBUTING.md §"JST is this workspace's
/// authoritative timezone". The minute component is kept
/// (rather than date-only) because poloto picks 1-2-5 tick
/// steps at whatever scale fits the data; multi-day spans
/// typically land sub-day ticks that a date-only format would
/// render as duplicates. Out-of-range timestamps fall back to
/// integer seconds.
fn iso_date_format(unix_seconds: f64) -> String {
    let secs = unix_seconds.round() as i64;
    let utc = match DateTime::<chrono::Utc>::from_timestamp(secs, 0) {
        Some(t) => t,
        None => return format!("{secs}"),
    };
    Tokyo
        .from_utc_datetime(&utc.naive_utc())
        .format("%Y-%m-%d %H:%M")
        .to_string()
}

/// Rewrite poloto's single `<text class="poloto_text poloto_ticks
/// poloto_x"><tspan>…</tspan>…</text>` block into a `<g
/// class="…">` containing one `<text>` per tick with a
/// `transform="rotate(-90 x y)"` attribute pivoting around the
/// tick's anchor point. SVG `transform` on a text element is the
/// portable way to render rotated text — every conformant SVG
/// viewer (browsers, GitHub's Markdown renderer, mdBook, image
/// tools) handles it. CSS `transform` on `<tspan>` doesn't have
/// the same compatibility. Returns the original string unchanged
/// if the X-tick block isn't found (defensive — poloto's output
/// shape may evolve in a future major).
fn rotate_x_ticks(svg: String) -> String {
    let marker = "<text class=\"poloto_text poloto_ticks poloto_x\">";
    let Some(block_start) = svg.find(marker) else {
        return svg;
    };
    let close_tag = "</text>";
    let Some(close_rel) = svg[block_start..].find(close_tag) else {
        return svg;
    };
    let block_end = block_start + close_rel + close_tag.len();
    let block = &svg[block_start + marker.len()..block_start + close_rel];

    let mut rotated = String::new();
    rotated.push_str("<g class=\"poloto_text poloto_ticks poloto_x\">");
    let mut cursor = block;
    while let Some(open) = cursor.find("<tspan ") {
        let attrs_start = open + "<tspan ".len();
        let Some(gt) = cursor[attrs_start..].find('>') else {
            break;
        };
        let attrs = &cursor[attrs_start..attrs_start + gt];
        let content_start = attrs_start + gt + 1;
        let Some(end_rel) = cursor[content_start..].find("</tspan>") else {
            break;
        };
        let content = &cursor[content_start..content_start + end_rel];
        let next = content_start + end_rel + "</tspan>".len();

        if let (Some(x), Some(y)) = (extract_attr(attrs, "x"), extract_attr(attrs, "y")) {
            // International convention for vertical Western /
            // Latin-script date labels is BOTTOM-TO-TOP — the
            // reader tilts their head LEFT (CCW) to read the
            // string left-to-right. matplotlib's rotation=90
            // default, D3.js examples, ggplot, IEEE figure-style
            // guides and Tufte all use this orientation.
            //
            // SVG geometry to achieve it: text-anchor="end" +
            // transform="rotate(-90 x y)". Original text "ends
            // at (x, y), extending LEFT". rotate(-90 x y) is
            // CCW around the anchor; LEFT direction maps to
            // DOWN (SVG +y is down). So the END of the string
            // sits at the tick (top, y), and the START extends
            // DOWN to (x, y + W). Reading bottom-to-top with a
            // leftward head-tilt yields normal left-to-right
            // text flow.
            rotated.push_str("\n\t\t<text x=\"");
            rotated.push_str(x);
            rotated.push_str("\" y=\"");
            rotated.push_str(y);
            rotated.push_str("\" text-anchor=\"end\" transform=\"rotate(-90 ");
            rotated.push_str(x);
            rotated.push(' ');
            rotated.push_str(y);
            rotated.push_str(")\">");
            rotated.push_str(content);
            rotated.push_str("</text>");
        }
        cursor = &cursor[next..];
    }
    rotated.push_str("\n\t</g>");

    let mut out = String::with_capacity(svg.len() + rotated.len());
    out.push_str(&svg[..block_start]);
    out.push_str(&rotated);
    out.push_str(&svg[block_end..]);
    out
}

/// Extract the value of `name="…"` from an attribute string. Used
/// only by `rotate_x_ticks` against poloto's well-formed output;
/// returns None if the attribute is missing or malformed.
fn extract_attr<'a>(attrs: &'a str, name: &str) -> Option<&'a str> {
    let needle_eq = format!("{name}=\"");
    let start = attrs.find(&needle_eq)? + needle_eq.len();
    let end_rel = attrs[start..].find('"')?;
    Some(&attrs[start..start + end_rel])
}

/// Move poloto's X-axis name (`<text class="… poloto_name
/// poloto_x" … y="…">date (JST)</text>`) down so it doesn't sit on
/// top of the rotated tick labels. Poloto positions the axis name
/// just below the tick row assuming horizontal tick labels; with
/// our -90° rotation the labels extend ~140 px down from the tick
/// line, so the axis name needs to move past them. Rewrites the
/// `y="…"` attribute to a fixed value chosen to clear the rotated
/// labels in our 800x680 canvas.
fn move_x_axis_name(svg: String, new_y: &str) -> String {
    let marker = "<text class=\"poloto_text poloto_name poloto_x\"";
    let Some(start) = svg.find(marker) else {
        return svg;
    };
    // Locate the `y="…"` attribute within this tag.
    let Some(close_rel) = svg[start..].find('>') else {
        return svg;
    };
    let tag_end = start + close_rel;
    let tag = &svg[start..tag_end];
    let needle = "y=\"";
    let Some(y_start_rel) = tag.find(needle) else {
        return svg;
    };
    let y_start = start + y_start_rel + needle.len();
    let Some(y_end_rel) = svg[y_start..].find('"') else {
        return svg;
    };
    let y_end = y_start + y_end_rel;

    let mut out = String::with_capacity(svg.len() + 8);
    out.push_str(&svg[..y_start]);
    out.push_str(new_y);
    out.push_str(&svg[y_end..]);
    out
}

/// Parse one stats-log.sh line; returns None for lines without
/// a parseable `Code-stats:` payload.
///
/// Expected line shape:
///   `<hash> <ISO> <author>... | <F>F <L>L (<C>C <D>D) | Δ ...`
///
/// Lines emitted before the `Code-stats:` trailer existed render
/// the stats segment as `-`; those produce None.
fn parse_line(line: &str) -> Option<Sample> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let parts: Vec<&str> = line.splitn(3, " | ").collect();
    if parts.len() < 2 {
        return None;
    }
    let stats = parts[1].trim();
    if stats == "-" {
        return None;
    }

    let mut header_iter = parts[0].split_whitespace();
    let _hash = header_iter.next()?;
    let iso = header_iter.next()?;

    // stats: "<F>F <L>L (<C>C <D>D)"
    // Replace parens with whitespace so split_whitespace gives tokens.
    let scrubbed = stats.replace(['(', ')'], " ");
    let mut total: Option<u64> = None;
    let mut code: Option<u64> = None;
    let mut docs: Option<u64> = None;
    for tok in scrubbed.split_whitespace() {
        if let Some(num) = tok.strip_suffix('L') {
            total = num.parse().ok();
        } else if let Some(num) = tok.strip_suffix('C') {
            code = num.parse().ok();
        } else if let Some(num) = tok.strip_suffix('D') {
            docs = num.parse().ok();
        }
    }

    let total = total?;
    let code = code?;
    let docs = docs?;
    let t = DateTime::parse_from_rfc3339(iso).ok()?;

    Some(Sample {
        t_unix: t.timestamp() as f64,
        total: total as f64,
        code: code as f64,
        docs: docs as f64,
    })
}

fn main() -> Result<()> {
    let stdin = io::stdin();
    let mut samples: Vec<Sample> = Vec::new();
    let mut skipped: usize = 0;
    for line in stdin.lock().lines() {
        let line = line.context("read stdin")?;
        if line.trim().is_empty() {
            continue;
        }
        match parse_line(&line) {
            Some(s) => samples.push(s),
            None => skipped += 1,
        }
    }
    if samples.is_empty() {
        return Err(anyhow!(
            "no parseable Code-stats lines on stdin (every line lacked the trailer or was malformed)"
        ));
    }
    // Surface the skip count so a sparse chart isn't silent about
    // its sparseness. We deliberately do NOT interpolate or
    // forward-fill across skipped commits — missing data is
    // missing.
    if skipped > 0 {
        eprintln!(
            "stats-graph: skipped {skipped} commits without a parseable Code-stats trailer ({} plotted)",
            samples.len()
        );
    }

    // stats-log.sh emits newest-first; plot wants oldest-first.
    samples.sort_by(|a, b| {
        a.t_unix
            .partial_cmp(&b.t_unix)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let total: Vec<(f64, f64)> = samples.iter().map(|s| (s.t_unix, s.total)).collect();
    let code: Vec<(f64, f64)> = samples.iter().map(|s| (s.t_unix, s.code)).collect();
    let docs: Vec<(f64, f64)> = samples.iter().map(|s| (s.t_unix, s.docs)).collect();

    // markers((), [0.0]) keeps the Y axis grounded at zero — by
    // default poloto fits the Y range tight to the data, which
    // makes growth charts read as percentages rather than absolute
    // size. The iterator types need to be `f64` to match the L =
    // (f64, f64) point type used by the line plots above.
    let no_x: [f64; 0] = [];
    let plots = poloto::plots!(
        poloto::build::plot("total").line(total),
        poloto::build::plot("code").line(code),
        poloto::build::plot("docs").line(docs),
        poloto::build::markers(no_x, [0.0_f64])
    );

    let stage3 = poloto::frame_build()
        .data(plots)
        .map_xticks(|_default| {
            // Render X tick values as ISO date+time (UTC) instead
            // of raw Unix-seconds floats with poloto's default
            // offset/`where j=...` notation. Labels are rotated
            // -90° via a custom CSS rule below, so we keep
            // poloto's default ideal tick count — vertical labels
            // don't fight each other for horizontal space.
            // with_where_fmt(|| "") suppresses the trailing
            // offset annotation since ISO timestamps are absolute
            // and self-explanatory.
            poloto::ticks::from_closure(|data, canvas, req| {
                let dist = poloto::num::float::FloatTickFmt.generate(data, canvas, req);
                dist.with_tick_fmt(|&v| iso_date_format(v))
                    .with_where_fmt(|| "")
            })
        })
        .map_yticks(|_default| {
            // Swap default float scientific notation (`1e5`) on
            // the Y axis for SI suffixes (`100K`, `1.5M`).
            poloto::ticks::from_closure(|data, canvas, req| {
                let dist = poloto::num::float::FloatTickFmt.generate(data, canvas, req);
                dist.with_tick_fmt(|&v| si_format(v))
            })
        })
        .build_and_label(("Workspace lines over time", "date (JST)", "lines"));

    // Default header is 800x500. Vertical X-axis labels (rotated
    // -90° to read top-to-bottom from each tick) sit at y≈430 and
    // extend ~140px downward, so the canvas needs extra height to
    // contain them. Bump dim+viewbox to 800x680 (180px of extra
    // bottom padding); the chart area is positioned by poloto's
    // internal padding constants and stays put — the extra height
    // becomes label gutter, not stretched chart.
    let stage4 = stage3.append_to(
        poloto::header()
            .with_dim([800.0, 680.0])
            .with_viewbox([800.0, 680.0])
            .light_theme(),
    );

    let svg = stage4.render_string().context("render SVG")?;
    // Rewrite the X-tick text block so each label is a separate
    // `<text>` rotated -90° around its anchor — portable across
    // SVG viewers in a way CSS-on-tspan isn't. See rotate_x_ticks
    // doc comment.
    let svg = rotate_x_ticks(svg);
    // Push the X-axis name "date (JST)" down past the rotated
    // tick labels so it doesn't overlap them. Tick text sits at
    // y=430 and extends ~140 px down; place the name at y=640
    // (still inside the 800x680 canvas, with margin).
    let svg = move_x_axis_name(svg, "640");
    let mut out = io::stdout().lock();
    out.write_all(svg.as_bytes())?;
    out.write_all(b"\n")?;
    Ok(())
}
