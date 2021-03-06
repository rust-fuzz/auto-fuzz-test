#[test]
#[ignore]
fn fn_nomodules_noborrows() {
    let t = trybuild::TestCases::new();
    t.pass("tests/builds/fn_no_modules_no_borrows.rs");
}
