#![deny(warnings)]
#![deny(clippy::all)]

use std::collections::BTreeMap;
use std::path::Path;
use std::process::{exit, Command, Stdio};

use anyhow::{bail, Context};
use cargo::core::{compiler::BuildOutput, Shell};
use cargo::util::CargoResult;
use clap::{crate_authors, crate_version, App, Arg, ArgMatches};

use cargo_container_tools::{BuildScriptOutput, RuntimeEnv};

fn main() {
    let matches = get_cli_app().get_matches();

    if let Err(error) = run(&matches) {
        cargo::display_error(&error, &mut Shell::new());
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
            {
                Arg::with_name("metadata_from")
                    .long("with-metadata-from")
                    .takes_value(true)
                    .value_name("PATH")
                    .multiple(true)
                    .help("Dependency build script outputs")
            },
        ])
}

fn run(matches: &ArgMatches<'static>) -> CargoResult<()> {
    let metadata = get_dependencies_metadata(
        matches
            .values_of("metadata_from")
            .unwrap_or_default()
            .map(Path::new),
    )?;

    let output = get_buildscript_output(
        matches.value_of("buildscript_path").unwrap(),
        matches.values_of("buildscript_args").unwrap_or_default(),
        metadata,
    )?;

    output.serialize()
}

fn get_buildscript_output<'a>(
    bin_path: &'a str,
    bin_args: impl Iterator<Item = &'a str>,
    metadata: BTreeMap<String, String>,
) -> CargoResult<BuildScriptOutput> {
    let mut command = Command::new(bin_path);

    command.stderr(Stdio::inherit());
    command.stdout(Stdio::piped());
    command.envs(metadata);

    let output = {
        command
            .args(bin_args)
            .output()
            .with_context(|| format!("Unable to spawn '{}'", bin_path))?
    };

    let cargo_output_result = BuildOutput::parse(
        &output.stdout,
        RuntimeEnv::package_name()?,
        RuntimeEnv::output_dir()?,
        RuntimeEnv::output_dir()?,
        false,
    );

    let cargo_output = match cargo_output_result {
        Ok(output) => output,

        Err(error) => {
            eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
            eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
            return Err(error);
        }
    };

    for msg in &cargo_output.warnings {
        eprintln!("warning: {}", msg);
    }

    if !output.status.success() {
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        bail!("Buildscript failed. Exit status: {}", output.status);
    }

    Ok(cargo_output.into())
}

fn get_dependencies_metadata<'a>(
    paths: impl Iterator<Item = &'a Path>,
) -> CargoResult<BTreeMap<String, String>> {
    let mut metadata = BTreeMap::default();

    for path in paths {
        let output = BuildScriptOutput::deserialize_from_dir(path)
            .context("Unable to open dependency build script output")?;

        if let Some(ref name) = output.link_name {
            let name = envify(&name);

            for (key, value) in output.metadata {
                metadata.insert(format!("DEP_{}_{}", name, envify(&key)), value);
            }
        }
    }

    Ok(metadata)
}

fn envify(s: &str) -> String {
    s.chars()
        .flat_map(|c| c.to_uppercase())
        .map(|c| if c == '-' { '_' } else { c })
        .collect()
}
