#[create_cargofuzz_harness(module)]
pub fn fn_borrow_mut(num: &mut i32) {
    *num += 1;
}
