use syn;
use syn::{visit::Visit};
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
    FnVisitor.visit_file(&syntax_tree);
}
