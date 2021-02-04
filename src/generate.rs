use proc_macro2::TokenStream;
use syn::FnArg;
use syn::__private::Span;
use syn::{
    Expr, Fields, GenericArgument, Ident, ItemFn, ItemStruct, Member, Pat, PathArguments,
    Signature, Stmt, Type,
};

pub fn fuzz_struct(signature: &Signature, impl_type: Option<Type>) -> ItemStruct {
    // struct for function arguments template
    let mut fuzz_struct: ItemStruct = syn::parse_str(
        "#[derive(Arbitrary)]
        #[derive(Debug)]
            pub struct fuzz {a:u32, b:Box<u64>}",
    )
    .unwrap();

    // Struct ident generation
    fuzz_struct.ident = Ident::new(
        &("__fuzz_struct_".to_owned() + &(*signature).ident.to_string()),
        Span::call_site(),
    );

    // Struct fields generation
    if let Fields::Named(ref mut fields) = fuzz_struct.fields {
        let default_boxed_variable = fields
            .named
            .pop()
            .expect(
                "Struct template must contain
                Boxed variable",
            )
            .into_value();
        let default_variable = fields
            .named
            .pop()
            .expect(
                "Struct template must contain
                unBoxed variable",
            )
            .into_value();
        for item in (*signature).inputs.iter() {
            match item {
                FnArg::Typed(i) => {
                    if let Pat::Ident(id) = &*i.pat {
                        match *i.ty.clone() {
                            Type::Reference(rf) => {
                                if let Type::Path(path) = *rf.elem.clone() {
                                    // `variable` is a new struct field
                                    let mut variable = default_boxed_variable.clone();
                                    variable.ident = Some(id.ident.clone());

                                    // Copying variable type
                                    if let Type::Path(ref mut new_path) = variable.ty {
                                        if let PathArguments::AngleBracketed(
                                            ref mut new_generic_arg,
                                        ) = new_path
                                            .path
                                            .segments
                                            .iter_mut()
                                            .next()
                                            .unwrap()
                                            .arguments
                                        {
                                            if let GenericArgument::Type(ref mut new_subpath) =
                                                new_generic_arg.args.iter_mut().next().unwrap()
                                            {
                                                *new_subpath = Type::Path(path);
                                            } else {
                                                panic!("Wrong boxed variable template");
                                            }
                                        } else {
                                            panic!("Wrong boxed variable template");
                                        }
                                    } else {
                                        panic!("Wrong boxed variable template");
                                    }
                                    // Pushing variable type for the struct field
                                    fields.named.push(variable);
                                } else {
                                    unimplemented!("Sliced arguments.");
                                }
                            }
                            Type::Path(path) => {
                                // `variable` is a new struct field
                                let mut variable = default_variable.clone();
                                variable.ident = Some(id.ident.clone());
                                // Copying variable type
                                variable.ty = Type::Path(path);
                                // Pushing variable type for the struct field
                                fields.named.push(variable);
                            }
                            _ => {
                                unimplemented!("Type of the function must be either standalone, or borrowed standalone");
                            }
                        };
                    } else {
                        unimplemented!("Only simple arguments are currently supported.");
                    }
                }
                FnArg::Receiver(res) => {
                    unimplemented!("Only standalone functions are currently supported.");
                }
            }
        }
    } else {
        panic!("Struct template must contain named fields");
    }

    fuzz_struct
}

