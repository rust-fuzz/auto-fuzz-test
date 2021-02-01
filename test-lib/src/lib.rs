#[macro_use]
extern crate auto_fuzz_test;
#[macro_use]
extern crate arbitrary;

#[create_cargofuzz_harness]
pub fn bool_to_num(string: String) -> usize {
    string.len()
}

pub mod module;
