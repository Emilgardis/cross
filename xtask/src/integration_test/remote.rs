//! cross with `CROSS_REMOTE=1`, with and without a persistent data volume.

use std::fs;

use cross::CommandExt;
use cross::shell::MessageInfo;

use super::Tester;
use crate::test_util::{
    RUST_CPP_HELLO_WORLD, Steps, binary_path, build_cross, command, fetch, git_clone,
};

impl Tester<'_> {
    pub(super) fn remote(&mut self) -> cross::Result<()> {
        build_cross(self.msg_info, None, &["cross", "cross-util"])?;
        let cross_util = || {
            let mut cross_util = command(binary_path("cross-util"));
            cross_util.env("CROSS_REMOTE", "1");
            cross_util
        };
        let list_volumes = |msg_info: &mut MessageInfo| -> cross::Result<Vec<String>> {
            Ok(cross_util()
                .args(["volumes", "list"])
                .run_and_get_stdout(msg_info)?
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_owned)
                .collect())
        };
        let mut volumes = vec![];
        self.step("create volume", |t| {
            let output = cross_util()
                .args(["volumes", "create"])
                .run_and_get_output(t.msg_info)?;
            let stderr = String::from_utf8_lossy(&output.stderr);
            // if the create volume fails, ensure it exists.
            eyre::ensure!(
                output.status.success() || stderr.contains("already exists"),
                "creating volume failed: {stderr}"
            );
            volumes = list_volumes(t.msg_info)?;
            eyre::ensure!(!volumes.is_empty(), "no volumes after creating one");
            Ok(())
        })?;
        self.step("run", |t| t.run_cpp_hello())?;
        self.step("remove volume", |t| {
            cross_util()
                .args(["volumes", "remove"])
                .run(t.msg_info, false)?;
            // volumes are per toolchain and rustc version (`cross-<toolchain>-<hash>-<commit>`),
            // `remove` should remove the current one
            let remaining = list_volumes(t.msg_info)?;
            eyre::ensure!(
                volumes.iter().any(|volume| !remaining.contains(volume)),
                "no volume was removed, still have {remaining:?}"
            );
            Ok(())
        })?;
        self.step("run without persistent volume", |t| t.run_cpp_hello())
    }

    fn run_cpp_hello(&mut self) -> cross::Result<()> {
        let dir = git_clone(self.msg_info, RUST_CPP_HELLO_WORLD, false)?;
        fetch(self.msg_info, &dir, None)?;
        let stdout = self
            .cross(&dir)
            .args(["run", "--target", &self.target])
            .env("CROSS_REMOTE", "1")
            .run_and_get_stdout(self.msg_info)?;
        eyre::ensure!(
            stdout.contains("Hello, world!"),
            "expected `Hello, world!`, got {stdout:?}"
        );
        let source = dir.join("hellopp.cc");
        fs::write(
            &source,
            fs::read_to_string(&source)?.replace("Hello, world", "Hello, test"),
        )?;
        let stdout = self
            .cross(&dir)
            .args(["run", "--target", &self.target])
            .env("CROSS_REMOTE", "1")
            .run_and_get_stdout(self.msg_info)?;
        eyre::ensure!(
            stdout.contains("Hello, test!"),
            "expected `Hello, test!`, got {stdout:?}"
        );
        Ok(())
    }
}
