use std::env::current_dir;
use std::path::{Path, PathBuf};

use cargo::core::{Shell, Workspace};
use cargo::util::{homedir, CargoResult, Config as CargoConfig};

use clap::{crate_version, App, AppSettings, Arg, ArgMatches};
use failure::{bail, format_err};

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

    run_command(matches).unwrap(); // TODO(denzp): handle errors here correctly
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

fn run_command(matches: ArgMatches<'static>) -> CargoResult<()> {
    let cargo_config = match matches.value_of("crate_root") {
        None => CargoConfig::default()?,

        Some(path) => {
            let crate_path = if Path::new(path).is_absolute() {
                PathBuf::from(path)
            } else {
                current_dir()?.join(path)
            };

            let homedir = homedir(&crate_path).ok_or_else(|| {
                format_err!(
                    "Cargo couldn't find your home directory. \
                     This probably means that $HOME was not set."
                )
            })?;

            CargoConfig::new(Shell::new(), crate_path, homedir)
        }
    };

    let workspace = Workspace::new(&cargo_config.cwd().join("Cargo.toml"), &cargo_config)?;
    let config = Config::from_cargo_workspace(&workspace)?;

    match matches.subcommand() {
        ("generate", Some(matches)) => commands::GenerateCommand::run(&config, matches),
        ("build", Some(matches)) => commands::BuildCommand::run(&config, matches),
        ("test", Some(matches)) => commands::TestCommand::run(&config, matches),

        (command, _) => {
            bail!("Unknown command: {}", command);
        }
    }
}
