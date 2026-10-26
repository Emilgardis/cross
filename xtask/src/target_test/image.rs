//! Building, running and testing with cross for a target, including C++ and cmake.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use cross::shell::MessageInfo;
use cross::{CommandExt, Stream};

use super::TargetTest;
use crate::test_util::{
    COMPILER_BUILTINS, CORTEST, RUST_CMAKE_HELLO_WORLD, RUST_CPP_ACCUMULATE, RUST_CPP_HELLO_WORLD,
    Steps, TEST_WORKSPACE, binary_path, build_cross, command, git_clone, image_var, strip_ansi,
    tmp_dir,
};
use crate::util::CiTarget;

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
}

impl Steps for Tester<'_> {
    fn title(&self) -> &str {
        &self.target
    }
}

impl Tester<'_> {
    fn git_clone(&mut self, url: &str, recursive: bool) -> cross::Result<PathBuf> {
        git_clone(self.msg_info, url, recursive)
    }

    fn cargo_init(&mut self, kind: &str, name: &str) -> cross::Result<PathBuf> {
        crate::test_util::cargo_init(self.msg_info, kind, name)
    }

    fn fetch(&mut self, dir: &Path) -> cross::Result<()> {
        crate::test_util::fetch(self.msg_info, dir, Some(&self.target))
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
        let mut cross = self.cross_command(dir, subcommand);
        cross.args(args).args(&self.cross_flags);
        let (status, stderr) =
            cross.run_and_get_output_streamed(self.msg_info, Stream::Stderr, |_| true)?;
        let lines: Vec<_> = stderr.lines().map(strip_ansi).collect();
        let tail = lines[lines.len().saturating_sub(10)..].join("\n");
        eyre::ensure!(
            status.success(),
            "`{}` failed with {status}\n{tail}",
            cross.command_pretty(self.msg_info, |_| false)
        );
        Ok(())
    }

    fn cross_build(&mut self, dir: &Path, args: &[&str]) -> cross::Result<()> {
        self.cross(dir, "build", args)
    }

    /// `cross <subcommand>` once per runner
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
                tmp_dir().join("Cross.toml"),
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
                let dir = t.git_clone(COMPILER_BUILTINS, true)?;
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
                    let dir = t.git_clone(CORTEST, true)?;
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
                    let dir = t.git_clone(TEST_WORKSPACE, true)?;
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
                let dir = t.git_clone(RUST_CPP_ACCUMULATE, false)?;
                t.fetch(&dir)?;
                t.cross_build(&dir, &[])
            })?;
        }

        // Test C++ support
        if self.std && self.cpp {
            self.step("c++", |t| {
                let dir = t.git_clone(RUST_CPP_HELLO_WORLD, false)?;
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
            let dir = t.git_clone(RUST_CMAKE_HELLO_WORLD, true)?;
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

pub(super) fn test(
    args: TargetTest,
    ci_target: &CiTarget,
    toolchain: Option<&str>,
    msg_info: &mut MessageInfo,
) -> cross::Result<()> {
    let target = &ci_target.target;
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

    build_cross(msg_info, Some(target), &["cross"])?;

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
        envs.push((image_var(target), image));
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
        cross: binary_path("cross"),
        toolchain,
        cross_flags,
        rustflags,
        envs,
        msg_info,
    }
    .test()
}
