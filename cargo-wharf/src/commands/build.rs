use std::io::{copy, Cursor, Read};
use std::process::{Command, Stdio};

use cargo::util::CargoResult;
use clap::{App, Arg, ArgMatches, SubCommand};
use failure::{bail, ResultExt};

use crate::config::Config;
use crate::engine::{DockerfilePrinter, OutputMode};

use super::construct_build_graph;

#[derive(Default)]
pub struct BuildCommand;

impl super::SubCommand for BuildCommand {
    fn api() -> App<'static, 'static> {
        SubCommand::with_name("build")
            .about("Creates a Docker image for the crate")
            .args(&[
                {
                    Arg::with_name("tag")
                        .short("t")
                        .long("tag")
                        .takes_value(true)
                        .value_name("NAME")
                        .multiple(true)
                        .required(true)
                        .number_of_values(1)
                        .help("Resulting image tag")
                },
                {
                    Arg::with_name("template")
                        .long("template")
                        .takes_value(true)
                        .value_name("PATH")
                        .default_value("Dockerfile.hbs")
                        .help("Dockerfile template location")
                },
            ])
    }

    fn run(config: &Config, matches: &ArgMatches<'static>) -> CargoResult<()> {
        let build_plan = cargo_get_build_plan(config).context("Unable to define a Build plan")?;
        let graph = construct_build_graph(config, Cursor::new(build_plan))?;

        let dockerfile_template = matches.value_of("template").unwrap();
        let tags = matches.values_of("tag").unwrap().collect::<Vec<_>>();

        let dockerfile = {
            DockerfilePrinter::from_template(OutputMode::Binaries, dockerfile_template)
                .context("Unable to initialize Dockerfile template")?
        };

        let mut dockerfile_contents = Vec::new();

        dockerfile
            .write(graph, &mut dockerfile_contents)
            .context("Unable to generate Dockerfile")?;

        build_docker_image(config, &tags, Cursor::new(dockerfile_contents))
            .context("Unable to build Docker image")?;

        Ok(())
    }
}

fn cargo_get_build_plan(config: &Config) -> CargoResult<Vec<u8>> {
    let mut command = Command::new("cargo");

    command.args(&["build"]);
    command.args(&["-Z", "unstable-options", "--build-plan"]);
    command.args(&["--all-targets"]);
    command.args(&[
        "--manifest-path",
        &config
            .get_local_root()
            .join("Cargo.toml")
            .display()
            .to_string(),
    ]);

    command.stderr(Stdio::inherit());
    command.stdout(Stdio::piped());

    let output = command.output().context("Unable to spawn Cargo")?;

    if output.status.success() {
        Ok(output.stdout)
    } else {
        bail!("Cargo failed: {}", output.status);
    }
}

fn build_docker_image(config: &Config, tags: &[&str], mut output: impl Read) -> CargoResult<()> {
    let mut command = Command::new("docker");

    command.args(&["build"]);
    command.args(&["-f", "-"]);

    for tag in tags {
        command.args(&["-t", tag]);
    }

    command.arg(config.get_local_root());
    command.stderr(Stdio::inherit());
    command.stdin(Stdio::piped());

    let mut child = command.spawn().context("Unable to spawn Docker")?;
    copy(&mut output, &mut child.stdin.take().unwrap())?;

    let output = child.wait_with_output()?;

    if output.status.success() {
        Ok(())
    } else {
        bail!("Docker failed: {}", output.status);
    }
}
