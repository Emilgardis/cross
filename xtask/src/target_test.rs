//! Integration tests for a target, ported from `ci/test.sh`.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use clap::Args;
use clap::builder::BoolishValueParser;
use cross::CommandExt;
use cross::shell::MessageInfo;

use crate::util::{CiTarget, ImageTarget, get_cargo_workspace, get_matrix};

#[derive(Args, Debug)]
pub struct TargetTest {
    /// Target to test, `name` or `name.sub`.
    target: ImageTarget,
    /// Image to test, sets `CROSS_TARGET_<TARGET>_IMAGE`.
    #[clap(long)]
    image: Option<String>,
    /// Override `std` from targets.toml.
    #[clap(long, value_parser = BoolishValueParser::new())]
    std: Option<bool>,
    /// Override `cpp` from targets.toml.
    #[clap(long, value_parser = BoolishValueParser::new())]
    cpp: Option<bool>,
    /// Override `dylib` from targets.toml.
    #[clap(long, value_parser = BoolishValueParser::new())]
    dylib: Option<bool>,
    /// Override `run` from targets.toml.
    #[clap(long, value_parser = BoolishValueParser::new())]
    run: Option<bool>,
    /// Override `build-std` from targets.toml.
    #[clap(long, value_parser = BoolishValueParser::new())]
    build_std: Option<bool>,
    /// Override `runners` from targets.toml.
    #[clap(long, short, num_args = 0.., value_delimiter = ' ')]
    runners: Vec<String>,
    /// Set `CROSS_DEBUG=1`.
    #[clap(long)]
    cross_debug: bool,
}

