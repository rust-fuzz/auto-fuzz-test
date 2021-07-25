use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::{fs, io};

// Copied from dtolnay/trybuild

fn raw_cargo() -> Command {
    match env::var_os("CARGO") {
        Some(cargo) => Command::new(cargo),
        None => Command::new("cargo"),
    }
}

fn cargo(project_dir: &PathBuf) -> Command {
    let mut cmd = raw_cargo();
    cmd.current_dir(&project_dir);
    cmd.arg("--offline");
    set_env(&mut cmd);
    cmd
}

const RUSTFLAGS: &str = "RUSTFLAGS";

fn set_env(cmd: &mut Command) {
    let rustflags = match env::var_os(RUSTFLAGS) {
        Some(rustflags) => rustflags,
        None => return,
    };

    cmd.env(RUSTFLAGS, rustflags);
}

pub fn test_dir(source_dir: &PathBuf) -> std::io::Result<PathBuf> {
    let test_dir = source_dir.join("target").join("testing-workspace");
    match std::fs::create_dir_all(&test_dir) {
        Ok(_) => Ok(test_dir),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                Ok(test_dir)
            } else {
                Err(e)
            }
        }
    }
}

pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

pub fn cargo_build(project_dir: &PathBuf) -> Result<ExitStatus, std::io::Error> {
    let cargo_handle = cargo(project_dir)
        .arg("build")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    print!("{}", String::from_utf8(cargo_handle.stdout).unwrap());
    eprint!("{}", String::from_utf8(cargo_handle.stderr).unwrap());
    Ok(cargo_handle.status)
}

pub fn fuzz_build(project_dir: &PathBuf, target_name: &str) -> Result<ExitStatus, std::io::Error> {
    let cargo_handle = cargo(project_dir)
        .arg("fuzz")
        .arg("build")
        .arg(&target_name)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    print!("{}", String::from_utf8(cargo_handle.stdout).unwrap());
    eprint!("{}", String::from_utf8(cargo_handle.stderr).unwrap());
    Ok(cargo_handle.status)
}
