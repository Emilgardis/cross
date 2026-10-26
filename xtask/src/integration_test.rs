//! Integration tests of cross features.

mod bisect;
mod docker_in_docker;
mod foreign;
mod podman;
mod remote;

use std::path::Path;
use std::process::Command;

use clap::{Args, ValueEnum};
use cross::shell::MessageInfo;

use crate::test_util::{Steps, binary_path, command, image_var};

#[derive(Args, Debug)]
pub struct IntegrationTest {
    /// Test to run.
    test: Test,
    /// Target to test with.
    #[clap(long, default_value = "aarch64-unknown-linux-gnu")]
    target: String,
    /// Image to test, sets `CROSS_TARGET_<TARGET>_IMAGE`.
    #[clap(long)]
    image: Option<String>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Test {
    /// Remote docker support, `CROSS_REMOTE=1`.
    Remote,
    /// Custom toolchains with cargo-bisect-rustc.
    Bisect,
    /// Images with foreign toolchains.
    Foreign,
    /// Running cross in a docker-in-docker container.
    DockerInDocker,
    /// Running and building images with podman.
    Podman,
    /// The script `bisect` runs with cargo-bisect-rustc.
    #[value(hide = true)]
    BisectScript,
}

struct Tester<'a> {
    msg_info: &'a mut MessageInfo,
    title: String,
    target: String,
    /// `CROSS_TARGET_<TARGET>_IMAGE`, if set
    image: Option<String>,
}

impl Steps for Tester<'_> {
    fn title(&self) -> &str {
        &self.title
    }
}

impl Tester<'_> {
    fn cross(&self, dir: &Path) -> Command {
        let mut cross = command(binary_path("cross"));
        cross.current_dir(dir);
        if let Some(image) = &self.image {
            cross.env(image_var(&self.target), image);
        }
        cross
    }
}

pub fn integration_test(args: IntegrationTest, msg_info: &mut MessageInfo) -> cross::Result<()> {
    let mut tester = Tester {
        msg_info,
        title: args
            .test
            .to_possible_value()
            .expect("no skipped tests")
            .get_name()
            .to_owned(),
        target: args.target,
        // OCI does not support uppercase characters from orga/owner names
        image: args.image.map(|image| image.to_lowercase()),
    };
    match args.test {
        Test::Remote => tester.remote()?,
        Test::Bisect => tester.bisect()?,
        Test::Foreign => tester.foreign()?,
        Test::DockerInDocker => tester.docker_in_docker()?,
        Test::Podman => tester.podman()?,
        Test::BisectScript => return bisect::script(&tester.target),
    }
    tester
        .msg_info
        .print(format_args!("integration test `{}` passed", tester.title))
}