pub fn fuzz_function(signature: &Signature, impl_type: Option<Type>) -> ItemFn {
    // function harness template
    let mut fuzz_function: syn::ItemFn = syn::parse_str(
        "pub fn fuzz(mut input:MyStruct) {
           foo(input.a, &mut *input.b); 
        }",
    )
    .unwrap();

    // Arguments for internal function call
    if let Stmt::Semi(Expr::Call(fn_call), _) = &mut fuzz_function.block.stmts[0] {
        let args = &mut fn_call.args;
        let default_borrowed_field = args.pop().unwrap().into_value();
        let default_field = args.pop().unwrap().into_value();

        for item in (*signature).inputs.iter() {
            match item {
                FnArg::Typed(i) => {
                    if let Pat::Ident(id) = &*i.pat {
                        match *i.ty.clone() {
                            Type::Reference(rf) => {
                                let mut new_field = default_borrowed_field.clone();
                                if let Expr::Reference(ref mut new_rf) = new_field {
                                    // Copying borrow mutability
                                    new_rf.mutability = rf.mutability;
                                    // Copying field ident
                                    if let Expr::Unary(ref mut new_subfield) = *new_rf.expr {
                                        if let Expr::Field(ref mut new_unary_subfield) =
                                            *new_subfield.expr
                                        {
                                            new_unary_subfield.member =
                                                Member::Named(id.ident.clone());
                                        } else {
                                            panic!("Wrong borrowed field template");
                                        }
                                    } else {
                                        panic!("Wrong borrowed field template");
                                    }
                                } else {
                                    panic!("Wrong borrowed field template");
                                }

                                // Pushing arguments to the function call
                                args.push(new_field);
                            }
                            Type::Path(_) => {
                                let mut new_field = default_field.clone();
                                if let Expr::Field(ref mut f) = new_field {
                                    f.member = Member::Named(id.ident.clone());
                                } else {
                                    panic!("Wrong unborrowed field template");
                                }
                                // Pushing arguments to the function call
                                args.push(new_field);
                            }
                            _ => {
                                unimplemented!("Type of the function must be either standalone, or borrowed standalone");
                            }
                        };
                    } else {
                        unimplemented!("Only simple arguments are currently supported.");
                    }
                }
                FnArg::Receiver(res) => {
                    unimplemented!("Only standalone functions are currently supported.");
                }
            }
        }
    }

    // Fuzing function input type
    if let FnArg::Typed(i) = fuzz_function.sig.inputs.iter_mut().next().unwrap() {
        if let Type::Path(typ) = &mut *i.ty {
            typ.path.segments.iter_mut().next().unwrap().ident = Ident::new(
                &("__fuzz_struct_".to_owned() + &(*signature).ident.to_string()),
                Span::call_site(),
            );
        }
    }

    // Fuzzing function ident
    fuzz_function.sig.ident = Ident::new(
        &("__fuzz_".to_owned() + &(*signature).ident.to_string()),
        Span::call_site(),
    );

    // FnCall inside fuzzing function
    if let Stmt::Semi(Expr::Call(fn_call), _) = &mut fuzz_function.block.stmts[0] {
        if let Expr::Path(path) = &mut *fn_call.func {
            path.path.segments.iter_mut().next().unwrap().ident = (*signature).ident.clone();
        }
    }

    fuzz_function
}

