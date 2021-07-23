use fs3::FileExt;
use proc_macro2::TokenStream;
use std::fs::{File, OpenOptions};
use std::io::{Error, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use syn::{Ident, Type};

use cargo_toml::Value::String as TomlString;
use cargo_toml::Value::Table as TomlTable;
use toml::value::Value as TomlValue;

#[derive(Clone)]
#[cfg_attr(test, derive(Debug))]
pub struct CrateInfo {
    crate_root: PathBuf,
    crate_name: String,
}

impl CrateInfo {
    pub fn from_root(path: &Path) -> Option<CrateInfo> {
        let cargo_toml_path = path.join("Cargo.toml");
        if cargo_toml_path.exists() {
            CrateInfo::parse_crate_name(&cargo_toml_path).map(|crate_name| CrateInfo {
                crate_root: path.to_path_buf(),
                crate_name,
            })
        } else {
            None
        }
    }

    pub fn crate_name(&self) -> &str {
        &self.crate_name
    }

    pub fn fuzz_dir(&self) -> std::io::Result<PathBuf> {
        let fuzz_dir_path = self.crate_root.join("fuzz").join("fuzz_targets");
        match std::fs::create_dir_all(&fuzz_dir_path) {
            Ok(_) => Ok(fuzz_dir_path),
            Err(e) => {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    Ok(fuzz_dir_path)
                } else {
                    Err(e)
                }
            }
        }
    }

    pub fn add_target_to_cargo_toml(
        &self,
        function: &Ident,
        impl_type: Option<&Type>,
        module_path: &TokenStream,
    ) -> Result<(), Error> {
        let ident = construct_harness_ident(function, impl_type, module_path);

        let cargo_toml_path = self.fuzz_dir()?.parent().unwrap().join("Cargo.toml");
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&cargo_toml_path)
        {
            Ok(mut file) => {
                file.lock_exclusive()?;

                write!(
                    file,
                    "{}{}{}{}{}",
                    &CrateInfo::CARGO_TOML_TEMPLATE_PREFIX,
                    &self.crate_name(),
                    &CrateInfo::CARGO_TOML_TEMPLATE_INFIX,
                    &self.crate_name(),
                    &CrateInfo::CARGO_TOML_TEMPLATE_POSTFIX
                )?;

                write!(
                    file,
                    "{}{}{}{}{}",
                    &CrateInfo::TARGET_TEMPLATE_PREFIX,
                    &ident,
                    &CrateInfo::TARGET_TEMPLATE_INFIX,
                    &ident,
                    &CrateInfo::TARGET_TEMPLATE_POSTFIX
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
                        .open(&cargo_toml_path)?;
                    file.lock_exclusive()?;

                    // Checking, that we are not going to duplicate [[bin]] targets
                    let mut buffer = String::new();
                    file.read_to_string(&mut buffer)?;
                    // Generated Cargo.toml consists of several sections splitted by '\n\n'
                    // Here we split and skip the first 5 of them: [package], [package.metadata], [dependencies], [dependencies.<crate_name>] and [workspace]
                    let parts = buffer.split("\n\n");
                    let fuzz_target_exists = parts.skip(5).any(|item| {
                        // In this closure we extract target ident and compare it with the one we want to add.
                        // If anything goes wrong, this closure returns `false`.
                        item.lines().nth(1).map_or(false, |target_name_line| {
                            if let Ok(TomlTable(table)) = &target_name_line.parse::<TomlValue>() {
                                if let Some(TomlString(s)) = table.get("name") {
                                    s == &ident
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        })
                    });
                    if !fuzz_target_exists {
                        write!(
                            file,
                            "{}{}{}{}{}",
                            &CrateInfo::TARGET_TEMPLATE_PREFIX,
                            &ident,
                            &CrateInfo::TARGET_TEMPLATE_INFIX,
                            &ident,
                            &CrateInfo::TARGET_TEMPLATE_POSTFIX
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

    const CARGO_TOML_TEMPLATE_PREFIX: &'static str = r#"[package]
name = ""#;

    const CARGO_TOML_TEMPLATE_INFIX: &'static str = r#"-fuzz"
version = "0.0.0"
authors = ["Automatically generated"]
publish = false
edition = "2018"

[package.metadata]
cargo-fuzz = true

[dependencies]
libfuzzer-sys = "0.4"

[dependencies."#;

    const CARGO_TOML_TEMPLATE_POSTFIX: &'static str = r#"]
path = ".."

# Prevent this from interfering with workspaces
[workspace]
members = ["."]
"#;

    const TARGET_TEMPLATE_PREFIX: &'static str = r#"
[[bin]]
name = ""#;

    const TARGET_TEMPLATE_INFIX: &'static str = r#""
path = "fuzz_targets/"#;

    const TARGET_TEMPLATE_POSTFIX: &'static str = r#".rs"
test = false
doc = false
"#;
}

pub fn construct_harness_ident(
    function: &Ident,
    impl_type: Option<&Type>,
    module_path: &TokenStream,
) -> String {
    // Functions in different modules and/or in different impl's can have identical names. To
    // avoid collisions, this function adds module path and impl type to target filenames.
    match impl_type {
        Some(typ) => {
            if let Type::Path(path) = typ {
                if module_path.is_empty() {
                    format!(
                        "{}_{}",
                        &(path.path.segments.iter().next().unwrap().ident).to_string(),
                        &function.to_string()
                    )
                } else {
                    format!(
                        "{}__{}_{}",
                        module_path.to_string().replace(" :: ", "__"),
                        &(path.path.segments.iter().next().unwrap().ident).to_string(),
                        &function.to_string()
                    )
                }
            } else {
                unimplemented!("Complex self types.")
            }
        }
        None => {
            if module_path.is_empty() {
                function.to_string()
            } else {
                format!(
                    "{}__{}",
                    module_path.to_string().replace(" :: ", "__"),
                    &function.to_string()
                )
            }
        }
    }
}

#[cfg(test)]
impl PartialEq for CrateInfo {
    fn eq(&self, other: &Self) -> bool {
        self.crate_name == other.crate_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use quote::{format_ident, quote};
    use std::fs::File;
    use std::io::Write;
    use std::thread;
    use syn::ItemImpl;
    use tempfile::tempdir;

    #[test]
    fn no_cargo_toml() {
        let dir = tempdir().expect("Could not create a tempdir fot test");
        assert_eq!(CrateInfo::from_root(dir.path()), None);
    }

    #[test]
    fn empty_cargo_toml() {
        let dir = tempdir().expect("Could not create tempdir fot test");
        let cargo_toml_path = dir.path().join("Cargo.toml");
        File::create(&cargo_toml_path).expect("Could not create Cargo.toml fot test");

        assert_eq!(CrateInfo::parse_crate_name(&cargo_toml_path), None);
    }

    #[test]
    fn parse_valid_cargo_toml() {
        let dir = tempdir().expect("Could not create tempdir fot test");
        let cargo_toml_path = dir.path().join("Cargo.toml");
        let mut cargo_toml =
            File::create(&cargo_toml_path).expect("Could not create Cargo.toml fot test");
        writeln!(cargo_toml, "{}", VALID_CARGO_TOML)
            .expect("Could not write valid data to Cargo.toml fot test");

        assert_eq!(
            CrateInfo::parse_crate_name(&cargo_toml_path),
            Some("test-lib".to_string())
        );
    }

    #[test]
    fn create_dirs() {
        let dir = tempdir().expect("Could not create tempdir fot test");
        let cargo_toml_path = dir.path().join("Cargo.toml");
        let mut cargo_toml =
            File::create(&cargo_toml_path).expect("Could not create Cargo.toml fot test");
        writeln!(cargo_toml, "{}", VALID_CARGO_TOML)
            .expect("Could not write valid data to Cargo.toml fot test");
        let crate_info = CrateInfo::from_root(dir.path()).unwrap();

        assert_eq!(
            crate_info.fuzz_dir().unwrap(),
            crate_info.crate_root.join("fuzz").join("fuzz_targets")
        );
    }

    #[test]
    fn write_cargo_nomodule_noimpl() {
        let dir = tempdir().expect("Could not create tempdir fot test");
        let cargo_toml_path = dir.path().join("Cargo.toml");
        let mut cargo_toml =
            File::create(&cargo_toml_path).expect("Could not create Cargo.toml fot test");
        writeln!(cargo_toml, "{}", VALID_CARGO_TOML)
            .expect("Could not write valid data to Cargo.toml fot test");
        let crate_info = CrateInfo::from_root(dir.path()).unwrap();

        let ident = format_ident!("foo");
        let module = TokenStream::new();

        crate_info
            .add_target_to_cargo_toml(&ident, None, &module)
            .unwrap();

        crate_info
            .add_target_to_cargo_toml(&ident, None, &module)
            .unwrap();

        let mut cargo_toml = File::open(dir.path().join("fuzz").join("Cargo.toml")).unwrap();
        let mut cargo_contents = String::new();
        cargo_toml.read_to_string(&mut cargo_contents).unwrap();
        assert_eq!(cargo_contents, VALID_GENERATED_CARGO_TOML_NOMODULE_NOIMPL);
    }

    #[test]
    fn write_cargo_module_noimpl() {
        let dir = tempdir().expect("Could not create tempdir fot test");
        let cargo_toml_path = dir.path().join("Cargo.toml");
        let mut cargo_toml =
            File::create(&cargo_toml_path).expect("Could not create Cargo.toml fot test");
        writeln!(cargo_toml, "{}", VALID_CARGO_TOML)
            .expect("Could not write valid data to Cargo.toml fot test");
        let crate_info = CrateInfo::from_root(dir.path()).unwrap();

        let ident = format_ident!("cat");
        let module = quote!(foo::bar::dog);

        crate_info
            .add_target_to_cargo_toml(&ident, None, &module)
            .unwrap();

        crate_info
            .add_target_to_cargo_toml(&ident, None, &module)
            .unwrap();

        let mut cargo_toml = File::open(dir.path().join("fuzz").join("Cargo.toml")).unwrap();
        let mut cargo_contents = String::new();
        cargo_toml.read_to_string(&mut cargo_contents).unwrap();
        assert_eq!(cargo_contents, VALID_GENERATED_CARGO_TOML_MODULE_NOIMPL);
    }

    #[test]
    fn write_cargo_nomodule_impl() {
        let dir = tempdir().expect("Could not create tempdir fot test");
        let cargo_toml_path = dir.path().join("Cargo.toml");
        let mut cargo_toml =
            File::create(&cargo_toml_path).expect("Could not create Cargo.toml fot test");
        writeln!(cargo_toml, "{}", VALID_CARGO_TOML)
            .expect("Could not write valid data to Cargo.toml fot test");
        let crate_info = CrateInfo::from_root(dir.path()).unwrap();

        let ident = format_ident!("foo");
        let implementation: ItemImpl = syn::parse2(quote! {
            impl TestStruct {
            }
        })
        .unwrap();
        let module = TokenStream::new();

        crate_info
            .add_target_to_cargo_toml(&ident, Some(&implementation.self_ty), &module)
            .unwrap();
        crate_info
            .add_target_to_cargo_toml(&ident, Some(&implementation.self_ty), &module)
            .unwrap();

        let mut cargo_toml = File::open(dir.path().join("fuzz").join("Cargo.toml")).unwrap();
        let mut cargo_contents = String::new();
        cargo_toml.read_to_string(&mut cargo_contents).unwrap();
        assert_eq!(cargo_contents, VALID_GENERATED_CARGO_TOML_NOMODULE_IMPL);
    }

    #[test]
    fn write_cargo_module_impl() {
        let dir = tempdir().expect("Could not create tempdir fot test");
        let cargo_toml_path = dir.path().join("Cargo.toml");
        let mut cargo_toml =
            File::create(&cargo_toml_path).expect("Could not create Cargo.toml fot test");
        writeln!(cargo_toml, "{}", VALID_CARGO_TOML)
            .expect("Could not write valid data to Cargo.toml fot test");
        let crate_info = CrateInfo::from_root(dir.path()).unwrap();

        let ident = format_ident!("cat");
        let implementation: ItemImpl = syn::parse2(quote! {
            impl TestStruct {
            }
        })
        .unwrap();
        let module = quote!(foo::bar::dog);

        crate_info
            .add_target_to_cargo_toml(&ident, Some(&implementation.self_ty), &module)
            .unwrap();
        crate_info
            .add_target_to_cargo_toml(&ident, Some(&implementation.self_ty), &module)
            .unwrap();

        let mut cargo_toml = File::open(dir.path().join("fuzz").join("Cargo.toml")).unwrap();
        let mut cargo_contents = String::new();
        cargo_toml.read_to_string(&mut cargo_contents).unwrap();
        assert_eq!(cargo_contents, VALID_GENERATED_CARGO_TOML_MODULE_IMPL);
    }

    #[test]
    fn write_cargo_concurently() {
        let dir = tempdir().expect("Could not create tempdir fot test");
        let cargo_toml_path = dir.path().join("Cargo.toml");
        let mut cargo_toml =
            File::create(&cargo_toml_path).expect("Could not create Cargo.toml fot test");
        writeln!(cargo_toml, "{}", VALID_CARGO_TOML)
            .expect("Could not write valid data to Cargo.toml fot test");
        let crate_info = CrateInfo::from_root(dir.path()).unwrap();

        // Here comments with numbers are used to enumerate different function idents later they
        // will be used in different threads in different order
        let mut idents_needed = vec![
            "foo".to_string(),                      // 1
            "bar".to_string(),                      // 2
            "foo__foo".to_string(),                 // 3
            "foo__bar".to_string(),                 // 4
            "foo__bar__dog__cat".to_string(),       // 5
            "foo__bar__dog__dog".to_string(),       // 6
            "TestStruct_foo".to_string(),           // 7
            "TestStruct_bar".to_string(),           // 8
            "foo__TestStruct_foo".to_string(),      // 9
            "foo__TestStruct_bar".to_string(),      // 10
            "foo__bar__TestStruct_foo".to_string(), // 11
            "foo__bar__TestStruct_bar".to_string(), // 12
        ];

        idents_needed.sort();

        let crate_info_thread_1 = crate_info.clone();
        let handle_1 = thread::spawn(move || {
            // 1
            let ident = format_ident!("foo");
            let module = quote!();
            crate_info_thread_1
                .add_target_to_cargo_toml(&ident, None, &module)
                .unwrap();

            // 3
            let ident = format_ident!("foo");
            let module = quote!(foo);
            crate_info_thread_1
                .add_target_to_cargo_toml(&ident, None, &module)
                .unwrap();

            // 5
            let ident = format_ident!("cat");
            let module = quote!(foo::bar::dog);
            crate_info_thread_1
                .add_target_to_cargo_toml(&ident, None, &module)
                .unwrap();

            // 7
            let ident = format_ident!("bar");
            let module = quote!();
            let implementation: ItemImpl = syn::parse2(quote! {
                impl TestStruct {
                }
            })
            .unwrap();
            crate_info_thread_1
                .add_target_to_cargo_toml(&ident, Some(&implementation.self_ty), &module)
                .unwrap();

            // 3
            let ident = format_ident!("foo");
            let module = quote!(foo);
            crate_info_thread_1
                .add_target_to_cargo_toml(&ident, None, &module)
                .unwrap();

            // 7
            let ident = format_ident!("foo");
            let module = quote!();
            let implementation: ItemImpl = syn::parse2(quote! {
                impl TestStruct {
                }
            })
            .unwrap();
            crate_info_thread_1
                .add_target_to_cargo_toml(&ident, Some(&implementation.self_ty), &module)
                .unwrap();

            // 9
            let ident = format_ident!("foo");
            let module = quote!(foo);
            let implementation: ItemImpl = syn::parse2(quote! {
                impl TestStruct {
                }
            })
            .unwrap();
            crate_info_thread_1
                .add_target_to_cargo_toml(&ident, Some(&implementation.self_ty), &module)
                .unwrap();

            // 11
            let ident = format_ident!("foo");
            let module = quote!(foo::bar);
            let implementation: ItemImpl = syn::parse2(quote! {
                impl TestStruct {
                }
            })
            .unwrap();
            crate_info_thread_1
                .add_target_to_cargo_toml(&ident, Some(&implementation.self_ty), &module)
                .unwrap();
        });

        let crate_info_thread_2 = crate_info.clone();
        let handle_2 = thread::spawn(move || {
            // 6
            let ident = format_ident!("dog");
            let module = quote!(foo::bar::dog);
            crate_info_thread_2
                .add_target_to_cargo_toml(&ident, None, &module)
                .unwrap();

            // 8
            let ident = format_ident!("bar");
            let module = quote!();
            let implementation: ItemImpl = syn::parse2(quote! {
                impl TestStruct {
                }
            })
            .unwrap();
            crate_info_thread_2
                .add_target_to_cargo_toml(&ident, Some(&implementation.self_ty), &module)
                .unwrap();

            // 10
            let ident = format_ident!("bar");
            let module = quote!(foo);
            let implementation: ItemImpl = syn::parse2(quote! {
                impl TestStruct {
                }
            })
            .unwrap();
            crate_info_thread_2
                .add_target_to_cargo_toml(&ident, Some(&implementation.self_ty), &module)
                .unwrap();

            // 12
            let ident = format_ident!("bar");
            let module = quote!(foo::bar);
            let implementation: ItemImpl = syn::parse2(quote! {
                impl TestStruct {
                }
            })
            .unwrap();
            crate_info_thread_2
                .add_target_to_cargo_toml(&ident, Some(&implementation.self_ty), &module)
                .unwrap();

            // 6
            let ident = format_ident!("dog");
            let module = quote!(foo::bar::dog);
            crate_info_thread_2
                .add_target_to_cargo_toml(&ident, None, &module)
                .unwrap();

            // 10
            let ident = format_ident!("bar");
            let module = quote!(foo);
            let implementation: ItemImpl = syn::parse2(quote! {
                impl TestStruct {
                }
            })
            .unwrap();
            crate_info_thread_2
                .add_target_to_cargo_toml(&ident, Some(&implementation.self_ty), &module)
                .unwrap();

            // 2
            let ident = format_ident!("bar");
            let module = quote!();
            crate_info_thread_2
                .add_target_to_cargo_toml(&ident, None, &module)
                .unwrap();

            // 4
            let ident = format_ident!("bar");
            let module = quote!(foo);
            crate_info_thread_2
                .add_target_to_cargo_toml(&ident, None, &module)
                .unwrap();
        });

        {
            // 9
            let ident = format_ident!("foo");
            let module = quote!(foo);
            let implementation: ItemImpl = syn::parse2(quote! {
                impl TestStruct {
                }
            })
            .unwrap();
            crate_info
                .add_target_to_cargo_toml(&ident, Some(&implementation.self_ty), &module)
                .unwrap();

            // 2
            let ident = format_ident!("bar");
            let module = quote!();
            crate_info
                .add_target_to_cargo_toml(&ident, None, &module)
                .unwrap();

            // 4
            let ident = format_ident!("bar");
            let module = quote!(foo);
            crate_info
                .add_target_to_cargo_toml(&ident, None, &module)
                .unwrap();

            // 11
            let ident = format_ident!("foo");
            let module = quote!(foo::bar);
            let implementation: ItemImpl = syn::parse2(quote! {
                impl TestStruct {
                }
            })
            .unwrap();
            crate_info
                .add_target_to_cargo_toml(&ident, Some(&implementation.self_ty), &module)
                .unwrap();
        }

        handle_1.join().unwrap();
        handle_2.join().unwrap();

        let mut cargo_toml = File::open(dir.path().join("fuzz").join("Cargo.toml")).unwrap();
        let mut cargo_contents = String::new();
        cargo_toml.read_to_string(&mut cargo_contents).unwrap();

        let parts = cargo_contents.split("\n\n");
        let mut idents = parts
            .skip(5)
            .map(|item| {
                if let TomlTable(table) =
                    &item.lines().nth(1).unwrap().parse::<TomlValue>().unwrap()
                {
                    if let TomlString(s) = table.get("name").unwrap() {
                        Some(s.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .fold(Vec::<String>::new(), |mut acc, x| {
                if let Some(s) = x {
                    acc.push(s);
                }
                acc
            });

        idents.sort();

        assert_eq!(idents_needed, idents);
    }

    const VALID_CARGO_TOML: &str = r#"[package]
name = "test-lib"
version = "0.1.0"
authors = ["<test>"]
edition = "2018"

[dependencies]
auto-fuzz-test = { path = "../"  }
arbitrary = { version = "1", features = ["derive"]  }
"#;

    const VALID_GENERATED_CARGO_TOML_NOMODULE_NOIMPL: &str = r#"[package]
name = "test-lib-fuzz"
version = "0.0.0"
authors = ["Automatically generated"]
publish = false
edition = "2018"

[package.metadata]
cargo-fuzz = true

[dependencies]
libfuzzer-sys = "0.4"

[dependencies.test-lib]
path = ".."

# Prevent this from interfering with workspaces
[workspace]
members = ["."]

[[bin]]
name = "foo"
path = "fuzz_targets/foo.rs"
test = false
doc = false
"#;

    const VALID_GENERATED_CARGO_TOML_MODULE_NOIMPL: &str = r#"[package]
name = "test-lib-fuzz"
version = "0.0.0"
authors = ["Automatically generated"]
publish = false
edition = "2018"

[package.metadata]
cargo-fuzz = true

[dependencies]
libfuzzer-sys = "0.4"

[dependencies.test-lib]
path = ".."

# Prevent this from interfering with workspaces
[workspace]
members = ["."]

[[bin]]
name = "foo__bar__dog__cat"
path = "fuzz_targets/foo__bar__dog__cat.rs"
test = false
doc = false
"#;

    const VALID_GENERATED_CARGO_TOML_NOMODULE_IMPL: &str = r#"[package]
name = "test-lib-fuzz"
version = "0.0.0"
authors = ["Automatically generated"]
publish = false
edition = "2018"

[package.metadata]
cargo-fuzz = true

[dependencies]
libfuzzer-sys = "0.4"

[dependencies.test-lib]
path = ".."

# Prevent this from interfering with workspaces
[workspace]
members = ["."]

[[bin]]
name = "TestStruct_foo"
path = "fuzz_targets/TestStruct_foo.rs"
test = false
doc = false
"#;

    const VALID_GENERATED_CARGO_TOML_MODULE_IMPL: &str = r#"[package]
name = "test-lib-fuzz"
version = "0.0.0"
authors = ["Automatically generated"]
publish = false
edition = "2018"

[package.metadata]
cargo-fuzz = true

[dependencies]
libfuzzer-sys = "0.4"

[dependencies.test-lib]
path = ".."

# Prevent this from interfering with workspaces
[workspace]
members = ["."]

[[bin]]
name = "foo__bar__dog__TestStruct_cat"
path = "fuzz_targets/foo__bar__dog__TestStruct_cat.rs"
test = false
doc = false
"#;
}
