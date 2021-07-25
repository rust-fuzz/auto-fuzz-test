use std::env;
use std::path::PathBuf;
use test_case::test_case;

mod runner;

#[test_case("ImplBlock_borrow")]
#[test_case("ImplBlock_borrow_mut")]
#[test_case("ImplBlock_generator")]
#[test_case("fn_borrow")]
#[test_case("fn_borrow_mut")]
#[test_case("fn_no_borrow")]
#[test_case("module__fn_borrow")]
#[test_case("module__fn_borrow_mut")]
#[test_case("module__fn_no_borrow")]
#[test_case("module__ImplBlock_borrow")]
#[test_case("module__ImplBlock_borrow_mut")]
#[test_case("module__ImplBlock_generator")]
fn target(target_name: &str) {
    let source_dir = env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap();
    let test_lib_sources = source_dir.join("tests").join("testing-workspace");
    let test_dir = runner::test_dir(&source_dir).unwrap();
    // it will be better if each test will copy only its own testing crate
    runner::copy_dir_all(&test_lib_sources, &test_dir).unwrap();
    let test_dir = test_dir.join(target_name.to_lowercase().replace("_","-"));

    assert!(runner::cargo_build(&test_dir).unwrap().success());
    assert!(runner::fuzz_build(&test_dir.join("fuzz"), target_name).unwrap().success());
}