pub fn fuzz_harness(signature: &Signature, crate_ident: &Ident, attr: TokenStream) -> TokenStream {
    let arg_type = Ident::new(
        &("__fuzz_struct_".to_owned() + &(*signature).ident.to_string()),
        Span::call_site(),
    );
    let function_ident = Ident::new(
        &("__fuzz_".to_owned() + &(*signature).ident.to_string()),
        Span::call_site(),
    );

    let path = {
        if !attr.is_empty() {
            quote!(#crate_ident :: #attr ::)
        } else {
            quote!(#crate_ident ::)
        }
    };

    let code = quote!(
            // Autogenerated fuzzing harness.
    #![no_main]
            use libfuzzer_sys::fuzz_target;
            extern crate #crate_ident;

            fuzz_target!(|input: #path #arg_type| {
            #path #function_ident (input);
            });
        );

    code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_struct_no_borrows() {
        let function: ItemFn = syn::parse2(quote! {
            pub fn maybe_checked_mul(a: u64, b: u64, crash_on_overflow: bool) -> u64 {
                if crash_on_overflow {
                    a.checked_mul(b).expect("Overflow has occurred")
                } else {
                    a.overflowing_mul(b).0
                }
            }
        })
        .unwrap();

        let fuzz_struct_needed: ItemStruct = syn::parse2(quote! {
            #[derive(Arbitrary)]
            #[derive(Debug)]
            pub struct __fuzz_struct_maybe_checked_mul {
                a: u64,
                b: u64,
                crash_on_overflow: bool
            }
        })
        .unwrap();

        assert_eq!(fuzz_struct(&function.sig, None), fuzz_struct_needed);
    }

    #[test]
    fn test_function_no_borrows() {
        let function: ItemFn = syn::parse2(quote! {
            pub fn maybe_checked_mul(a: u64, b: u64, crash_on_overflow: bool) -> u64 {
                if crash_on_overflow {
                    a.checked_mul(b).expect("Overflow has occurred")
                } else {
                    a.overflowing_mul(b).0
                }
            }
        })
        .unwrap();

        let fuzz_function_needed: ItemFn = syn::parse2(quote! {
            pub fn __fuzz_maybe_checked_mul(mut input:__fuzz_struct_maybe_checked_mul) {
                maybe_checked_mul(input.a, input.b, input.crash_on_overflow);
            }
        })
        .unwrap();

        assert_eq!(fuzz_function(&function.sig, None), fuzz_function_needed);
    }

    #[test]
    fn test_struct_borrowed() {
        let function: ItemFn = syn::parse2(quote! {
            pub fn maybe_checked_mul_borrowed(a: &mut u64, b: u64, crash_on_overflow: bool) {
                if crash_on_overflow {
                    *a = a.checked_mul(b).expect("Overflow has occurred");
                } else {
                    *a = a.overflowing_mul(b).0;
                }
            }
        })
        .unwrap();

        let fuzz_struct_needed: ItemStruct = syn::parse2(quote! {
            #[derive(Arbitrary)]
            #[derive(Debug)]
            pub struct __fuzz_struct_maybe_checked_mul_borrowed {
                a: Box<u64>,
                b: u64,
                crash_on_overflow: bool
            }
        })
        .unwrap();

        assert_eq!(fuzz_struct(&function.sig, None), fuzz_struct_needed);
    }

    #[test]
    fn test_function_borrowed() {
        let function: ItemFn = syn::parse2(quote! {
            pub fn maybe_checked_mul_borrowed(a: &mut u64, b: u64, crash_on_overflow: bool) {
                if crash_on_overflow {
                    *a = a.checked_mul(b).expect("Overflow has occurred");
                } else {
                    *a = a.overflowing_mul(b).0;
                }
            }
        })
        .unwrap();

        let fuzz_function_needed: ItemFn = syn::parse2(
            quote! {
                pub fn __fuzz_maybe_checked_mul_borrowed(mut input:__fuzz_struct_maybe_checked_mul_borrowed) {
                    maybe_checked_mul_borrowed(&mut *input.a, input.b, input.crash_on_overflow);
                }
            }
        ).unwrap();

        assert_eq!(fuzz_function(&function.sig, None), fuzz_function_needed);
    }

    #[test]
    fn test_function_harness() {
        let function: ItemFn = syn::parse2(quote! {
            pub fn maybe_checked_mul(a: u64, b: u64, crash_on_overflow: bool) -> u64 {
                if crash_on_overflow {
                    a.checked_mul(b).expect("Overflow has occurred")
                } else {
                    a.overflowing_mul(b).0
                }
            }
        })
        .unwrap();

        let fuzz_harness_needed: syn::File = syn::parse2(quote! {
            #![no_main]
            use libfuzzer_sys::fuzz_target;
            extern crate test_lib;

            fuzz_target!( |input: test_lib::foo::bar::__fuzz_struct_maybe_checked_mul| {
                    test_lib::foo::bar::__fuzz_maybe_checked_mul(input);
                }
            );
        })
        .unwrap();

        let attrs = quote!(foo::bar);
        let crate_ident = Ident::new("test_lib", Span::call_site());
        let fuzz_harness_generated: syn::File =
            syn::parse2(fuzz_harness(&function.sig, &crate_ident, attrs)).unwrap();

        assert_eq!(fuzz_harness_generated, fuzz_harness_needed);
    }
}
