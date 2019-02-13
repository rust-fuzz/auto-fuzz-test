use syn::FnArg::Captured;
use quote::quote;
use syn::{visit::Visit, File, ItemFn};

struct FnVisitor;

impl<'ast> Visit<'ast> for FnVisitor {
    fn visit_item_fn(&mut self, f: &'ast ItemFn) {
        let name = &f.ident;
        print!("proptest! {{ #[test] fn test_{}_fuzz (", name);
        for a in &f.decl.inputs {
            if let Captured(a) = a {
                let pat = &a.pat;
                let ty = &a.ty;
                print!("{}: Any::<{}>(),", quote!(#pat), quote!(#ty));
            }
        }
        print!(") {{");
        print!("{} (", name);
        for a in &f.decl.inputs {
            if let Captured(a) = a {
                let pat = &a.pat;
                print!("{},", quote!(#pat));
            }
        }
        println!(")}}}}");
        syn::visit::visit_item_fn(self, f);
    }
}

fn main() {
    let code = quote! {
        pub fn f(a: String) {}
        pub fn g(b: String, c: bool) {}
    };

    let syntax_tree: File = syn::parse2(code).unwrap();
    FnVisitor.visit_file(&syntax_tree);
}
