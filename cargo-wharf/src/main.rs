#![warn(clippy::all)]
#![deny(warnings)]

use std::process::exit;

use cargo::core::Shell;
use cargo::util::CargoResult;
use clap::{crate_authors, crate_version, App, AppSettings, ArgMatches};
use failure::bail;

mod commands;

use crate::commands::SubCommand;

fn main() {
    let matches = get_cli_app().get_matches();

    if let Err(error) = run_command(&matches.subcommand_matches("wharf").unwrap()) {
        cargo::handle_error(&error, &mut Shell::new());
        exit(1);
    }
}

fn get_cli_app() -> App<'static, 'static> {
    App::new("cargo-wharf")
        .bin_name("cargo")
        .version(crate_version!())
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            clap::SubCommand::with_name("wharf")
                .version(crate_version!())
                .author(crate_authors!())
                .about("Container builder for Rust ecosystem.")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .setting(AppSettings::VersionlessSubcommands)
                .subcommands(vec![
                    commands::BuildCommand::api(),
                    commands::TestCommand::api(),
                ]),
        )
}

fn run_command(matches: &ArgMatches<'static>) -> CargoResult<()> {
    match matches.subcommand() {
        ("build", Some(matches)) => commands::BuildCommand::run(matches),
        ("test", Some(matches)) => commands::TestCommand::run(matches),

        (command, _) => {
            bail!("Unknown command: {}", command);
        }
    }
}
