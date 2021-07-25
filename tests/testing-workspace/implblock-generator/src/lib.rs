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
    pub fn generator(a: u64, b: u64) -> ImplBlock {
        ImplBlock { a, b }
    }
}
