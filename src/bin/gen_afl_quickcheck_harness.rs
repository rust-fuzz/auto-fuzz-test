use std::fmt::Write;
use syn;
use syn::{visit::Visit};
use syn::{FnArg, FnDecl, Ident, Type};
use syn::token::{Unsafe, Async};
use quote::quote;
use auto_fuzz_test::FnVisitor;

fn main() {
    let code = quote! {
        pub fn f(a: String) {}
        pub fn g(b: String, c: bool) {}
        impl String {
            fn h(&self, d: u8) {}
            fn i(self, e: u8) {}
        }
    };

    let syntax_tree: syn::File = syn::parse2(code).expect("Failed to parse input. Is it Rust code?");
    // function print_a_test doesn't care about some of the parameters, so we throw them away here
    let callback = |this: Option<&Type>, ident: &Ident, decl: &FnDecl, unsafety: &Option<Unsafe>, asyncness: &Option<Async>| {
        // Unsafe functions cannot have fuzzing harnesses generated automatically,
        // since it's valid for them to crash for some inputs.
        // Async functions are simply not supported for now.
        if unsafety.is_none() && asyncness.is_none() {
            println!("{}", generate_fuzzing_harness(this, ident, decl));
        }
    };
    FnVisitor{callback: Box::new(callback)}.visit_file(&syntax_tree);
}

fn generate_fuzzing_harness(this: Option<&Type>, ident: &Ident, decl: &FnDecl) -> String {
    let mut result = String::from("
extern crate rand;
extern crate quickcheck;

use rand;
use std::io::prelude::*;
use quickcheck::{Arbitrary};

// suppress ASAN false positives
const ASAN_DEFAULT_OPTIONS: &'static [u8] = b\"allocator_may_return_null=1,detect_odr_violation=1\0\";
#[no_mangle]
pub extern \"C\" fn __asan_default_options() -> *const u8 {
    ASAN_DEFAULT_OPTIONS as *const [u8] as *const u8
}

fn main() -> std::result::Result<(), std::io::Error> {
    // read fuzzer input from stdin
    let mut raw_input = vec![];
    std::io::stdin().read_to_end(&mut raw_input)?;

    // input preparation for QuickCheck, not specific to the fuzzed function
    let input_cursor = std::io::Cursor::new(raw_input);
    let read_rng = rand::rngs::adapter::ReadRng::new(input_cursor);
    let mut read_rng = quickcheck::StdGen::new(read_rng, std::usize::MAX);

    // create input data for specific function from random bytes
");
    // print creation of variables
    if let Some(self_type) = &this {
        writeln!(&mut result, "    fuzz_self = {}::arbitrary(&mut read_rng);", quote!(#self_type)).unwrap();
    }
    let mut arg_numbers: Vec<usize> = Vec::new();
    for (num, a) in decl.inputs.iter().enumerate() {
        if let FnArg::Captured(a) = a {
            let arg_type = &a.ty;
            writeln!(&mut result, "    fuzz_arg_{} = {}::arbitrary(&mut read_rng);", num, quote!(#arg_type)).unwrap();
            arg_numbers.push(num);
        }
    };
    // print actual invocation of the function
    write!(&mut result, "\n    // invoke function\n    ").unwrap();
    if this.is_some() {
        write!(&mut result, "fuzz_self.").unwrap();
    }
    write!(&mut result, "{}(", ident).unwrap();
    let is_first_argument = true;
    for arg_num in arg_numbers {
        write!(&mut result, "fuzz_arg_{}", arg_num).unwrap();
        if ! is_first_argument {write!(&mut result, ",").unwrap()};
    }


    writeln!(&mut result, ");\n    Ok(())\n}}").unwrap();
    result
}