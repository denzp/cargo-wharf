use std::env::current_dir;
use std::path::PathBuf;

use cargo::util::CargoResult;
use clap::{crate_version, App, AppSettings, Arg, ArgMatches};
use failure::bail;

mod commands;
mod config;
mod engine;
mod graph;
mod path;
mod plan;

use crate::commands::SubCommand;
use crate::config::Config;

fn main() {
    let matches = get_cli_app().get_matches();

    run_command(&matches).unwrap(); // TODO(denzp): handle errors here correctly
}

fn get_cli_app() -> App<'static, 'static> {
    App::new("cargo-wharf")
        .version(crate_version!())
        .author("Denys Zariaiev <denys.zariaiev@gmail.com>")
        .about("Container builder for Rust ecosystem")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .setting(AppSettings::VersionlessSubcommands)
        .subcommands(vec![
            commands::GenerateCommand::api(),
            commands::BuildCommand::api(),
            commands::TestCommand::api(),
        ])
        .args(&[
            {
                Arg::with_name("crate_root")
                    .long("crate-root")
                    .value_name("PATH")
                    .takes_value(true)
            },
            {
                Arg::with_name("engine")
                    .long("engine")
                    .value_name("NAME")
                    .takes_value(true)
                    .possible_values(&["docker"])
                    .default_value("docker")
            },
        ])
}

fn run_command(matches: &ArgMatches<'static>) -> CargoResult<()> {
    let root_path = match matches.value_of("crate_root") {
        None => current_dir()?,
        Some(path) => PathBuf::from(path),
    };

    let config = Config::from_workspace_root(root_path)?;

    match matches.subcommand() {
        ("generate", Some(matches)) => commands::GenerateCommand::run(&config, matches),
        ("build", Some(matches)) => commands::BuildCommand::run(&config, matches),
        ("test", Some(matches)) => commands::TestCommand::run(&config, matches),

        (command, _) => {
            bail!("Unknown command: {}", command);
        }
    }
}
