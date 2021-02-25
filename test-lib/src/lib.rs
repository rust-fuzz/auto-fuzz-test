#[macro_use]
extern crate auto_fuzz_test;
#[macro_use]
extern crate arbitrary;

#[create_cargofuzz_harness]
pub fn bool_to_num(string: String) -> usize {
    string.len()
}

pub mod module;

#[derive(Arbitrary, Debug)]
pub struct TestStruct {
    a: u64,
    b: u64,
}
#[create_cargofuzz_impl_harness]
impl TestStruct {
    pub fn new(a: u64, b: u64) -> TestStruct {
        TestStruct { a, b }
    }

    pub fn get_a(&self) -> u64 {
        self.a
    }

    pub fn set_b(&mut self, b: u64) {
        self.b = b;
    }

    pub fn multiply(&mut self) {
        self.a *= self.b;
    }
}
