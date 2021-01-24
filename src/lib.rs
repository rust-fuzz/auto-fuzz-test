extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;
use proc_macro2::TokenStream;
use std::env;
use std::fs;
use syn::FnArg::Typed;
use syn::__private::Span;
use syn::{Expr, Fields, Ident, ItemFn, Member, Pat, Stmt, Type};
mod crate_parse;

#[proc_macro_attribute]
pub fn create_cargofuzz_harness(
    attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let output = transform_stream(attr, input);
    proc_macro::TokenStream::from(output)
}

fn transform_stream(attr: proc_macro::TokenStream, input: proc_macro::TokenStream) -> TokenStream {
    // By now, we can parse only standalone functions
    let function: ItemFn = syn::parse(input).unwrap();

    // Checking that the function meets the requirements
    assert_eq!(
        function.sig.asyncness, None,
        "Can not fuzz async functions."
    );
    assert_eq!(
        function.sig.unsafety, None,
        "unsafe functions can not be fuzzed automatically."
    );
    assert!(
        function.sig.generics.params.is_empty(),
        "Generics are not currently supported."
    );
    //TODO: tests

    let mut arg_struct: syn::ItemStruct = syn::parse_str(
        "#[derive(Arbitrary)]
        #[derive(Debug)]
            pub struct fuzz {a:u32}",
    )
    .unwrap();

    if let Fields::Named(ref mut fields) = arg_struct.fields {
        let default_variable = fields.named.pop().unwrap().into_value();
        for item in function.sig.inputs.iter() {
            if let Typed(i) = item {
                if let Pat::Ident(id) = &*i.pat {
                    let mut variable = default_variable.clone();
                    variable.ident = Some(id.ident.clone());
                    variable.ty = *i.ty.clone();
                    fields.named.push(variable);
                } else {
                    panic!("Such functions are no supported yet.");
                    //compile_error!("Wrong syn::Type enum");
                }
            } else {
                panic!("Such functions are no supported yet.");
                //compile_error!("Wrong syn::FnArg enum");
            }
        }
    } else {
        panic!("Such functions are no supported yet.");
        //compile_error!("Wrong syn::Fields enum");
    }
    // TODO: Better error messages

    // Struct ident generation
    arg_struct.ident = Ident::new(
        &("__fuzz_struct_".to_owned() + &function.sig.ident.to_string()),
        Span::call_site(),
    );

    let mut fuzzing_harness: syn::ItemFn = syn::parse_str(
        "pub fn fuzz(input:MyStruct) {
           foo(input.a); 
        }",
    )
    .unwrap();

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

    // Arguments for internal function call
    if let Stmt::Semi(Expr::Call(fn_call), _) = &mut fuzzing_harness.block.stmts[0] {
        let args = &mut fn_call.args;
        let default_field = args.pop().unwrap().into_value();
        if let Fields::Named(fields) = &arg_struct.fields {
            for field in fields.named.iter() {
                let mut new_field = default_field.clone();
                if let Expr::Field(ref mut f) = new_field {
                    if let Member::Named(name) = &mut f.member {
                        *name = field.ident.clone().unwrap();
                    }
                }
                args.push(new_field);
            }
        }
    }

    let crate_info = crate_parse::CrateInfo::from_root(&env::current_dir().unwrap()).unwrap();

    let fuzz_dir_path = crate_info.fuzz_dir().unwrap();


    let code = crate_parse::write_fn_invocation(&fuzzing_harness.sig.ident,&arg_struct.ident, crate_info.crate_name()).unwrap();
    fs::write(fuzz_dir_path.join(String::new()+&function.sig.ident.to_string()+".rs"), code);
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
