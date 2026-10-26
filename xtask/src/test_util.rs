//! Helpers for `target-test` and `integration-test`.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use cross::CommandExt;
use cross::shell::MessageInfo;

use crate::util::get_cargo_workspace;

pub const RUST_CPP_HELLO_WORLD: &str = "https://github.com/cross-rs/rust-cpp-hello-word";
pub const RUST_CPP_ACCUMULATE: &str = "https://github.com/cross-rs/rust-cpp-accumulate";
pub const RUST_CMAKE_HELLO_WORLD: &str = "https://github.com/cross-rs/rust-cmake-hello-world";
pub const TEST_WORKSPACE: &str = "https://github.com/cross-rs/test-workspace";
pub const COMPILER_BUILTINS: &str = "https://github.com/rust-lang-nursery/compiler-builtins";
pub const CORTEST: &str = "https://github.com/japaric/cortest";

/// A command without the toolchain pinning `cargo xtask` inherits.
pub fn command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    // rustup and `cargo run` set these for xtask, they'd force xtask's toolchain onto
    // every nested cargo and cross, overriding `+nightly`
    command.env_remove("RUSTUP_TOOLCHAIN");
    command.env_remove("RUSTUP_TOOLCHAIN_SOURCE");
    command.env_remove("CARGO");
    for (key, _) in std::env::vars_os() {
        let key_str = key.to_string_lossy();
        if key_str.starts_with("CARGO_PKG_") || key_str.starts_with("CARGO_MANIFEST_") {
            command.env_remove(&key);
        }
    }
    command
}

pub fn cargo() -> Command {
    command("cargo")
}

/// Runs `command`, retrying 5 times with backoff.
pub fn retry(mut command: Command, msg_info: &mut MessageInfo) -> cross::Result<()> {
    let mut timeout = Duration::from_secs(1);
    let mut tries = 5;
    loop {
        tries -= 1;
        match command.run(msg_info, false) {
            Ok(()) => return Ok(()),
            Err(e) if tries == 0 => return Err(e),
            Err(_) => {
                eprintln!("Retrying ...");
                std::thread::sleep(timeout);
                timeout *= 2;
            }
        }
    }
}

/// Escapes data for a GitHub workflow command, see
/// https://github.com/actions/toolkit/blob/main/packages/core/src/command.ts
fn gha_escape(data: &str, property: bool) -> String {
    let data = data
        .replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A");
    if property {
        data.replace(':', "%3A").replace(',', "%2C")
    } else {
        data
    }
}

fn is_gha() -> bool {
    std::env::var_os("GITHUB_ACTIONS").is_some()
}

pub trait Steps: Sized {
    /// What is being tested, prefixed to step names in annotations.
    fn title(&self) -> &str;

    /// Runs a named step, grouped in GitHub Actions logs and annotated on failure.
    fn step(
        &mut self,
        name: &str,
        f: impl FnOnce(&mut Self) -> cross::Result<()>,
    ) -> cross::Result<()> {
        let title = format!("{} {name}", self.title());
        if is_gha() {
            println!("::group::{name}");
        } else {
            eprintln!("==> {name}");
        }
        let result = f(self);
        if is_gha() {
            println!("::endgroup::");
        }
        if let Err(e) = &result {
            if is_gha() {
                println!(
                    "::error title={}::{}",
                    gha_escape(&title, true),
                    gha_escape(&format!("{e:#}"), false)
                );
            }
        }
        result.map_err(|e| e.wrap_err(format!("step `{title}` failed")))
    }
}

/// Steps that only need a title.
#[derive(Debug)]
pub struct Ctx<'a> {
    pub msg_info: &'a mut MessageInfo,
    pub title: String,
}

impl Steps for Ctx<'_> {
    fn title(&self) -> &str {
        &self.title
    }
}

pub fn tmp_dir() -> PathBuf {
    get_cargo_workspace().join("target/tmp")
}

/// Creates an empty directory in `target/tmp`, which is made the only workspace member.
pub fn temp_dir() -> cross::Result<PathBuf> {
    let tmp = tmp_dir();
    fs::create_dir_all(&tmp)?;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let name = format!("tmp.{nanos:x}");
    let dir = tmp.join(&name);
    fs::create_dir_all(&dir)?;
    // left behind by earlier steps or runs, e.g. a runner, which would apply to this crate too
    match fs::remove_file(tmp.join("Cross.toml")) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e.into()),
        _ => {}
    }
    fs::write(
        tmp.join("Cargo.toml"),
        format!("[workspace]\nmembers = [\"{name}\"]\nresolver = \"3\"\n"),
    )?;
    Ok(dir)
}

/// Creates an empty directory in the system temp directory, outside the cross workspace.
pub fn system_temp_dir() -> cross::Result<PathBuf> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("cross-xtask.{nanos:x}"));
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn git_clone(msg_info: &mut MessageInfo, url: &str, recursive: bool) -> cross::Result<PathBuf> {
    let dir = temp_dir()?;
    let mut git = command("git");
    git.args(["clone", "--depth", "1"]);
    if recursive {
        git.arg("--recursive");
    }
    git.arg(url).arg(&dir).run(msg_info, false)?;
    Ok(dir)
}

pub fn cargo_init(msg_info: &mut MessageInfo, kind: &str, name: &str) -> cross::Result<PathBuf> {
    let dir = temp_dir()?;
    cargo()
        .args(["init", kind, "--name", name])
        .arg(&dir)
        .run(msg_info, false)?;
    Ok(dir)
}

pub fn fetch(msg_info: &mut MessageInfo, dir: &Path, target: Option<&str>) -> cross::Result<()> {
    let mut fetch = cargo();
    fetch.arg("fetch").current_dir(dir);
    if let Some(target) = target {
        fetch.args(["--target", target]);
    }
    retry(fetch, msg_info)
}

pub fn binary_path(name: &str) -> PathBuf {
    get_cargo_workspace()
        .join("target/debug")
        .join(name)
        .with_extension(std::env::consts::EXE_EXTENSION)
}

/// Builds the debug binaries of cross, fetching for `target` first.
pub fn build_cross(
    msg_info: &mut MessageInfo,
    target: Option<&str>,
    bins: &[&str],
) -> cross::Result<()> {
    let workspace = get_cargo_workspace();
    fetch(msg_info, workspace, target)?;
    let mut build = cargo();
    build.args(["build", "-p", "cross"]).current_dir(workspace);
    for bin in bins {
        build.args(["--bin", bin]);
    }
    build.run(msg_info, false)
}

/// `CROSS_TARGET_<TARGET>_IMAGE`
pub fn image_var(target: &str) -> String {
    format!(
        "CROSS_TARGET_{}_IMAGE",
        target.to_ascii_uppercase().replace('-', "_")
    )
}

/// Container engine, `CROSS_CONTAINER_ENGINE` or docker, then podman.
pub fn container_engine() -> cross::Result<PathBuf> {
    Ok(cross::docker::get_container_engine()?)
}
