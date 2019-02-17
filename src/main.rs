use quote::quote;
use syn::{visit::Visit, File, FnArg, FnDecl, Ident, ImplItem, ItemFn, ItemImpl};

fn print_a_test(this: Option<&str>, ident: &Ident, decl: &FnDecl) {
    print!("proptest! {{ #[test] fn test_{}_fuzz (", ident);
    if let Some(self_type) = &this {
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

struct FnVisitor;

impl<'ast> Visit<'ast> for FnVisitor {
    fn visit_item_fn(&mut self, f: &'ast ItemFn) {
        print_a_test(None, &f.ident, &*f.decl);
        syn::visit::visit_item_fn(self, f);
    }
    fn visit_item_impl(&mut self, f: &'ast ItemImpl) {
        let self_type = &f.self_ty;
        let self_type = format!("{}", quote!(#self_type));

        for item in &f.items {
            if let ImplItem::Method(f) = item {
                print_a_test(Some(&self_type), &f.sig.ident, &f.sig.decl);
            }
        }
        syn::visit::visit_item_impl(self, f);
    }
}

fn main() {
    let code = quote! {
        pub fn f(a: String) {}
        pub fn g(b: String, c: bool) {}
        impl String {
            fn h(&self, d: u8) {}
            fn i(self, e: u8) {}
        }
    };

    let syntax_tree: File = syn::parse2(code).expect("Failed to parse input. Is it Rust code?");
    FnVisitor.visit_file(&syntax_tree);
}
