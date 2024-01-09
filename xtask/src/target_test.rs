use clap::Args;

use crate::util::{ImageTarget, cargo_metadata};

#[derive(Args, Debug)]
struct TargetTest {
    #[arg(long, required = true)]
    target: ImageTarget,
}

pub fn test(args: _, as_deref: Option<&str>, msg_info: &mut cross::shell::MessageInfo) -> cross::Result<()> {
    let metadata = cargo_metadata(msg_info)?;
    Ok(())
}

