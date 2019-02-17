use quote::quote;
use syn::{visit::Visit, FnArg, FnDecl, Ident, ImplItem, ItemFn, ItemImpl, Type};
use syn::token::{Unsafe, Async};

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

pub struct FnVisitor;

impl<'ast> Visit<'ast> for FnVisitor {
    fn visit_item_fn(&mut self, f: &'ast ItemFn) {
        print_a_test(None, &f.ident, &*f.decl, &f.unsafety, &f.asyncness);
        syn::visit::visit_item_fn(self, f);
    }
    fn visit_item_impl(&mut self, f: &'ast ItemImpl) {
        let self_type = &f.self_ty;
        for item in &f.items {
            if let ImplItem::Method(f) = item {
                print_a_test(Some(self_type), &f.sig.ident, &f.sig.decl, &f.sig.unsafety, &f.sig.asyncness);
            }
        }
        syn::visit::visit_item_impl(self, f);
    }
}