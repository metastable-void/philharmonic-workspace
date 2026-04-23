//! Prints the Japanese work-calendar context that AI agents (Claude Code,
//! Codex, ...) need when reasoning about deadlines in JST.
//!
//! Output shape:
//!   - Header row of weekday names
//!   - 5 rows, one per week (current week + next 4), with day-of-month
//!     numbers and markers for weekend / holiday / today
//!   - A holiday legend listing every 祝日 in the window
//!   - Current JST wall-clock timestamp (24-hour) at the bottom
//!
//! Run: `./scripts/xtask.sh calendar-jp` — the workspace-canonical
//! invocation (preferred over raw `cargo xtask calendar-jp` so the
//! wrapper's `CARGO_TARGET_DIR=target-xtask` isolation applies).

use chrono::{Datelike, Duration, NaiveDate, Weekday};
use chrono_tz::Asia::Tokyo;
use std::collections::BTreeMap;

fn main() -> anyhow::Result<()> {
    // 1. "Now" in JST wall-clock — never use the host TZ here.
    let now_jst = chrono::Utc::now().with_timezone(&Tokyo);
    let today: NaiveDate = now_jst.date_naive();

    // 2. Window: Monday of the current ISO week + 5 weeks (35 days).
    let week_start = monday_of(today);
    let range_end = week_start + Duration::weeks(5) - Duration::days(1);

    // 3. Holidays in [week_start, range_end], inclusive.
    //    yasumi::between returns Vec<(NaiveDate, String)> sorted by date.
    let holidays: BTreeMap<NaiveDate, String> =
        yasumi::between(week_start, range_end).into_iter().collect();

    // 4. Render.
    print_grid(today, week_start, &holidays);
    print_legend(&holidays);
    println!();
    println!("JST now: {}", now_jst.format("%Y-%m-%d (%a) %H:%M:%S"));

    Ok(())
}

fn monday_of(d: NaiveDate) -> NaiveDate {
    d - Duration::days(d.weekday().num_days_from_monday() as i64)
}

fn print_grid(today: NaiveDate, start: NaiveDate, holidays: &BTreeMap<NaiveDate, String>) {
    // Column widths: 5 chars per day cell (" NN? " or "[NN]").
    println!("Week              Mon   Tue   Wed   Thu   Fri   Sat   Sun");
    for w in 0..5 {
        let monday = start + Duration::weeks(w);
        let sunday = monday + Duration::days(6);
        let label = format!("{}–{}", monday.format("%m/%d"), sunday.format("%m/%d"));

        let mut line = format!("{:<16}", label);
        for d in 0..7 {
            let day = monday + Duration::days(d);
            line.push_str(&cell(day, today, holidays));
        }
        println!("{line}");
    }
}

/// 6-char fixed-width cell. Markers (from strongest to weakest):
///   ` [NN] ` today
///   `  NN* ` public holiday
///   `  NN· ` Saturday / Sunday (not an official 祝日, but non-working)
///   `  NN  ` normal weekday
fn cell(d: NaiveDate, today: NaiveDate, holidays: &BTreeMap<NaiveDate, String>) -> String {
    let n = d.day();
    if d == today {
        format!(" [{n:>2}] ")
    } else if holidays.contains_key(&d) {
        format!("  {n:>2}* ")
    } else if matches!(d.weekday(), Weekday::Sat | Weekday::Sun) {
        format!("  {n:>2}· ")
    } else {
        format!("  {n:>2}  ")
    }
}

fn print_legend(holidays: &BTreeMap<NaiveDate, String>) {
    if holidays.is_empty() {
        return;
    }
    println!();
    println!("Legend: [Today], Holiday*, Sat/Sun·");
    println!();
    println!("Holidays in window:");
    for (d, name) in holidays {
        println!(
            "  {} ({}) {}",
            d.format("%Y-%m-%d"),
            weekday_ja(d.weekday()),
            name
        );
    }
}

fn weekday_ja(w: Weekday) -> &'static str {
    match w {
        Weekday::Mon => "月",
        Weekday::Tue => "火",
        Weekday::Wed => "水",
        Weekday::Thu => "木",
        Weekday::Fri => "金",
        Weekday::Sat => "土",
        Weekday::Sun => "日",
    }
}
