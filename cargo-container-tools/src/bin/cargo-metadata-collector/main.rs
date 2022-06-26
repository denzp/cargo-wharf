#![deny(warnings)]
#![deny(clippy::all)]

use std::env::current_dir;
use std::fs::{read_to_string, File};
use std::io::{stdout, BufWriter};
use std::iter::once;
use std::process::exit;

use cargo::core::package::Package;
use cargo::core::{Shell, Workspace};
use cargo::util::{config::Config, CargoResult};

use clap::{crate_authors, crate_version, App, Arg, ArgMatches};
use either::Either;
use toml_edit::easy::Value;

use cargo_container_tools::metadata::*;

fn main() {
    let matches = get_cli_app().get_matches();

    if let Err(error) = run(&matches) {
        cargo::display_error(&error, &mut Shell::new());
        exit(1);
    }
}

fn get_cli_app() -> App<'static, 'static> {
    App::new("cargo-metadata-collector")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Tiny Rust [package.metadata] collector")
        .args(&[
            {
                Arg::with_name("output")
                    .long("output")
                    .takes_value(true)
                    .value_name("PATH")
                    .default_value("-")
                    .help("Metadata output path (or '-' for STDOUT)")
            },
            {
                Arg::with_name("manifest")
                    .long("manifest-path")
                    .takes_value(true)
                    .value_name("PATH")
                    .default_value("Cargo.toml")
                    .help("Path to Cargo.toml")
            },
        ])
}

fn run(matches: &ArgMatches<'static>) -> CargoResult<()> {
    let config = Config::default()?;
    let ws = Workspace::new(
        &current_dir()?.join(matches.value_of("manifest").unwrap()),
        &config,
    )?;

    let metadata = {
        ws.current_opt()
            .map(package_metadata)
            .map(|metadata| vec![metadata])
            .unwrap_or_else(|| workspace_metadata(&ws))
    };

    let writer = BufWriter::new(match matches.value_of("output").unwrap() {
        "-" => Either::Left(stdout()),
        path => Either::Right(File::create(path)?),
    });

    serde_json::to_writer_pretty(writer, &metadata)?;

    Ok(())
}

fn package_metadata(package: &Package) -> Metadata {
    Metadata {
        origin: Origin::Package {
            name: package.name().to_string(),
            version: package.version().clone(),
        },

        metadata: package.manifest().custom_metadata().cloned(),
    }
}

fn workspace_metadata(ws: &Workspace) -> Vec<Metadata> {
    let root_metadata = Metadata {
        origin: Origin::WorkspaceRoot,
        metadata: workspace_root_inner_metadata(ws),
    };

    once(root_metadata)
        .chain(ws.members().map(package_metadata))
        .collect()
}

fn workspace_root_inner_metadata(ws: &Workspace) -> Option<Value> {
    let manifest_contents = read_to_string(ws.root().join("Cargo.toml")).ok()?;
    let manifest: manifest::Root = toml_edit::easy::from_str(&manifest_contents).ok()?;

    manifest.workspace?.metadata
}
