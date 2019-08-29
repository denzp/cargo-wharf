#![deny(warnings)]
#![deny(clippy::all)]

use std::process::{exit, Command, Stdio};

use cargo::core::Shell;
use cargo::util::CargoResult;
use clap::{crate_authors, crate_version, App, Arg, ArgMatches};
use failure::{bail, ResultExt};

use cargo_container_tools::BuildScriptOutput;

fn main() {
    let matches = get_cli_app().get_matches();

    if let Err(error) = run(&matches) {
        cargo::handle_error(&error, &mut Shell::new());
        exit(1);
    }
}

fn get_cli_app() -> App<'static, 'static> {
    App::new("cargo-buildscript-apply")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Tiny Rust buildscript output adapter")
        .args(&[
            {
                Arg::with_name("rustc_path")
                    .required(true)
                    .value_name("RUSTC")
                    .help("Path to a rustc binary")
            },
            {
                Arg::with_name("rustc_args")
                    .value_name("ARG")
                    .multiple(true)
                    .help("Args to pass into rustc")
            },
        ])
}

fn run(matches: &ArgMatches<'static>) -> CargoResult<()> {
    let output = BuildScriptOutput::deserialize()?;

    invoke_rustc(
        matches.value_of("rustc_path").unwrap(),
        matches.values_of("rustc_args").unwrap_or_default(),
        output,
    )
}

fn invoke_rustc<'a>(
    bin_path: &'a str,
    bin_args: impl Iterator<Item = &'a str>,
    overrides: BuildScriptOutput,
) -> CargoResult<()> {
    let mut command = Command::new(bin_path);

    command.stderr(Stdio::inherit());
    command.stdout(Stdio::inherit());

    command.envs(overrides.env.into_iter());
    command.args(bin_args);

    for cfg in overrides.cfgs {
        command.arg("--cfg").arg(cfg);
    }

    for path in overrides.library_paths {
        command.arg("-L").arg(path);
    }

    for library in overrides.library_links {
        command.arg("-l").arg(library);
    }

    let output = command
        .output()
        .with_context(|_| format!("Unable to spawn '{}'", bin_path))?;

    if output.status.success() {
        Ok(())
    } else {
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        bail!("Compilation failed. Exit status: {}", output.status);
    }
}
