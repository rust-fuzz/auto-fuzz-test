#[macro_use]
extern crate auto_fuzz_test;
#[macro_use]
extern crate arbitrary;

#[create_cargofuzz_harness]
pub fn bool_to_num(string: String) -> usize {
    string.len()
}
//impl MyString {
//fn h(&self, d: u8) {}
//fn i(self, e: u8) {}
//}

pub mod module;
