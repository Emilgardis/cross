//! Integration tests for a target in targets.toml.

mod cross_image;
mod image;
mod zig;

use clap::Args;
use clap::builder::BoolishValueParser;
use cross::shell::MessageInfo;

use crate::util::{ImageTarget, get_matrix};

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
    let target = args.target.clone();
    match ci_target.target.as_str() {
        "zig" => zig::test(args.image, msg_info)?,
        "cross" => cross_image::test(args.image, msg_info)?,
        target if ci_target.special => eyre::bail!("`{target}` has no integration tests"),
        _ => image::test(args, ci_target, toolchain, msg_info)?,
    }
    msg_info.print(format_args!("target test `{target}` passed"))
}
