#[create_cargofuzz_harness(module)]
pub fn fn_no_borrow(string: String) -> usize {
    string.len()
}
