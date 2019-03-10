use syn::{visit::Visit, FnDecl, Ident, ImplItem, ItemFn, ItemImpl, Type};
use syn::token::{Unsafe, Async};

pub struct FnVisitor {
    pub callback: Box<FnMut(Option<&Type>, &Ident, &FnDecl, &Option<Unsafe>, &Option<Async>) -> ()>
}

impl<'ast> Visit<'ast> for FnVisitor {
    // based on syn visitor example by David Tolnay:
    // https://github.com/dtolnay/syn/issues/549
    fn visit_item_fn(&mut self, f: &'ast ItemFn) {
        (self.callback)(None, &f.ident, &*f.decl, &f.unsafety, &f.asyncness);
        syn::visit::visit_item_fn(self, f);
    }
    fn visit_item_impl(&mut self, f: &'ast ItemImpl) {
        let self_type = &f.self_ty;
        for item in &f.items {
            if let ImplItem::Method(f) = item {
                (self.callback)(Some(self_type), &f.sig.ident, &f.sig.decl, &f.sig.unsafety, &f.sig.asyncness);
            }
        }
        syn::visit::visit_item_impl(self, f);
    }
}
