//! cross from inside the cross image, with the docker socket mounted.

use std::fs;

use cross::shell::MessageInfo;
use cross::{CommandExt, ToUtf8};

use crate::test_util::{Ctx, RUST_CPP_HELLO_WORLD, Steps, command, git_clone, image_var};

pub(super) fn test(image: Option<String>, msg_info: &mut MessageInfo) -> cross::Result<()> {
    let target = "aarch64-unknown-linux-gnu";
    // OCI does not support uppercase characters from orga/owner names
    let image = image
        .unwrap_or_else(|| "ghcr.io/cross-rs/cross:main".to_owned())
        .to_lowercase();
    let target_image = std::env::var(image_var(target))
        .ok()
        .map(|image| image.to_lowercase());
    let docker_run = |envs: &[(&str, &str)]| {
        let mut docker = command("docker");
        docker.args(["run", "--rm"]);
        docker.args(["-e", &format!("TARGET={target}")]);
        for (key, value) in envs {
            docker.args(["-e", &format!("{key}={value}")]);
        }
        if let Some(target_image) = &target_image {
            docker.args(["-e", &format!("{}={target_image}", image_var(target))]);
        }
        docker.args(["-v", "/var/run/docker.sock:/var/run/docker.sock"]);
        docker
    };

    let mut ctx = Ctx {
        msg_info,
        title: "cross".to_owned(),
    };
    ctx.step("cross run in container", |ctx| {
        docker_run(&[])
            .args([&image, "sh", "-c"])
            .arg(
                r#"
td="$(mktemp -d)"
git clone --depth 1 https://github.com/cross-rs/rust-cpp-hello-word "${td}"
cd "${td}"
cross run --target "${TARGET}"
"#,
            )
            .run(ctx.msg_info, false)
    })?;
    ctx.step("cross build in container with mounted project", |ctx| {
        let dir = git_clone(ctx.msg_info, RUST_CPP_HELLO_WORLD, false)?;
        fs::write(
            dir.join("Cross.toml"),
            format!("[target.{target}]\npre-build = [\"exit 0\"]\n"),
        )?;
        docker_run(&[("CROSS_CONTAINER_IN_CONTAINER", "1")])
            .arg("-v")
            .arg(format!("{}:/mount", dir.to_utf8()?))
            .args(["-w", "/mount", &image, "cross", "build", "--target", target])
            .run(ctx.msg_info, false)
    })
}
