use proc_macro2::TokenStream;
use syn::FnArg::Typed;
use syn::__private::Span;
use syn::{
    Expr, Fields, GenericArgument, Ident, ItemFn, ItemStruct, Member, Pat, PathArguments, Stmt,
    Type,
};

pub fn fuzz_struct(function: &ItemFn) -> ItemStruct {
    // struct for function arguments template
    let mut fuzz_struct: ItemStruct = syn::parse_str(
        "#[derive(Arbitrary)]
        #[derive(Debug)]
            pub struct fuzz {a:u32, b:Box<u64>}",
    )
    .unwrap();

    // Struct ident generation
    fuzz_struct.ident = Ident::new(
        &("__fuzz_struct_".to_owned() + &(*function).sig.ident.to_string()),
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
        for item in (*function).sig.inputs.iter() {
            if let Typed(i) = item {
                if let Pat::Ident(id) = &*i.pat {
                    match *i.ty.clone() {
                        Type::Reference(rf) => {
                            if let Type::Path(path) = *rf.elem.clone() {
                                // `variable` is a new struct field
                                let mut variable = default_boxed_variable.clone();
                                variable.ident = Some(id.ident.clone());

                                // Copying variable type
                                if let Type::Path(ref mut new_path) = variable.ty {
                                    if let PathArguments::AngleBracketed(ref mut new_generic_arg) =
                                        new_path.path.segments.iter_mut().next().unwrap().arguments
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
            } else {
                unimplemented!("Only standalone functions are currently supported.");
            }
        }
    } else {
        panic!("Struct template must contain named fields");
    }

    fuzz_struct
}

pub fn fuzz_function(function: &ItemFn) -> ItemFn {
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

        for item in (*function).sig.inputs.iter() {
            if let Typed(i) = item {
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
                                        new_unary_subfield.member = Member::Named(id.ident.clone());
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
            } else {
                unimplemented!("Only standalone functions are currently supported.");
            }
        }
    }

    // Fuzing function input type
    if let Typed(i) = fuzz_function.sig.inputs.iter_mut().next().unwrap() {
        if let Type::Path(typ) = &mut *i.ty {
            typ.path.segments.iter_mut().next().unwrap().ident = Ident::new(
                &("__fuzz_struct_".to_owned() + &(*function).sig.ident.to_string()),
                Span::call_site(),
            );
        }
    }

    // Fuzzing function ident
    fuzz_function.sig.ident = Ident::new(
        &("__fuzz_".to_owned() + &(*function).sig.ident.to_string()),
        Span::call_site(),
    );

    // FnCall inside fuzzing function
    if let Stmt::Semi(Expr::Call(fn_call), _) = &mut fuzz_function.block.stmts[0] {
        if let Expr::Path(path) = &mut *fn_call.func {
            path.path.segments.iter_mut().next().unwrap().ident = (*function).sig.ident.clone();
        }
    }

    fuzz_function
}

pub fn fuzz_harness(function: &ItemFn, crate_ident: &Ident, attr: TokenStream) -> TokenStream {
    let arg_type = Ident::new(
        &("__fuzz_struct_".to_owned() + &(*function).sig.ident.to_string()),
        Span::call_site(),
    );
    let function_ident = Ident::new(
        &("__fuzz_".to_owned() + &(*function).sig.ident.to_string()),
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
