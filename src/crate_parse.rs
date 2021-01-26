extern crate fs3;
use fs3::FileExt;
use proc_macro2::TokenStream;
use std::fs::{File, OpenOptions};
use std::io::{Error, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use syn::Ident;

use cargo_toml::Value::String as TomlString;
use cargo_toml::Value::Table as TomlTable;
use toml::value::Value as TomlValue;

#[derive(Clone)]
pub struct CrateInfo {
    crate_root: PathBuf,
    crate_name: String,
}

impl CrateInfo {
    pub fn from_root(path: &Path) -> Option<CrateInfo> {
        if !path.is_dir() {
            return None;
        }
        let mut entries = path.read_dir().ok()?;
        let cargo_toml_present = entries.any(|result| {
            result
                .map(|entry| entry.file_name().to_string_lossy() == "Cargo.toml")
                .unwrap_or(false)
        });
        if cargo_toml_present {
            if let Some(crate_name) = parse_crate_name(&path.join("Cargo.toml")) {
                Some(CrateInfo {
                    crate_root: path.to_path_buf(),
                    crate_name,
                })
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn crate_name(&self) -> &str {
        &self.crate_name
    }

    pub fn fuzz_dir(&self) -> std::io::Result<PathBuf> {
        let fuzz_dir_path = self.crate_root.join("fuzz");
        let fuzz_targets_dir_path = self.crate_root.join("fuzz").join("fuzz_targets");
        match std::fs::create_dir(&fuzz_dir_path) {
            Ok(_) => match std::fs::create_dir(&fuzz_targets_dir_path) {
                Ok(_) => Ok(fuzz_targets_dir_path),
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::AlreadyExists {
                        Ok(fuzz_targets_dir_path)
                    } else {
                        Err(e)
                    }
                }
            },
            Err(e) => {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    match std::fs::create_dir(&fuzz_targets_dir_path) {
                        Ok(_) => Ok(fuzz_targets_dir_path),
                        Err(e) => {
                            if e.kind() == std::io::ErrorKind::AlreadyExists {
                                Ok(fuzz_targets_dir_path)
                            } else {
                                Err(e)
                            }
                        }
                    }
                } else {
                    Err(e)
                }
            }
        }
    }

    pub fn write_cargo_toml(&self, function: &Ident) -> Result<(), Error> {
        match OpenOptions::new().write(true).create_new(true).open(
            self.fuzz_dir()
                .unwrap()
                .parent()
                .unwrap()
                .join("Cargo.toml"),
        ) {
            Ok(mut file) => {
                file.lock_exclusive()?;

                write!(
                    file,
                    "{}{}{}",
                    &CARGO_TOML_TEMPLATE_PREFIX,
                    &self.crate_name(),
                    &CARGO_TOML_TEMPLATE_POSTFIX
                )?;

                write!(
                    file,
                    "{}{}{}{}{}",
                    &TARGET_TEMPLATE_PREFIX,
                    &function.to_string(),
                    &TARGET_TEMPLATE_INFIX,
                    &function.to_string(),
                    &TARGET_TEMPLATE_POSTFIX
                )?;
                file.flush()?;

                file.unlock()?;
                Ok(())
            }
            Err(err) => {
                if err.kind() == ErrorKind::AlreadyExists {
                    let mut file = OpenOptions::new()
                        .read(true)
                        .write(true)
                        .append(true)
                        .open(
                            self.fuzz_dir()
                                .unwrap()
                                .parent()
                                .unwrap()
                                .join("Cargo.toml"),
                        )?;
                    file.lock_exclusive()?;
                    let mut buffer = String::new();
                    file.read_to_string(&mut buffer)?;
                    let parts = buffer.split("\n\n");
                    let fuzz_target_exists = parts
                        .skip(5)
                        .map(|item| {
                            if let TomlTable(table) =
                                &item.lines().nth(1).unwrap().parse::<TomlValue>().unwrap()
                            {
                                if let TomlString(s) = table.get("name").unwrap() {
                                    s == &function.to_string()
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        })
                        .fold(false, |acc, x| acc | x);
                    if !fuzz_target_exists {
                        write!(
                            file,
                            "{}{}{}{}{}",
                            &TARGET_TEMPLATE_PREFIX,
                            &function.to_string(),
                            &TARGET_TEMPLATE_INFIX,
                            &function.to_string(),
                            &TARGET_TEMPLATE_POSTFIX
                        )?;
                        file.flush()?;
                    }

                    file.unlock()?;
                    Ok(())
                } else {
                    Err(err)
                }
            }
        }
    }
}

fn parse_crate_name(cargo_toml_path: &Path) -> Option<String> {
    let cargo_bytes = {
        let mut cargo_bytes = Vec::new();
        File::open(cargo_toml_path)
            .ok()?
            .read_to_end(&mut cargo_bytes)
            .ok()?;
        cargo_bytes
    };

    let cargo_toml: TomlValue = toml::from_slice(&cargo_bytes).ok()?;

    Some(
        cargo_toml
            .get("package")?
            .get("name")?
            .as_str()?
            .to_string(),
    )
}

pub fn compose_fn_invocation(
    func: &Ident,
    ty: &Ident,
    crate_ident: &Ident,
    attr: TokenStream,
) -> String {
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

            fuzz_target!(|input: #path #ty| {
            #path #func (input);
            });
        );

    code.to_string()
}

const CARGO_TOML_TEMPLATE_PREFIX: &str = r#"[package]
name = ""#;
const CARGO_TOML_TEMPLATE_POSTFIX: &str = r#"-fuzz"
version = "0.0.0"
authors = ["Automatically generated"]
publish = false
edition = "2018"

[package.metadata]
cargo-fuzz = true

[dependencies]
libfuzzer-sys = "0.3"

[dependencies.test-lib]
path = ".."

# Prevent this from interfering with workspaces
[workspace]
members = ["."]
"#;
const TARGET_TEMPLATE_PREFIX: &str = r#"
[[bin]]
name = ""#;
const TARGET_TEMPLATE_INFIX: &str = r#""
path = "fuzz_targets/"#;
const TARGET_TEMPLATE_POSTFIX: &str = r#".rs"
test = false
doc = false
"#;
