//! system-resources — print thread count and memory stats.
//!
//! Outputs one line: `<nthreads>\t<avail_mem>/<total_mem>`
//! where memory values are in bytes.
//!
//! Usage:
//!   ./scripts/xtask.sh system-resources

use sysinfo::System;

fn main() {
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    let mut sys = System::new();
    sys.refresh_memory();

    let total = sys.total_memory();
    let available = sys.available_memory();

    println!("{threads}\t{available}/{total}");
}
