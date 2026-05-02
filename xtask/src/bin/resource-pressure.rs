//! resource-pressure — one-line friendly summary of system pressure.
//!
//! Combines four signals into a single agent-readable line:
//!
//! - `global_cpu_usage` — system-wide CPU usage percentage right
//!   now (sampled across `MINIMUM_CPU_UPDATE_INTERVAL`).
//! - `load_avg_1 / num_cpus` — 1-minute load average normalized
//!   by logical CPU count. < 1.0 means the box has headroom;
//!   > 1.0 means runnable processes are queueing.
//! - `available_memory / total_memory` — how much RAM the OS
//!   considers reclaimable for new allocations vs. total.
//! - `used_swap / total_swap` — pressure indicator; non-zero
//!   means the box has spilled to disk-backed memory.
//!
//! Use before kicking off something resource-heavy
//! (`pre-landing.sh`, a Codex dispatch, a `cargo test --workspace`
//! pass) or when investigating "cargo appears stuck" alongside
//! `build-status.sh`. Output is one terminal line, no headers.
//!
//! Usage:
//!   ./scripts/xtask.sh resource-pressure

use std::thread::sleep;
use sysinfo::{
    CpuRefreshKind, MINIMUM_CPU_UPDATE_INTERVAL, MemoryRefreshKind, RefreshKind, System,
};

fn fmt_bytes(b: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = 1024.0 * KIB;
    const GIB: f64 = 1024.0 * MIB;
    const TIB: f64 = 1024.0 * GIB;
    let f = b as f64;
    if f >= TIB {
        format!("{:.2}T", f / TIB)
    } else if f >= GIB {
        format!("{:.2}G", f / GIB)
    } else if f >= MIB {
        format!("{:.1}M", f / MIB)
    } else if f >= KIB {
        format!("{:.0}K", f / KIB)
    } else {
        format!("{b}B")
    }
}

fn pct(num: u64, denom: u64) -> f64 {
    if denom == 0 {
        0.0
    } else {
        100.0 * (num as f64) / (denom as f64)
    }
}

fn main() {
    let mut sys = System::new_with_specifics(
        RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
            .with_memory(MemoryRefreshKind::everything()),
    );

    // global_cpu_usage requires two samples ≥ MINIMUM_CPU_UPDATE_INTERVAL
    // apart; the first refresh seeds the baseline, the sleep observes
    // activity, the second refresh reads the delta.
    sleep(MINIMUM_CPU_UPDATE_INTERVAL);
    sys.refresh_cpu_usage();
    sys.refresh_memory();

    let cpu_pct = sys.global_cpu_usage();

    let load = System::load_average();
    let num_cpus = sys.cpus().len().max(1);
    let load_pressure = load.one / num_cpus as f64;

    let avail_mem = sys.available_memory();
    let total_mem = sys.total_memory();
    let mem_avail_pct = pct(avail_mem, total_mem);

    let used_swap = sys.used_swap();
    let total_swap = sys.total_swap();
    let swap_used_pct = pct(used_swap, total_swap);

    println!(
        "cpu {cpu_pct:.1}% | load1/cpus {load_pressure:.2} | mem {avail_mem_h}/{total_mem_h} avail ({mem_avail_pct:.1}%) | swap {used_swap_h}/{total_swap_h} used ({swap_used_pct:.1}%)",
        avail_mem_h = fmt_bytes(avail_mem),
        total_mem_h = fmt_bytes(total_mem),
        used_swap_h = fmt_bytes(used_swap),
        total_swap_h = fmt_bytes(total_swap),
    );
}
