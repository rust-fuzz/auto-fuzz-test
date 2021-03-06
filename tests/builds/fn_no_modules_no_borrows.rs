#[macro_use]
extern crate auto_fuzz_test;
extern crate arbitrary;
use arbitrary::{Arbitrary, Unstructured};
use std::env;
use std::fs::{self, File};
use std::io::Read;

#[create_cargofuzz_harness]
pub fn foo(string: String) -> usize {
    string.len()
}

fn main() {
    let raw_bytes = [0, 0, 0, 0, 0, 0, 0, 0];
    let mut unstructured = Unstructured::new(&raw_bytes);
    let test_struct = __fuzz_struct_foo::arbitrary(&mut unstructured).unwrap();

    __fuzz_foo(test_struct);

    let fuzz_path = env::current_dir().unwrap().join("fuzz");
    let mut cargo_toml = File::open(fuzz_path.join("Cargo.toml")).expect("Can't open Cargo.toml");
    let mut cargo_contents = String::new();
    cargo_toml
        .read_to_string(&mut cargo_contents)
        .expect("Can't read Cargo.toml");

    assert_eq!(cargo_contents, VALID_GENERATED_CARGO_TOML_NOMODULE_NOIMPL);

    let mut fuzz_harness =
        File::open(fuzz_path.join("fuzz_targets").join("foo.rs")).expect("Can't open fuzz_target");
    let mut fuzz_harness_contents = String::new();
    fuzz_harness
        .read_to_string(&mut fuzz_harness_contents)
        .expect("Can't read fuzz_target");

    assert_eq!(
        fuzz_harness_contents,
        VALID_GENERATED_FUZZ_HARNESS_NOMODULE_NOIMPL
    );
    fs::remove_dir_all(fuzz_path).unwrap();
}

const VALID_GENERATED_CARGO_TOML_NOMODULE_NOIMPL: &str = r#"[package]
name = "auto-fuzz-test-tests-fuzz"
version = "0.0.0"
authors = ["Automatically generated"]
publish = false
edition = "2018"

[package.metadata]
cargo-fuzz = true

[dependencies]
libfuzzer-sys = "0.4"

[dependencies.auto-fuzz-test-tests]
path = ".."

# Prevent this from interfering with workspaces
[workspace]
members = ["."]

[[bin]]
name = "foo"
path = "fuzz_targets/foo.rs"
test = false
doc = false
"#;
const VALID_GENERATED_FUZZ_HARNESS_NOMODULE_NOIMPL: &str =
    "# ! [no_main] use libfuzzer_sys :: fuzz_target ; extern crate
auto_fuzz_test_tests ; fuzz_target !
(| input : auto_fuzz_test_tests :: __fuzz_struct_foo |
 { auto_fuzz_test_tests :: __fuzz_foo(input) ; }) ;";
