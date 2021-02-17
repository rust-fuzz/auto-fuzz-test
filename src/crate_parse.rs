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
#[cfg_attr(test, derive(Debug))]
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

    pub fn write_cargo_toml(&self, function: &Ident, attr: &TokenStream) -> Result<(), Error> {
        // This is used to distinguish functions with the same names but in different modules
        let ident = if attr.is_empty() {
            function.to_string()
        } else {
            attr.to_string().replace("::", "__") + "__" + &function.to_string()
        };

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
                    "{}{}{}{}{}",
                    &CARGO_TOML_TEMPLATE_PREFIX,
                    &self.crate_name(),
                    &CARGO_TOML_TEMPLATE_INFIX,
                    &self.crate_name(),
                    &CARGO_TOML_TEMPLATE_POSTFIX
                )?;

                write!(
                    file,
                    "{}{}{}{}{}",
                    &TARGET_TEMPLATE_PREFIX,
                    &ident,
                    &TARGET_TEMPLATE_INFIX,
                    &ident,
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

                    // Checking, that we are not going to duplicate [[bin]] targets
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
                                    s == &ident
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
                            &ident,
                            &TARGET_TEMPLATE_INFIX,
                            &ident,
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

const CARGO_TOML_TEMPLATE_PREFIX: &str = r#"[package]
name = ""#;

const CARGO_TOML_TEMPLATE_INFIX: &str = r#"-fuzz"
version = "0.0.0"
authors = ["Automatically generated"]
publish = false
edition = "2018"

[package.metadata]
cargo-fuzz = true

[dependencies]
libfuzzer-sys = "0.3"

[dependencies."#;

const CARGO_TOML_TEMPLATE_POSTFIX: &str = r#"]
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
    use std::fs::File;
    use std::io::Write;
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

        assert_eq!(parse_crate_name(&cargo_toml_path), None);
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
            parse_crate_name(&cargo_toml_path),
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

    const VALID_CARGO_TOML: &str = r#"[package]
name = "test-lib"
version = "0.1.0"
authors = ["jhwgh1968 <jhwgh1968@users.noreply.github.com>"]
edition = "2018"

[dependencies]
auto-fuzz-test = { path = "../"  }
arbitrary = { version = "0.4", features = ["derive"]  }
"#;
}
