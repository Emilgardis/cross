//! `cross build` with `CROSS_BUILD_ZIG=1`, building with zig in the zig image.

use std::path::PathBuf;

use cross::CommandExt;
use cross::shell::MessageInfo;

use crate::test_util::{
    Ctx, RUST_CPP_HELLO_WORLD, Steps, binary_path, build_cross, command, git_clone,
};

/// zig cc is very slow: only use a few targets.
const ZIG_TARGETS: &[&str] = &[
    "aarch64-unknown-linux-gnu",
    "aarch64-unknown-linux-musl",
    // disabled, see https://github.com/cross-rs/cross/issues/1425
    // "i586-unknown-linux-gnu",
    // "i586-unknown-linux-musl",
];

pub(super) fn test(image: Option<String>, msg_info: &mut MessageInfo) -> cross::Result<()> {
    build_cross(msg_info, None, &["cross"])?;
    let cross = |target: &str| {
        let mut cross = command(binary_path("cross"));
        cross
            .args(["build", "--target", target, "--verbose"])
            .env("CROSS_BUILD_ZIG", "1");
        if let Some(image) = &image {
            cross.env("CROSS_BUILD_ZIG_IMAGE", image);
        }
        cross
    };

    let mut ctx = Ctx {
        msg_info,
        title: "zig".to_owned(),
    };
    let mut dir = PathBuf::new();
    ctx.step("clone", |ctx| {
        dir = git_clone(ctx.msg_info, RUST_CPP_HELLO_WORLD, false)?;
        Ok(())
    })?;
    for target in ZIG_TARGETS {
        ctx.step(&format!("build {target}"), |ctx| {
            cross(target).current_dir(&dir).run(ctx.msg_info, false)?;
            // note: ensure #724 doesn't replicate during CI.
            // https://github.com/cross-rs/cross/issues/724
            command("cargo")
                .arg("clean")
                .current_dir(&dir)
                .run(ctx.msg_info, false)
        })?;
    }
    Ok(())
}
