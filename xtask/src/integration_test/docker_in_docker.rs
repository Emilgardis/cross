//! cross inside a docker-in-docker container, `CROSS_CONTAINER_IN_CONTAINER=1`.

use cross::file::PathExt;
use cross::{CommandExt, ToUtf8};

use super::Tester;
use crate::test_util::{Steps, command, image_var};
use crate::util::get_cargo_workspace;

impl Tester<'_> {
    pub(super) fn docker_in_docker(&mut self) -> cross::Result<()> {
        let workspace = get_cargo_workspace();
        let mount = workspace.as_posix_absolute()?;
        let target_upper = self.target.to_ascii_uppercase().replace('-', "_");
        let mut docker = command("docker");
        docker
            .args(["run", "--platform", "linux/amd64", "--rm", "-v"])
            .arg(format!("{}:{mount}", workspace.to_utf8()?))
            .args(["-w", &mount])
            .args(["-e", &format!("TARGET={}", self.target)])
            .args(["-e", &format!("TARGET_UPPER={target_upper}")]);
        // coverage, from `.github/actions/cargo-llvm-cov`
        for var in [
            "RUSTFLAGS",
            "RUST_TEST_THREADS",
            "LLVM_PROFILE_FILE",
            "CARGO_INCREMENTAL",
        ] {
            docker.args(["-e", var]);
        }
        if let Some(image) = &self.image {
            docker.args(["-e", &format!("{}={image}", image_var(&self.target))]);
        }
        docker
            .args(["-v", "/var/run/docker.sock:/var/run/docker.sock"])
            .args(["docker:20.10-dind", "sh", "-c"])
            .arg(
                r#"
set -x
set -euo pipefail

apk add curl
curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "${HOME}/.cargo/env"

# building on release is slow
apk add libgcc gcc musl-dev jq
cargo test --workspace
cargo install --path . --force --debug

export CROSS_CONTAINER_IN_CONTAINER=1

apk add git
td="$(mktemp -d)"
git clone --depth 1 https://github.com/cross-rs/rust-cpp-hello-word "${td}"
cd "${td}"
cross run --target "${TARGET}" --verbose

td="$(mktemp -d)"
git clone --depth 1 https://github.com/cross-rs/test-workspace "${td}"
cd "${td}"
cross build --target "${TARGET}" --workspace \
    --manifest-path="./workspace/Cargo.toml" --verbose
eval CROSS_TARGET_${TARGET_UPPER}_PRE_BUILD="exit" cross build --target "${TARGET}" --workspace \
    --manifest-path="./workspace/Cargo.toml" --verbose
cd workspace
cross build --target "${TARGET}" --workspace --verbose
eval CROSS_TARGET_${TARGET_UPPER}_PRE_BUILD="exit" cross build --target "${TARGET}" --workspace --verbose
cd binary
cross run --target "${TARGET}" --verbose
eval CROSS_TARGET_${TARGET_UPPER}_PRE_BUILD="exit" cross run --target "${TARGET}" --verbose
"#,
            );
        self.step("docker-in-docker", |t| docker.run(t.msg_info, false))
    }
}
