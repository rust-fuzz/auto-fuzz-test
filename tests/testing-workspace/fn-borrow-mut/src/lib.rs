#[macro_use]
extern crate auto_fuzz_test;
#[macro_use]
extern crate arbitrary;

#[create_cargofuzz_harness]
pub fn fn_borrow_mut(num: &mut i32) {
    *num += 1;
}
