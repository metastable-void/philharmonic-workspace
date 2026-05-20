#![forbid(unsafe_code)]
//! detect-virt - print the current systemd-detect-virt-style id.

fn main() {
    println!("{}", philharmonic_virt_detect::detect_virtualization());
}
