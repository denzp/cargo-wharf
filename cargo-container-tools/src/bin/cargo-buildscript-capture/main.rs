#![deny(warnings)]
#![deny(clippy::all)]

use std::process::{exit, Command, Stdio};

use cargo::core::{compiler::BuildOutput, Shell};
use cargo::util::CargoResult;
use clap::{crate_authors, crate_version, App, Arg, ArgMatches};
use failure::{bail, ResultExt};

use cargo_container_tools::{BuildScriptOutput, RuntimeEnv};

fn main() {
    let matches = get_cli_app().get_matches();

    if let Err(error) = run(&matches) {
        cargo::handle_error(&error, &mut Shell::new());
        exit(1);
    }
}

fn get_cli_app() -> App<'static, 'static> {
    App::new("cargo-buildscript-capture")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Tiny Rust buildscript output collector")
        .args(&[
            {
                Arg::with_name("buildscript_path")
                    .required(true)
                    .value_name("BUILDSCRIPT")
                    .help("Path to a buildscript binary")
            },
            {
                Arg::with_name("buildscript_args")
                    .value_name("ARG")
                    .multiple(true)
                    .help("Args to pass into the buildscript")
            },
        ])
}

fn run(matches: &ArgMatches<'static>) -> CargoResult<()> {
    let output = get_buildscript_output(
        matches.value_of("buildscript_path").unwrap(),
        matches.values_of("buildscript_args").unwrap_or_default(),
    )?;

    output.serialize()
}

fn get_buildscript_output<'a>(
    bin_path: &'a str,
    bin_args: impl Iterator<Item = &'a str>,
) -> CargoResult<BuildScriptOutput> {
    let mut command = Command::new(bin_path);

    command.stderr(Stdio::inherit());
    command.stdout(Stdio::piped());

    let output = {
        command
            .args(bin_args)
            .output()
            .with_context(|_| format!("Unable to spawn '{}'", bin_path))?
    };

    let buildscript_stdout = if output.status.success() {
        output.stdout
    } else {
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        bail!("Buildscript failed. Exit status: {}", output.status);
    };

    let cargo_output = BuildOutput::parse(
        &buildscript_stdout,
        RuntimeEnv::package_name()?,
        RuntimeEnv::output_dir()?,
        RuntimeEnv::output_dir()?,
    )?;

    Ok(cargo_output.into())
}
