extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;
use proc_macro2::TokenStream;
use std::env;
use std::fs;
use syn::FnArg::Typed;
use syn::__private::Span;
use syn::{Expr, Fields, GenericArgument, Ident, ItemFn, Member, Pat, PathArguments, Stmt, Type};

mod crate_parse;

#[proc_macro_attribute]
pub fn create_cargofuzz_harness(
    attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let output = transform_stream(TokenStream::from(attr), input);
    proc_macro::TokenStream::from(output)
}

fn transform_stream(attr: TokenStream, input: proc_macro::TokenStream) -> TokenStream {
    // By now, we can parse only standalone functions
    let function: ItemFn = syn::parse(input).expect("Failed to parse input");

    // Checking that the function meets the requirements
    assert_eq!(
        function.sig.asyncness, None,
        "Can not fuzz async functions."
    );
    assert_eq!(
        function.sig.unsafety, None,
        "unsafe functions can not be fuzzed automatically."
    );
    //assert!(
    //<Generic type parameter>,
    //"Generics are not currently supported."
    //);
    //TODO: tests

    // struct and function harness templates
    let mut arg_struct: syn::ItemStruct = syn::parse_str(
        "#[derive(Arbitrary)]
        #[derive(Debug)]
            pub struct fuzz {a:u32, b:Box<u64>}",
    )
    .unwrap();

    let mut fuzzing_harness: syn::ItemFn = syn::parse_str(
        "pub fn fuzz(mut input:MyStruct) {
           foo(input.a, &mut *input.b); 
        }",
    )
    .unwrap();

    // Arguments for internal function call
    if let Stmt::Semi(Expr::Call(fn_call), _) = &mut fuzzing_harness.block.stmts[0] {
        let args = &mut fn_call.args;
        let default_borrowed_field = args.pop().unwrap().into_value();
        let default_field = args.pop().unwrap().into_value();

        // Struct fields generation
        if let Fields::Named(ref mut fields) = arg_struct.fields {
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
            for item in function.sig.inputs.iter() {
                if let Typed(i) = item {
                    if let Pat::Ident(id) = &*i.pat {
                        // `variable` is a new struct field
                        match *i.ty.clone() {
                            Type::Reference(rf) => {
                                if let Type::Path(path) = *rf.elem.clone() {
                                    // `variable` is a new struct field
                                    let mut variable = default_boxed_variable.clone();
                                    variable.ident = Some(id.ident.clone());

                                    let mut new_field = default_borrowed_field.clone();
                                    if let Expr::Reference(ref mut new_rf) = new_field {
                                        // Copying borrow mutability
                                        new_rf.mutability = rf.mutability;
                                        // Copying variable ident
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
                                    // Pushing arguments to the function call
                                    args.push(new_field);
                                    // Pushing variable type for the struct field
                                    fields.named.push(variable);
                                } else {
                                    unimplemented!(
                                        "Sliced arguments."
                                    );
                                }
                            }
                            Type::Path(path) => {
                                // `variable` is a new struct field
                                let mut variable = default_variable.clone();
                                variable.ident = Some(id.ident.clone());
                                let mut new_field = default_field.clone();
                                if let Expr::Field(ref mut f) = new_field {
                                    f.member = Member::Named(id.ident.clone());
                                } else {
                                    panic!("Wrong unborrowed field template");
                                }
                                variable.ty = Type::Path(path);
                                // Pushing arguments to the function call
                                args.push(new_field);
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
    } else {
        panic!("Template must contain the function call.");
    }
    // TODO: Better error messages

    // Struct ident generation
    arg_struct.ident = Ident::new(
        &("__fuzz_struct_".to_owned() + &function.sig.ident.to_string()),
        Span::call_site(),
    );

    // Fuzing harness input type
    if let Typed(i) = fuzzing_harness.sig.inputs.iter_mut().next().unwrap() {
        if let Type::Path(typ) = &mut *i.ty {
            typ.path.segments.iter_mut().next().unwrap().ident = arg_struct.ident.clone();
            // Variable type
        }
    }

    // Fuzzing harness ident
    fuzzing_harness.sig.ident = Ident::new(
        &("__fuzz_".to_owned() + &function.sig.ident.to_string()),
        Span::call_site(),
    );

    // Function call inside fuzzing harness
    if let Stmt::Semi(Expr::Call(fn_call), _) = &mut fuzzing_harness.block.stmts[0] {
        if let Expr::Path(path) = &mut *fn_call.func {
            path.path.segments.iter_mut().next().unwrap().ident = function.sig.ident.clone();
        }
    }

    let crate_info = crate_parse::CrateInfo::from_root(&env::current_dir().expect(
        "Failed to obtain
            project root dir",
    ))
    .expect("Failed to obtain crate info");

    let fuzz_dir_path = crate_info.fuzz_dir().expect("Failed to create fuzz dir");

    let crate_name_underscored = str::replace(crate_info.crate_name(), "-", "_"); // required for `extern crate`

    let crate_ident = Ident::new(&crate_name_underscored, Span::call_site());

    // Writing fuzzing harness to file
    let code = crate_parse::compose_fn_invocation(
        &fuzzing_harness.sig.ident,
        &arg_struct.ident,
        &crate_ident,
        attr,
    );

    fs::write(
        fuzz_dir_path.join(String::new() + &function.sig.ident.to_string() + ".rs"),
        code,
    )
    .expect("Failed to write fuzzing harness to fuzz/fuzz_targets");
    // TODO: Error handing

    crate_info
        .write_cargo_toml(&function.sig.ident)
        .expect("Failed to update Cargo.toml");

    quote!(
        #function
      #arg_struct
    #fuzzing_harness
    )
}
// For testing purposes
//let test_struct: syn::ItemStruct = dbg!(syn::parse_str(
//"#[derive(Arbitrary)]
//pub struct fuzz {
//a:u64,
//b:u64,
//crash_on_overflow:bool
//}"
//)
//.unwrap());
