#![deny(warnings)]
#![deny(clippy::all)]

use std::process::{exit, Command, Stdio};

use cargo::core::Shell;
use cargo::util::CargoResult;

use clap::{crate_authors, crate_version, App, Arg, ArgMatches};
use failure::{bail, ResultExt};

fn main() {
    let matches = get_cli_app().get_matches();

    if let Err(error) = run(&matches) {
        cargo::handle_error(&error, &mut Shell::new());
        exit(1);
    }
}

fn get_cli_app() -> App<'static, 'static> {
    App::new("cargo-test-runner")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Tiny Rust tests runner")
        .arg(
            Arg::with_name("binaries")
                .value_name("BINARY")
                .multiple(true)
                .help("Test binaries to run"),
        )
}

fn run(matches: &ArgMatches<'static>) -> CargoResult<()> {
    for binary in matches.values_of("binaries").unwrap_or_default() {
        let child = Command::new(binary)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .context("Unable to start a test")?;

        if !child.status.success() {
            bail!("Test failed!");
        }
    }

    Ok(())
}
