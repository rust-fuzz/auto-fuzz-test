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
    FnVisitor{callback: Box::new(print_a_test)}.visit_file(&syntax_tree);
}

fn print_a_test(this: Option<&Type>, ident: &Ident, decl: &FnDecl, _unsafety: &Option<Unsafe>, _asyncness: &Option<Async>) {
    print!("proptest! {{ #[test] fn test_{}_fuzz (", ident);
    if let Some(self_type) = &this {
        let self_type = format!("{}", quote!(#self_type));
        print!("self_like_thing: Any::<{}>(), ", self_type);
    }
    for a in &decl.inputs {
        if let FnArg::Captured(a) = a {
            let pat = &a.pat;
            let ty = &a.ty;
            print!("{}: Any::<{}>(), ", quote!(#pat), quote!(#ty));
        }
    }
    print!(") {{");
    if this.is_some() {
        print!("self_like_thing.");
    }
    print!("{} (", ident);
    for a in &decl.inputs {
        if let FnArg::Captured(a) = a {
            let pat = &a.pat;
            print!("{}, ", quote!(#pat));
        }
    }
    println!(")}}}}");
}