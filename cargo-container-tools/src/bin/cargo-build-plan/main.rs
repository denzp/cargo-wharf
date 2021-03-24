#![deny(warnings)]
#![deny(clippy::all)]

use std::env::{current_dir, current_exe};
use std::fs::File;
use std::io::{copy, BufWriter};
use std::process::{exit, Command, Stdio};
use std::sync::Arc;

use cargo::core::{Shell, Workspace};
use cargo::ops::{CompileFilter, CompileOptions, FilterRule, LibRule, Packages};
use cargo::util::{config::Config, CargoResult};
use cargo::{
    core::compiler::{BuildConfig, CompileMode, DefaultExecutor, Executor},
    util::interning::InternedString,
};

use anyhow::{bail, Context};
use clap::{crate_authors, crate_version, App, Arg, ArgMatches};

fn main() {
    let matches = get_cli_app().get_matches();

    if let Err(error) = run(&matches) {
        cargo::display_error(&error, &mut Shell::new());
        exit(1);
    }
}

fn get_cli_app() -> App<'static, 'static> {
    App::new("cargo-build-plan")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Tiny Rust build plan writer")
        .args(&[
            {
                Arg::with_name("output")
                    .long("output")
                    .takes_value(true)
                    .value_name("PATH")
                    .default_value("-")
                    .help("Build plan output path (or '-' for STDOUT)")
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
                Arg::with_name("target")
                    .long("target")
                    .takes_value(true)
                    .value_name("TARGET")
                    .help("Target triple for which the code is compiled")
            },
            {
                Arg::with_name("release")
                    .long("release")
                    .takes_value(false)
                    .help("Build artifacts in release mode, with optimizations")
            },
            {
                Arg::with_name("no_default_features")
                    .long("no-default-features")
                    .takes_value(false)
                    .help("Disable crate default features")
            },
            {
                Arg::with_name("features")
                    .long("feature")
                    .takes_value(true)
                    .value_name("NAME")
                    .multiple(true)
                    .help("Target triple for which the code is compiled")
            },
        ])
}

fn run(matches: &ArgMatches<'static>) -> CargoResult<()> {
    let mut writer = BufWriter::new(match matches.value_of("output").unwrap() {
        "-" => return run_stdout(matches),
        path => File::create(path)?,
    });

    let mut process = Command::new(current_exe()?);

    process.stdout(Stdio::piped());
    process.stderr(Stdio::inherit());

    if matches.is_present("release") {
        process.arg("--release");
    }

    if matches.is_present("no_default_features") {
        process.arg("--no-default-features");
    }

    if let Some(path) = matches.value_of("manifest") {
        process.arg("--manifest-path").arg(path);
    }

    if let Some(target) = matches.value_of("target") {
        process.arg("--target").arg(target);
    }

    for feature in matches.values_of("features").unwrap_or_default() {
        process.arg("--feature").arg(feature);
    }

    let mut child = process.spawn()?;

    copy(&mut child.stdout.take().unwrap(), &mut writer)
        .context("Unable to copy child stdout into output")?;

    let exit_code = child.wait().context("Failed to wait on child")?;

    if !exit_code.success() {
        bail!("Child process failed");
    }

    Ok(())
}

fn run_stdout(matches: &ArgMatches<'static>) -> CargoResult<()> {
    let mut config = Config::default()?;
    config.configure(0, false, None, false, true, false, &None, &[], &[])?;

    let mut build_config = BuildConfig::new(
        &config,
        Some(1),
        matches
            .value_of("target")
            .unwrap_or_default()
            .split(",")
            .map(String::from)
            .collect::<Vec<_>>()
            .as_slice(),
        CompileMode::Build,
    )?;
    if matches.is_present("release") {
        build_config.requested_profile = InternedString::new("release");
    }
    build_config.force_rebuild = true;
    build_config.build_plan = true;

    let features = {
        matches
            .values_of("features")
            .unwrap_or_default()
            .map(String::from)
            .collect()
    };

    let options = CompileOptions {
        build_config,

        features,
        all_features: false,
        no_default_features: matches.is_present("no_default_features"),

        spec: Packages::All,
        filter: CompileFilter::Only {
            all_targets: true,
            lib: LibRule::Default,
            bins: FilterRule::All,
            examples: FilterRule::All,
            tests: FilterRule::All,
            benches: FilterRule::All,
        },

        target_rustdoc_args: None,
        target_rustc_args: None,
        local_rustdoc_args: None,
        rustdoc_document_private_items: false,
    };

    let executor: Arc<dyn Executor> = Arc::new(DefaultExecutor);
    let ws = Workspace::new(
        &current_dir()?.join(matches.value_of("manifest").unwrap()),
        &config,
    )?;

    cargo::ops::compile_ws(&ws, &options, &executor)?;
    Ok(())
}
