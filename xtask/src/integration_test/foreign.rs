//! Images with a toolchain for another host, `image.toolchain` in Cross.toml.

use std::fs;
use std::process::Command;

use cross::CommandExt;
use cross::shell::MessageInfo;

use super::Tester;
use crate::test_util::{Steps, build_cross, cargo_init, container_engine, tmp_dir};

impl Tester<'_> {
    pub(super) fn foreign(&mut self) -> cross::Result<()> {
        build_cross(self.msg_info, None, &["cross"])?;
        let engine = container_engine()?;
        let remove_images = |msg_info: &mut MessageInfo| -> cross::Result<()> {
            let images = Command::new(&engine)
                .args([
                    "images",
                    "--format",
                    "{{.Repository}}:{{.Tag}}",
                    "--filter",
                    "label=org.cross-rs.for-cross-target",
                ])
                .run_and_get_stdout(msg_info)?;
            let images: Vec<_> = images
                .lines()
                .filter(|image| image.contains("cross-custom-tmp"))
                .collect();
            eyre::ensure!(!images.is_empty(), "no `cross-custom-tmp` images found");
            Command::new(&engine)
                .arg("rmi")
                .args(images)
                .run(msg_info, false)
        };

        let dir = cargo_init(self.msg_info, "--bin", "foreign_toolchain")?;
        self.step("alpine with musl toolchain", |t| {
            fs::write(
                tmp_dir().join("Cross.toml"),
                r#"[build]
default-target = "x86_64-unknown-linux-musl"

[target."x86_64-unknown-linux-musl"]
image.name = "alpine:edge"
image.toolchain = ["x86_64-unknown-linux-musl"]
pre-build = ["apk add --no-cache gcc musl-dev"]
"#,
            )?;
            t.cross(&dir).args(["run", "-v"]).run(t.msg_info, false)?;
            remove_images(t.msg_info)
        })?;
        self.step("ubuntu with aarch64 toolchain", |t| {
            fs::write(
                tmp_dir().join("Cross.toml"),
                r#"[build]
default-target = "x86_64-unknown-linux-gnu"

[target.x86_64-unknown-linux-gnu]
pre-build = [
    "apt-get update && apt-get install -y libc6 g++-x86-64-linux-gnu libc6-dev-amd64-cross",
]

[target.x86_64-unknown-linux-gnu.env]
passthrough = [
    "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc",
    "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER=/qemu-runner x86_64",
    "CC_x86_64_unknown_linux_gnu=x86_64-linux-gnu-gcc",
    "CXX_x86_64_unknown_linux_gnu=x86_64-linux-gnu-g++",
]

[target.x86_64-unknown-linux-gnu.image]
name = "ubuntu:20.04"
toolchain = ["aarch64-unknown-linux-gnu"]
"#,
            )?;
            t.cross(&dir).args(["build", "-v"]).run(t.msg_info, false)?;
            remove_images(t.msg_info)
        })
    }
}
