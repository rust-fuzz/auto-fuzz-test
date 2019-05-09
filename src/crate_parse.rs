use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

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
        if entries.any(|result| result.map(|entry| entry.file_name().to_string_lossy()
                                            == "Cargo.toml").unwrap_or(false)) {
            if let Some(crate_name) = parse_crate_name(&path.join("Cargo.toml")) {
                Some(CrateInfo {
                    crate_root: path.to_path_buf(),
                    crate_name: crate_name
                })
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn from_inner_source_file<'p>(rs_path: &Path) -> Option<CrateInfo> {
        if !rs_path.is_file() {
            return None;
        }
        let mut parent: Option<&Path> = rs_path.parent();
        while parent.is_some() {
            if let Some(info) = CrateInfo::from_root(*parent.as_ref().unwrap()) {
                return Some(info);
            }
            parent = parent.and_then(|p| p.parent());
        }
        None
    }

    pub fn crate_root(&self) -> &Path {
        &self.crate_root
    }

    pub fn crate_name(&self) -> &str {
        &self.crate_name
    }

    pub fn fuzz_dir(&self) -> std::io::Result<PathBuf> {
        let fuzz_dir_path = self.crate_root.join("fuzz").join("fuzz_targets");
        match std::fs::create_dir(&fuzz_dir_path) {
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
}

fn parse_crate_name(cargo_toml_path: &Path) -> Option<String> {
    let cargo_bytes = {
        let mut cargo_bytes = Vec::new();
        File::open(cargo_toml_path).ok()?.read_to_end(&mut cargo_bytes).ok()?;
        cargo_bytes
    };

    let cargo_toml: TomlValue = toml::from_slice(&cargo_bytes).ok()?;

    Some(cargo_toml.get("package")?.get("name")?.as_str()?.to_string())
}
