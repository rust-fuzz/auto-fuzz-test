#[create_cargofuzz_harness(module)]
pub fn fn_borrow(string: &str) -> usize {
    string.len()
}
