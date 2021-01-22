#[create_cargofuzz_harness]
pub fn maybe_checked_mul(a: u64, b: u64, crash_on_overflow: bool) -> u64 {
    if crash_on_overflow {
        a.checked_mul(b).expect("Overflow has occurred")
    } else {
        a.overflowing_mul(b).0
    }
}
