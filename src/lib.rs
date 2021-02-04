extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;
use proc_macro2::TokenStream;
use std::env;
use std::fs;
use syn::__private::Span;
use syn::{Ident, ItemFn};

mod crate_parse;
mod generate;

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

    let fuzz_struct = generate::fuzz_struct(&function.sig, None);
    let fuzz_function = generate::fuzz_function(&function.sig, None);

    let crate_info = crate_parse::CrateInfo::from_root(
        &env::current_dir().expect("Failed to obtain project root dir"),
    )
    .expect("Failed to obtain crate info");

    let fuzz_dir_path = crate_info.fuzz_dir().expect("Failed to create fuzz dir");

    let crate_name_underscored = str::replace(crate_info.crate_name(), "-", "_"); // required for `extern crate`

    let crate_ident = Ident::new(&crate_name_underscored, Span::call_site());

    // Writing fuzzing harness to file
    let code = generate::fuzz_harness(&function.sig, &crate_ident, attr);

    fs::write(
        fuzz_dir_path.join(String::new() + &function.sig.ident.to_string() + ".rs"),
        code.to_string(),
    )
    .expect("Failed to write fuzzing harness to fuzz/fuzz_targets");
    // TODO: Error handing

    crate_info
        .write_cargo_toml(&function.sig.ident)
        .expect("Failed to update Cargo.toml");

    quote!(
        #function
      #fuzz_struct
    #fuzz_function
    )
}
