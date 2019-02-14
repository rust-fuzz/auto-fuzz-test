use quote::quote;
use syn::{visit::Visit, File, FnArg, ImplItem, ItemFn, ItemImpl};

struct FnVisitor;

impl<'ast> Visit<'ast> for FnVisitor {
    fn visit_item_fn(&mut self, f: &'ast ItemFn) {
        let name = &f.ident;
        print!("proptest! {{ #[test] fn test_{}_fuzz (", name);
        for a in &f.decl.inputs {
            if let FnArg::Captured(a) = a {
                let pat = &a.pat;
                let ty = &a.ty;
                print!("{}: Any::<{}>(), ", quote!(#pat), quote!(#ty));
            }
        }
        print!(") {{");
        print!("{} (", name);
        for a in &f.decl.inputs {
            if let FnArg::Captured(a) = a {
                let pat = &a.pat;
                print!("{}, ", quote!(#pat));
            }
        }
        println!(")}}}}");
        syn::visit::visit_item_fn(self, f);
    }
    fn visit_item_impl(&mut self, f: &'ast ItemImpl) {
        let self_type = &f.self_ty;

        for item in &f.items {
            if let ImplItem::Method(f) = item {
                let name = &f.sig.ident;
                print!("proptest! {{ #[test] fn test_{}_fuzz (", name);
                print!("self_like_thing: Any::<{}>(), ", quote!(#self_type));
                for a in &f.sig.decl.inputs {
                    if let FnArg::Captured(a) = a {
                        let pat = &a.pat;
                        let ty = &a.ty;
                        print!("{}: Any::<{}>(), ", quote!(#pat), quote!(#ty));
                    }
                }
                print!(") {{ self_like_thing.{} (", name);
                for a in &f.sig.decl.inputs {
                    if let FnArg::Captured(a) = a {
                        let pat = &a.pat;
                        print!("{}, ", quote!(#pat));
                    }
                }
                println!(")}}}}");
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

    let syntax_tree: File = syn::parse2(code).unwrap();
    FnVisitor.visit_file(&syntax_tree);
}
