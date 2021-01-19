use std::fs::File;
use std::io;
use std::io::{BufWriter, Error, ErrorKind, Write};
use std::path::Path;
use std::process::Command;

use auto_fuzz_test::CrateInfo;
use auto_fuzz_test::FnVisitor;
use quote::quote;
use syn;
use syn::token::{Async, Unsafe};
use syn::visit::Visit;
use syn::{FnArg, punctuated::Punctuated, token::Comma, Ident, Type};

fn main() -> io::Result<()> {
    // Make sure at least one path was provided.
    if std::env::args().skip(1).next().is_none() {
        eprintln!(
            "Usage: {} path [path ...]",
            std::env::args().next().unwrap()
        );
        std::process::exit(2);
    }

    // Find the crate from the passed argument.
    let crate_info: CrateInfo = {
        let rust_info;
        let path_str = std::env::args()
            .skip(1)
            .next()
            .expect("crate path required");
        let path = Path::new(&path_str);
        if path.is_dir() {
            // Presume the path is a crate, and add it.
            if let Some(info) = CrateInfo::from_root(path) {
                rust_info = info;
            } else {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!("Input path '{}': cannot find containing crate", &path_str),
                ));
            }
        } else if path.is_file() {
            // Presume the file is a member of a crate, and add the crate.
            if let Some(info) = CrateInfo::from_inner_source_file(path) {
                rust_info = info;
            } else {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!("Input path '{}': cannot find containing crate", &path_str),
                ));
            }
        } else {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("Input path '{}': not found, skipping", &path_str),
            ));
        }
        rust_info
    };

    // Read the entire file into a string after parsing it with rustc
    let rustc_output = Command::new("cargo")
        .arg("+nightly")
        .arg("rustc")
        .arg("--profile=check")
        .arg("--")
        .arg("-Z")
        .arg("unpretty=hir")
        .current_dir(&crate_info.crate_root())
        .output()
        .expect("Failed to execute rustc");

    if !rustc_output.status.success() {
        // Unfortunately, this way of printing does not preserve colors
        eprintln!("{}", String::from_utf8(rustc_output.stderr).unwrap());
        panic!("rustc failed");
    }

    let code_str: String =
        String::from_utf8(rustc_output.stdout).expect("Failed to parse rustc output: not UTF-8");

    // Parse the file into a syntax tree.
    let syntax_tree: syn::File = syn::parse_str(&code_str).expect(&format!(
        "Crate '{:?}': not Rust code, does the crate compile?",
        &crate_info.crate_name()
    ));

    // Generate a visitor that will gather what information we need.
    // Note that function write_fn_invocation doesn't care about some of the parameters, so we
    // throw them away here.
    let callback = |this: Option<&Type>,
                    ident: &Ident,
                    inputs: &Punctuated<FnArg,_>,
                    unsafety: &Option<Unsafe>,
                    asyncness: &Option<Async>,
                    crate_info: &CrateInfo| {
        // Unsafe functions cannot have fuzzing harnesses generated automatically, since it's
        // valid for them to crash for some inputs.
        // Async functions are simply not supported for now.
        if unsafety.is_none() && asyncness.is_none() {
            let mut fn_inv = BufWriter::new(
                File::create(
                    crate_info
                        .fuzz_dir()
                        .unwrap()
                        .join(format!("{}.rs", &ident.to_string())),
                )
                .unwrap(),
            );
            println!("{:?}", ident);
            write_fn_invocation(&mut fn_inv, this, ident, inputs, crate_info.crate_name()).unwrap();
        }
    };

    FnVisitor {
        callback: Box::new(callback),
        context: crate_info,
    }
    .visit_file(&syntax_tree);
    Ok(())
}

fn write_fn_invocation(
    mut result: &mut dyn Write,
    this: Option<&Type>,
    ident: &Ident,
    inputs: &Punctuated<FnArg,Comma>,
    crate_name: &str,
) -> Result<(), std::io::Error> {
    // split the template around where the generated functions go
    let (prefix, suffix) = {
        let mut template_split = FUZZING_HARNESS_TEMPLATE.split("{0}");
        (
            template_split.next().unwrap(),
            template_split.next().expect("need '{0}' in template"),
        )
    };

    // insert the crate name into the template
    let crate_name_undescored = str::replace(crate_name, "-", "_"); // required for `extern crate`
    let (crate_prefix, crate_suffix) = {
        let mut template_split = prefix.split("{crate_name}");
        (
            template_split.next().unwrap(),
            template_split
                .next()
                .expect("need '{crate_name}' in template"),
        )
    };

    // write the beginning of the template up to the {0} insertion point
    write!(
        &mut result,
        "{}{}{}",
        crate_prefix, crate_name_undescored, crate_suffix
    )?;

    // print creation of variables
    writeln!(
        &mut result,
        "    // create input data for specific function from random bytes"
    )?;
    if let Some(self_type) = &this {
        writeln!(
            &mut result,
            "    let fuzz_self = {}::arbitrary(&mut read_rng);",
            quote!(#self_type)
        )?;
    }
    let mut arg_numbers: Vec<usize> = Vec::new();
    for (num, a) in inputs.iter().enumerate() {
        if let FnArg::Typed(a) = a {
            let pat = &*a.pat;
            let arg_type = &*a.ty;
            writeln!(
                &mut result,
                "    let fuzz_arg_{} = {}::arbitrary(&mut read_rng); // {}",
                num,
                quote!(#arg_type),
                quote!(#pat)
            )?;
            arg_numbers.push(num);
        }
    }
    // print actual invocation of the function
    write!(&mut result, "\n    // invoke function\n    ")?;
    if this.is_some() {
        write!(&mut result, "fuzz_self.")?;
    }
    write!(&mut result, "{}(", ident)?;
    let mut is_first_argument = true;
    for arg_num in arg_numbers {
        if !is_first_argument {
            write!(&mut result, ",")?
        };
        is_first_argument = false;
        write!(&mut result, "fuzz_arg_{}", arg_num)?;
    }
    writeln!(&mut result, ");")?;

    // print the end of the template
    write!(&mut result, "{}", suffix)?;

    Ok(())
}

const FUZZING_HARNESS_TEMPLATE: &str = "// Autogenerated fuzzing harness.
#![no_main]
#[macro_use] extern crate libfuzzer_sys;
extern crate rand;
extern crate quickcheck;
extern crate {crate_name};

use std::io::prelude::*;
use quickcheck::Arbitrary;

fuzz_target!(|raw_input: &[u8]| {
    // input preparation for QuickCheck, not specific to the fuzzed function
    let input_cursor = std::io::Cursor::new(raw_input);
    let read_rng = rand::rngs::adapter::ReadRng::new(input_cursor);
    let mut read_rng = quickcheck::StdGen::new(read_rng, std::usize::MAX);

{0}
});
";
