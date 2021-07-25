#[derive(Arbitrary, Debug)]
pub struct ImplBlock {
    a: u64,
    b: u64,
}
#[create_cargofuzz_impl_harness(module)]
impl ImplBlock {
    pub fn borrow(&self) -> u64 {
        self.a
    }
}