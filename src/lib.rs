use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::env;
use std::fs;
use syn::{ImplItem, ItemFn, ItemImpl, ItemStruct, Type};

mod crate_parse;
mod generate;

#[proc_macro_attribute]
pub fn create_cargofuzz_harness(
    attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let output = create_function_harness(TokenStream::from(attr), input);
    proc_macro::TokenStream::from(output)
}

#[proc_macro_attribute]
pub fn create_cargofuzz_impl_harness(
    attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let output = create_impl_harness(TokenStream::from(attr), input);
    proc_macro::TokenStream::from(output)
}

fn create_function_harness(attr: TokenStream, input: proc_macro::TokenStream) -> TokenStream {
    let function: ItemFn = syn::parse(input).expect("Failed to parse input");

    let fuzz_struct = generate::fuzz_struct(&function.sig, None).unwrap();
    let fuzz_function = generate::fuzz_function(&function.sig, None).unwrap();

    let crate_info = crate_parse::CrateInfo::from_root(
        &env::current_dir().expect("Failed to obtain project root dir"),
    )
    .expect("Failed to obtain crate info");

    let fuzz_dir_path = crate_info.fuzz_dir().expect("Failed to create fuzz dir");

    let crate_name_underscored = str::replace(crate_info.crate_name(), "-", "_"); // required for `extern crate`

    let crate_ident = format_ident!("{}", &crate_name_underscored);

    // Writing fuzzing harness to file
    let ident = crate_parse::construct_harness_ident(&function.sig.ident, None, &attr);

    let code = generate::fuzz_harness(&function.sig, None, &crate_ident, &attr);

    fs::write(
        fuzz_dir_path.join(String::new() + &ident + ".rs"),
        code.to_string(),
    )
    .expect("Failed to write fuzzing harness to fuzz/fuzz_targets");
    // TODO: Error handing

    crate_info
        .add_target_to_cargo_toml(&function.sig.ident, None, &attr)
        .expect("Failed to update Cargo.toml");

    quote!(
        #function
        #fuzz_struct
        #fuzz_function
    )
}

fn create_impl_harness(attr: TokenStream, input: proc_macro::TokenStream) -> TokenStream {
    let implementation: ItemImpl = syn::parse(input).expect("Failed to parse input");
    // Checking that the implementation meets the requirements
    assert_eq!(
        implementation.unsafety, None,
        "unsafe traits can not be fuzzed automatically."
    );
    //assert!(
    //<Generic type parameter>,
    //"Generics are not currently supported."
    //);
    //TODO: tests
    let crate_info = crate_parse::CrateInfo::from_root(
        &env::current_dir().expect("Failed to obtain project root dir"),
    )
    .expect("Failed to obtain crate info");

    let fuzz_dir_path = crate_info.fuzz_dir().expect("Failed to create fuzz dir");

    let crate_name_underscored = str::replace(crate_info.crate_name(), "-", "_"); // required for `extern crate`

    let crate_ident = format_ident!("{}", &crate_name_underscored);

    let mut fuzz_structs = Vec::<ItemStruct>::new();
    let mut fuzz_functions = Vec::<ItemFn>::new();

    for item in &implementation.items {
        if let ImplItem::Method(method) = item {
            let fuzz_struct_result =
                generate::fuzz_struct(&method.sig, Some(&implementation.self_ty));
            let fuzz_function_result =
                generate::fuzz_function(&method.sig, Some(&implementation.self_ty));

            match (fuzz_struct_result, fuzz_function_result) {
                (Ok(fuzz_struct), Ok(fuzz_function)) => {
                    // Writing fuzzing harness to file
                    let code = generate::fuzz_harness(
                        &method.sig,
                        Some(&implementation.self_ty),
                        &crate_ident,
                        &attr,
                    );
                    let filename = if let Type::Path(ref path) = *implementation.self_ty {
                        format!(
                            "{}_{}.rs",
                            &(path.path.segments.iter().next().unwrap().ident).to_string(),
                            &method.sig.ident.to_string()
                        )
                    } else {
                        panic!("Complex self type.")
                    };

                    fs::write(fuzz_dir_path.join(filename), code.to_string())
                        .expect("Failed to write fuzzing harness to fuzz/fuzz_targets");
                    // TODO: Error handing

                    crate_info
                        .add_target_to_cargo_toml(
                            &method.sig.ident,
                            Some(&implementation.self_ty),
                            &attr,
                        )
                        .expect("Failed to update Cargo.toml");
                    fuzz_structs.push(fuzz_struct);
                    fuzz_functions.push(fuzz_function);
                }
                (Ok(_), Err(error)) => {
                    eprintln!("Skipping method {}, due to:\n{}", &method.sig.ident, error);
                    continue;
                }
                (Err(error), Ok(_)) => {
                    eprintln!("Skipping method {}, due to\n{}", &method.sig.ident, error);
                    continue;
                }
                (Err(_), Err(function_error)) => {
                    eprintln!(
                        "Skipping method {}, due to:\n{}",
                        &method.sig.ident, function_error
                    );
                    continue;
                }
            }
        }
    }

    quote!(
        #implementation
        #(#fuzz_structs)*
        #(#fuzz_functions)*
    )
}
