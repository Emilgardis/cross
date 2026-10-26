//! Building an image with `pre-build` using podman.

use std::fs;

use cross::CommandExt;

use super::Tester;
use crate::test_util::{Steps, build_cross, cargo_init, tmp_dir};

impl Tester<'_> {
    pub(super) fn podman(&mut self) -> cross::Result<()> {
        build_cross(self.msg_info, None, &["cross"])?;
        self.step("build with pre-build", |t| {
            let dir = cargo_init(t.msg_info, "--bin", "hello")?;
            fs::write(
                tmp_dir().join("Cross.toml"),
                "[build]\npre-build = [\"apt-get update\"]\n",
            )?;
            t.cross(&dir)
                .args(["build", "--target", &t.target, "--verbose"])
                .env("CROSS_CONTAINER_ENGINE", "podman")
                .run(t.msg_info, false)
        })
    }
}
