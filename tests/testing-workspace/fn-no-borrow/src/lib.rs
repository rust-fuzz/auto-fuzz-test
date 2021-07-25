#[macro_use]
extern crate auto_fuzz_test;
#[macro_use]
extern crate arbitrary;

#[create_cargofuzz_harness]
pub fn fn_no_borrow(string: String) -> usize {
    string.len()
}
