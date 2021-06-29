#[macro_use]
extern crate auto_fuzz_test;
#[macro_use]
extern crate arbitrary;

#[create_cargofuzz_harness]
pub fn fn_no_borrow(string: String) -> usize {
    string.len()
}

#[create_cargofuzz_harness]
pub fn fn_borrow(string: &str) -> usize {
    string.len()
}

#[create_cargofuzz_harness]
pub fn fn_borrow_mut(num: &mut i32) {
    *num += 1;
}

pub mod module;

#[derive(Arbitrary, Debug)]
pub struct ImplBlock {
    a: u64,
    b: u64,
}
#[create_cargofuzz_impl_harness]
impl ImplBlock {
    pub fn generator(a: u64, b: u64) -> ImplBlock {
        ImplBlock { a, b }
    }

    pub fn borrow(&self) -> u64 {
        self.a
    }

    pub fn borrow_mut(&mut self, b: u64) {
        self.b = b;
    }
}
