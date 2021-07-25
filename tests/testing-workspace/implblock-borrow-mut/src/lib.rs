#[macro_use]
extern crate auto_fuzz_test;
#[macro_use]
extern crate arbitrary;

#[derive(Arbitrary, Debug)]
pub struct ImplBlock {
    a: u64,
    b: u64,
}
#[create_cargofuzz_impl_harness]
impl ImplBlock {
    pub fn borrow_mut(&mut self, b: u64) {
        self.b = b;
    }
}
