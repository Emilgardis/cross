//! cross with toolchains installed by cargo-bisect-rustc, `CROSS_CUSTOM_TOOLCHAIN=1`.

use std::fs;
use std::process::{Command, Output};

use cross::CommandExt;

use super::Tester;
use crate::test_util::{Steps, build_cross, cargo, command, system_temp_dir};

impl Tester<'_> {
    pub(super) fn bisect(&mut self) -> cross::Result<()> {
        build_cross(self.msg_info, None, &["cross"])?;
        self.step("install cargo-bisect-rustc", |t| {
            cargo()
                .args(["install", "cargo-bisect-rustc", "--debug"])
                .run(t.msg_info, false)
        })?;
        self.step("bisect", |t| {
            let dir = system_temp_dir()?;
            fs::create_dir(dir.join("src"))?;
            fs::write(
                dir.join("Cargo.toml"),
                "[package]\nname = \"bisect\"\nversion = \"0.1.0\"\nedition = \"2021\"\nrust-version = \"1.64\"\n",
            )?;
            fs::write(
                dir.join("build.rs"),
                r#"
fn main() {
    let rustc = std::env::var("RUSTC").unwrap();
    let version = std::process::Command::new(rustc).arg("-V").output().unwrap();
    let version = String::from_utf8(version.stdout).unwrap();
    println!("cargo:rustc-env=RUSTC_VERSION={}", version.trim());
}
"#,
            )?;
            fs::write(
                dir.join("src/main.rs"),
                "fn main() {\n    println!(\"{}\", env!(\"RUSTC_VERSION\"));\n}\n",
            )?;
            // xtask itself is the script, see `fn@script`
            let output = command("cargo-bisect-rustc")
                // shows the output of the script
                .arg("-vv")
                .args(["--start", "2022-07-01", "--end", "2022-07-03", "--script"])
                .arg(std::env::current_exe()?)
                .args(["--target", &t.target])
                // the toolchain cross mounts into the linux container
                .args(["--host", "x86_64-unknown-linux-gnu"])
                .args([
                    "--",
                    "integration-test",
                    "bisect-script",
                    "--target",
                    &t.target,
                ])
                .current_dir(&dir)
                .run_and_get_output(t.msg_info)?;
            let out = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )
            .lines()
            // toolchain download progress
            .filter(|line| !(line.contains(" / ") && line.contains("B/s")))
            .collect::<Vec<_>>()
            .join("\n");
            if t.msg_info.is_verbose() {
                t.msg_info.print(&out)?;
            }
            // bisecting further by commit fails, CI artifacts from 2022 are gone
            eyre::ensure!(
                out.contains("Regression in nightly-2022-07-02"),
                "cargo-bisect-rustc didn't find the regression in nightly-2022-07-02: {out}"
            );
            Ok(())
        })
    }
}

/// The script cargo-bisect-rustc runs for each toolchain.
pub(super) fn script(target: &str) -> cross::Result<()> {
    let cross = std::env::current_exe()?
        .with_file_name("cross")
        .with_extension(std::env::consts::EXE_EXTENSION);
    eprintln!(
        "[bisect-script] RUSTUP_TOOLCHAIN={:?}",
        std::env::var_os("RUSTUP_TOOLCHAIN")
    );
    // not `command()`: cargo-bisect-rustc passes the toolchain with `RUSTUP_TOOLCHAIN` and `command()` removes that
    let mut cross = Command::new(cross);
    cross
        .args(["run", "-v", "--target", target])
        .env("CROSS_CUSTOM_TOOLCHAIN", "1");
    let output = traced(cross)?;
    eyre::ensure!(output.status.success(), "cross run failed");
    // the crate prints the version of the rustc that built it
    let version = String::from_utf8_lossy(&output.stdout);
    eyre::ensure!(
        version.contains("rustc 1.64.0-nightly") && version.contains(" 2022-06-"),
        "not built with a 2022-06 rustc: {version}"
    );
    Ok(())
}

/// Runs `command`, printing it and its result, which cargo-bisect-rustc shows with `-vv`.
fn traced(mut command: Command) -> cross::Result<Output> {
    eprintln!("[bisect-script] running {command:?}");
    let output = command.output();
    match &output {
        Ok(output) => eprintln!(
            "[bisect-script] {}\n--- stdout\n{}\n--- stderr\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
        Err(e) => eprintln!("[bisect-script] failed to start: {e}"),
    }
    Ok(output?)
}