/// A command without the toolchain pinning `cargo xtask` inherits.
fn command(program: impl AsRef<OsStr>) -> Command {
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

fn cargo() -> Command {
    command("cargo")
}

/// Runs `command`, retrying with backoff like `retry` in `ci/shared.sh`.
fn retry(mut command: Command, msg_info: &mut MessageInfo) -> cross::Result<()> {
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

struct Tester<'a> {
    msg_info: &'a mut MessageInfo,
    target: String,
    std: bool,
    cpp: bool,
    dylib: bool,
    run: bool,
    runners: Vec<String>,
    cross: PathBuf,
    /// `+toolchain` for cross
    toolchain: Option<String>,
    cross_flags: Vec<String>,
    rustflags: String,
    envs: Vec<(String, String)>,
    tmp: PathBuf,
    is_gha: bool,
}

impl Tester<'_> {
    /// Runs a named step, grouped in GitHub Actions logs and annotated on failure.
    fn step(
        &mut self,
        name: &str,
        f: impl FnOnce(&mut Self) -> cross::Result<()>,
    ) -> cross::Result<()> {
        if self.is_gha {
            println!("::group::{name}");
        } else {
            eprintln!("==> {name}");
        }
        let result = f(self);
        if self.is_gha {
            println!("::endgroup::");
        }
        if let Err(e) = &result {
            if self.is_gha {
                println!(
                    "::error title={}::{}",
                    gha_escape(&format!("{} {name}", self.target), true),
                    gha_escape(&format!("{e:#}"), false)
                );
            }
        }
        result.map_err(|e| e.wrap_err(format!("step `{name}` failed for {}", self.target)))
    }

    /// Creates an empty directory in `target/tmp`, which is made the only workspace member,
    /// like `mkcargotemp` in `ci/shared.sh`.
    fn temp_dir(&self) -> cross::Result<PathBuf> {
        fs::create_dir_all(&self.tmp)?;
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let name = format!("tmp.{nanos:x}");
        let dir = self.tmp.join(&name);
        fs::create_dir_all(&dir)?;
        fs::write(
            self.tmp.join("Cargo.toml"),
            format!("[workspace]\nmembers = [\"{name}\"]\nresolver = \"3\"\n"),
        )?;
        Ok(dir)
    }

    fn git_clone(&mut self, url: &str, recursive: bool) -> cross::Result<PathBuf> {
        let dir = self.temp_dir()?;
        let mut git = command("git");
        git.args(["clone", "--depth", "1"]);
        if recursive {
            git.arg("--recursive");
        }
        git.arg(url).arg(&dir).run(self.msg_info, false)?;
        Ok(dir)
    }

    fn cargo_init(&mut self, kind: &str, name: &str) -> cross::Result<PathBuf> {
        let dir = self.temp_dir()?;
        cargo()
            .args(["init", kind, "--name", name])
            .arg(&dir)
            .run(self.msg_info, false)?;
        Ok(dir)
    }

    fn fetch(&mut self, dir: &Path) -> cross::Result<()> {
        let mut fetch = cargo();
        fetch
            .args(["fetch", "--target", &self.target])
            .current_dir(dir);
        retry(fetch, self.msg_info)
    }

    fn cross_command(&self, dir: &Path, subcommand: &str) -> Command {
        let mut cross = command(&self.cross);
        if let Some(toolchain) = &self.toolchain {
            cross.arg(toolchain);
        }
        cross
            .arg(subcommand)
            .args(["--target", &self.target])
            .current_dir(dir)
            .env("RUSTFLAGS", &self.rustflags)
            .env("QEMU_STRACE", "1")
            .envs(self.envs.iter().map(|(k, v)| (k, v)));
        cross
    }

    /// `cross <subcommand>` once, without iterating runners
    fn cross(&mut self, dir: &Path, subcommand: &str, args: &[&str]) -> cross::Result<()> {
        self.cross_command(dir, subcommand)
            .args(args)
            .args(&self.cross_flags)
            .run(self.msg_info, false)
    }

    fn cross_build(&mut self, dir: &Path, args: &[&str]) -> cross::Result<()> {
        self.cross(dir, "build", args)
    }

    /// `cross <subcommand>` once per runner, like `cross_run` in `ci/test.sh`
    fn cross_with_runners(
        &mut self,
        dir: &Path,
        subcommand: &str,
        args: &[&str],
    ) -> cross::Result<()> {
        if self.runners.is_empty() {
            return self.cross(dir, subcommand, args);
        }
        for runner in self.runners.clone() {
            fs::write(
                self.tmp.join("Cross.toml"),
                format!("[target.{}]\nrunner = \"{runner}\"\n", self.target),
            )?;
            self.cross(dir, subcommand, args)?;
        }
        Ok(())
    }

    fn cross_run(&mut self, dir: &Path, args: &[&str]) -> cross::Result<()> {
        self.cross_with_runners(dir, "run", args)
    }

    fn cross_test(&mut self, dir: &Path, args: &[&str]) -> cross::Result<()> {
        self.cross_with_runners(dir, "test", args)
    }

    fn cross_bench(&mut self, dir: &Path, args: &[&str]) -> cross::Result<()> {
        self.cross_with_runners(dir, "bench", args)
    }

    fn workspace_test(&mut self, dir: &Path, args: &[&str]) -> cross::Result<()> {
        self.cross(dir, "build", &[&["--workspace"], args].concat())?;
        self.cross(dir, "run", &[&["-p", "binary"], args].concat())?;
        self.cross(
            dir,
            "run",
            &[&["--bin", "dependencies", "--features=dependencies"], args].concat(),
        )
    }

    fn test(&mut self) -> cross::Result<()> {
        let target = self.target.clone();

        if self.std {
            self.step("cross check", |t| {
                let dir = t.cargo_init("--lib", "foo")?;
                fs::write(dir.join("src/lib.rs"), "#![no_std]\n")?;
                t.cross(&dir, "check", &[])
            })?;
        } else {
            // `cross build` test for targets where `std` is not available
            self.step("compiler-builtins", |t| {
                let dir = t.git_clone(
                    "https://github.com/rust-lang-nursery/compiler-builtins",
                    true,
                )?;
                t.fetch(&dir)?;
                t.cross_build(&dir, &["-p", "compiler_builtins", "--lib"])
            })?;
        }

        // Solaris: Map old to new symbols. compat.o is compiled in solaris.sh during image build.
        // - https://github.com/cross-rs/cross/issues/1599
        // - https://github.com/cross-rs/cross/issues/1730
        if target.ends_with("-solaris") {
            self.rustflags.push_str(" -C link-arg=/compat.o");
        }

        if target.ends_with("-unknown-emscripten") {
            self.step("build lib", |t| {
                let dir = t.cargo_init("--lib", "foo")?;
                t.fetch(&dir)?;
                t.cross_build(&dir, &[])
            })?;
        } else if !target.starts_with("thumb") {
            // thumb targets are tested in later steps
            self.step("build bin", |t| {
                let dir = t.cargo_init("--bin", "hello")?;
                t.fetch(&dir)?;
                t.cross_build(&dir, &[])
            })?;
        }

        if self.run {
            if self.dylib {
                self.step("test lib", |t| {
                    let dir = t.cargo_init("--lib", "foo")?;
                    t.cross_test(&dir, &[])?;
                    t.cross_bench(&dir, &[])
                })?;
            }

            if target.starts_with("thumb") && target.contains("-none-eabi") {
                self.step("run cortest", |t| {
                    let dir = t.git_clone("https://github.com/japaric/cortest", true)?;
                    t.cross_run(&dir, &["--example", "hello", "--release"])
                })?;
            } else {
                self.step("run bin", |t| {
                    let dir = t.cargo_init("--bin", "hello")?;
                    fs::create_dir(dir.join("examples"))?;
                    fs::create_dir(dir.join("tests"))?;
                    fs::write(
                        dir.join("examples/e.rs"),
                        "fn main() { println!(\"Example!\"); }\n",
                    )?;
                    fs::write(dir.join("tests/t.rs"), "#[test] fn t() {}\n")?;
                    t.cross_run(&dir, &[])?;
                    t.cross_run(&dir, &["--example", "e"])?;
                    t.cross_test(&dir, &[])?;
                    t.cross_bench(&dir, &[])
                })?;
                self.step("test-workspace", |t| {
                    let dir = t.git_clone("https://github.com/cross-rs/test-workspace", true)?;
                    t.workspace_test(&dir, &["--manifest-path=./workspace/Cargo.toml"])?;
                    let workspace = dir.join("workspace");
                    t.workspace_test(&workspace, &[])?;
                    t.cross(&workspace.join("binary"), "run", &[])
                })?;
            }
        }

        // Test C++ support in a no_std context
        if self.cpp {
            self.step("c++ no_std", |t| {
                let dir = t.git_clone("https://github.com/cross-rs/rust-cpp-accumulate", false)?;
                t.fetch(&dir)?;
                t.cross_build(&dir, &[])
            })?;
        }

        // Test C++ support
        if self.std && self.cpp {
            self.step("c++", |t| {
                let dir = t.git_clone("https://github.com/cross-rs/rust-cpp-hello-word", false)?;
                t.fetch(&dir)?;
                if t.run {
                    t.cross_run(&dir, &[])
                } else {
                    t.cross_build(&dir, &[])
                }
            })?;
        }

        // special tests for a shared C runtime, since we disable the shared c++ runtime
        // https://github.com/cross-rs/cross/issues/902
        if target.contains("-linux-musl") {
            self.step("musl shared crt", |t| {
                let dir = t.cargo_init("--bin", "hello")?;
                t.fetch(&dir)?;
                let rustflags = t.rustflags.clone();
                t.rustflags = format!("{rustflags} -C target-feature=-crt-static");
                let result = t.cross_build(&dir, &[]);
                t.rustflags = rustflags;
                result
            })?;
        }

        self.step("cmake", |t| {
            let dir = t.git_clone("https://github.com/cross-rs/rust-cmake-hello-world", true)?;
            t.fetch(&dir)?;
            if target == "arm-linux-androideabi" {
                // ARMv5te isn't supported anymore by Android, which produces missing
                // symbol errors with re2 like `__libcpp_signed_lock_free`.
                t.cross_run(&dir, &["--features=tryrun"])
            } else {
                match (t.std, t.run, t.cpp) {
                    (true, true, true) => t.cross_run(&dir, &["--features=re2,tryrun"]),
                    (true, false, true) => t.cross_build(&dir, &["--features=re2"]),
                    (true, true, false) => t.cross_run(&dir, &["--features=tryrun"]),
                    (true, false, false) => t.cross_build(&dir, &["--features=tryrun"]),
                    (false, _, _) => t.cross_build(&dir, &["--lib"]),
                }
            }
        })?;

        // test running binaries with cleared environment via runner, regression test for #1420
        if self.run && target != "wasm32-unknown-emscripten" {
            self.step("run with cleared env", |t| {
                let dir = t.cargo_init("--bin", "foo")?;
                fs::create_dir(dir.join("src/bin"))?;
                let upper_target = target.to_ascii_uppercase().replace('-', "_");
                fs::write(
                    dir.join("src/bin/launch.rs"),
                    r#"
fn main() {
    let runner = std::env::var("CARGO_TARGET_{upper_target}_RUNNER");
    let mut command = if let Ok(runner) = runner {
        runner.split(' ').map(str::to_string).collect()
    } else {
        vec![]
    };
    let executable = format!("/target/{target}/debug/foo{}", std::env::consts::EXE_SUFFIX);
    command.push(executable.to_string());
    let status = dbg!(std::process::Command::new(&command[0])
        .args(&command[1..])
        .env_clear()) // drop all environment variables
    .status()
    .unwrap();
    std::process::exit(status.code().unwrap());
}
                    "#
                    .replace("{upper_target}", &upper_target)
                    .replace("{target}", &target),
                )?;
                t.cross_build(&dir, &[])?;
                t.cross_run(&dir, &["--bin", "launch"])
            })?;
        }

        Ok(())
    }
}

