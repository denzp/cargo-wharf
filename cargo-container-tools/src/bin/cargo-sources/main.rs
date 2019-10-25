#![deny(warnings)]
#![deny(clippy::all)]

use std::collections::HashMap;
use std::env::current_dir;
use std::fs::File;
use std::io::{stdout, BufReader, BufWriter};
use std::process::exit;

use cargo::core::{Shell, Workspace};
use cargo::ops::resolve_ws;
use cargo::util::{config::Config, CargoResult};

use clap::{crate_authors, crate_version, App, Arg, ArgMatches};
use either::Either;
use failure::ResultExt;
use semver::Version;
use serde_derive::Deserialize;

use cargo_container_tools::source::SourceKind;

fn main() {
    let matches = get_cli_app().get_matches();

    if let Err(error) = run(&matches) {
        cargo::handle_error(&error, &mut Shell::new());
        exit(1);
    }
}

fn get_cli_app() -> App<'static, 'static> {
    App::new("cargo-sources")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Tiny Rust crate dependencies sources finder")
        .args(&[
            {
                Arg::with_name("output")
                    .long("output")
                    .takes_value(true)
                    .value_name("PATH")
                    .default_value("-")
                    .help("Sources output path (or '-' for STDOUT)")
            },
            {
                Arg::with_name("manifest")
                    .long("manifest-path")
                    .takes_value(true)
                    .value_name("PATH")
                    .default_value("Cargo.toml")
                    .help("Path to Cargo.toml")
            },
            {
                Arg::with_name("build_plan")
                    .long("build-plan-path")
                    .required(true)
                    .takes_value(true)
                    .value_name("PATH")
                    .help("Path to the generated build plan")
            },
        ])
}

fn run(matches: &ArgMatches<'static>) -> CargoResult<()> {
    let config = Config::default()?;
    let ws = Workspace::new(
        &current_dir()?.join(matches.value_of("manifest").unwrap()),
        &config,
    )?;

    let build_plan_reader = BufReader::new(
        File::open(matches.value_of("build_plan").unwrap())
            .context("Unable to open the build plan")?,
    );

    let plan: BuildPlan =
        serde_json::from_reader(build_plan_reader).context("Unable to parse the build plan")?;

    let writer = BufWriter::new(match matches.value_of("output").unwrap() {
        "-" => Either::Left(stdout()),
        path => Either::Right(File::create(path)?),
    });

    serde_json::to_writer_pretty(writer, &collect_sources(&ws, plan)?)?;

    Ok(())
}

#[allow(clippy::map_entry)]
fn collect_sources(ws: &Workspace, plan: BuildPlan) -> CargoResult<HashMap<String, SourceKind>> {
    let (_, resolved) = resolve_ws(ws)?;
    let mut sources = HashMap::new();

    for invocation in plan.invocations {
        let name_and_version =
            format!("{}:{}", invocation.package_name, invocation.package_version);

        if !sources.contains_key(&name_and_version) {
            let kind = SourceKind::find(&resolved, &name_and_version)
                .with_context(|_| format!("Unable to find the source: {}", name_and_version))?;

            sources.insert(name_and_version, kind);
        }
    }

    Ok(sources)
}

#[derive(Debug, Deserialize)]
struct BuildPlan {
    invocations: Vec<Invocation>,
}

#[derive(Debug, Deserialize)]
struct Invocation {
    package_name: String,
    package_version: Version,
}
