extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;
use proc_macro2::TokenStream;
use syn::FnArg::Typed;
use syn::__private::Span;
use syn::{Expr, Fields, Ident, ItemFn, Member, Pat, Stmt, Type};

#[proc_macro_attribute]
pub fn create_cargofuzz_harness(
    _: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let output = transform_stream(input);
    proc_macro::TokenStream::from(output)
}

fn transform_stream(input: proc_macro::TokenStream) -> TokenStream {
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

    if let Typed(i) = fuzzing_harness.sig.inputs.iter_mut().next().unwrap() {
        if let Type::Path(typ) = &mut *i.ty {
            typ.path.segments.iter_mut().next().unwrap().ident = arg_struct.ident.clone();
            // Variable type
        }
    }

    fuzzing_harness.sig.ident = Ident::new(
        &("__fuzz_".to_owned() + &function.sig.ident.to_string()),
        Span::call_site(),
    );

    if let Stmt::Semi(Expr::Call(fn_call), _) = &mut fuzzing_harness.block.stmts[0] {
        if let Expr::Path(path) = &mut *fn_call.func {
            path.path.segments.iter_mut().next().unwrap().ident = function.sig.ident.clone();
        }
    }

    if let Stmt::Semi(Expr::Call(fn_call), _) = &mut fuzzing_harness.block.stmts[0] {
        let args = &mut fn_call.args;
        let default_field = args.pop().unwrap().into_value();
        if let Fields::Named(fields) = &arg_struct.fields {
            for field in fields.named.iter() {
                let mut new_field = default_field.clone();
                if let Expr::Field(ref mut f) = new_field {
                    if let Member::Named(name) = &mut f.member {
                        *name = field.ident.clone().unwrap();
                        //*name = field.ident.unwrap();
                        //dbg!(&name);
                    }
                }
                //dbg!(&field.ident);
                //dbg!(&new_field);
                args.push(new_field);
            }
            //dbg!(&args);
        }
        //dbg!(&default_field);
    }
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