pub fn target_test(
    args: TargetTest,
    toolchain: Option<&str>,
    msg_info: &mut MessageInfo,
) -> cross::Result<()> {
    let ci_target = get_matrix()
        .iter()
        .find(|t| t.to_image_target() == args.target)
        .ok_or_else(|| eyre::eyre!("`{}` is not a target in targets.toml", args.target))?;
    if ci_target.disabled {
        msg_info.warn(format_args!(
            "`{}` is disabled in targets.toml",
            args.target
        ))?;
    }
    let CiTarget { target, .. } = ci_target;

    let std = args.std.or(ci_target.std).unwrap_or_default();
    let build_std = args.build_std.or(ci_target.build_std).unwrap_or_default();
    let runners = if args.runners.is_empty() {
        ci_target
            .runners
            .as_deref()
            .unwrap_or_default()
            .split_whitespace()
            .map(str::to_owned)
            .collect()
    } else {
        args.runners
    };

    let workspace = get_cargo_workspace();
    let mut fetch = cargo();
    fetch
        .args(["fetch", "--target", target])
        .current_dir(workspace);
    retry(fetch, msg_info)?;
    cargo()
        .args(["build", "-p", "cross", "--bin", "cross"])
        .current_dir(workspace)
        .run(msg_info, false)?;

    let mut cross_flags = vec!["-v".to_owned()];
    let mut rustflags = String::new();
    if build_std {
        cross_flags.push("-Zbuild-std".to_owned());
        if target.contains("mips") {
            // workaround for https://github.com/cross-rs/cross/issues/1322 & https://github.com/rust-lang/rust/issues/108835
            rustflags.push_str("-C opt-level=1");
        }
    }
    let toolchain = match toolchain {
        Some(toolchain) => Some(format!("+{toolchain}")),
        // build-std requires nightly, and no_std targets use nightly
        None if build_std || !std => {
            command("rustup")
                .args(["toolchain", "add", "nightly"])
                .run(msg_info, false)?;
            Some("+nightly".to_owned())
        }
        None => None,
    };

    let mut envs = vec![];
    if let Some(image) = args.image {
        envs.push((
            format!(
                "CROSS_TARGET_{}_IMAGE",
                target.to_ascii_uppercase().replace('-', "_")
            ),
            image,
        ));
    }
    if args.cross_debug {
        envs.push(("CROSS_DEBUG".to_owned(), "1".to_owned()));
    }

    Tester {
        target: target.clone(),
        std,
        cpp: args.cpp.or(ci_target.cpp).unwrap_or_default(),
        dylib: args.dylib.or(ci_target.dylib).unwrap_or_default(),
        run: args.run.or(ci_target.run).unwrap_or_default(),
        runners,
        cross: workspace
            .join("target/debug/cross")
            .with_extension(std::env::consts::EXE_EXTENSION),
        toolchain,
        cross_flags,
        rustflags,
        envs,
        tmp: workspace.join("target/tmp"),
        is_gha: std::env::var_os("GITHUB_ACTIONS").is_some(),
        msg_info,
    }
    .test()
}
