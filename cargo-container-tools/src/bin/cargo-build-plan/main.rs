#![deny(warnings)]
#![deny(clippy::all)]

use std::env::{current_dir, current_exe};
use std::fs::File;
use std::io::{copy, BufWriter};
use std::process::{exit, Command, Stdio};
use std::sync::Arc;

use cargo::core::compiler::{BuildConfig, CompileMode, DefaultExecutor, Executor};
use cargo::core::{Shell, Workspace};
use cargo::ops::{CompileFilter, CompileOptions, FilterRule, LibRule, Packages};
use cargo::util::{config::Config, CargoResult};

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
                Arg::with_name("release")
                    .long("release")
                    .takes_value(false)
                    .help("Build artifacts in release mode, with optimizations")
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

    if let Some(path) = matches.value_of("manifest") {
        process.arg("--manifest-path").arg(path);
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
    config.configure(0, None, &None, false, true, false, &None, &[])?;

    let mut build_config = BuildConfig::new(&config, Some(1), &None, CompileMode::Build)?;
    build_config.release = matches.is_present("release");
    build_config.force_rebuild = true;
    build_config.build_plan = true;

    let options = CompileOptions {
        config: &config,
        build_config,

        features: vec![],
        all_features: false,
        no_default_features: false,

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
        export_dir: None,
    };

    let executor: Arc<dyn Executor> = Arc::new(DefaultExecutor);
    let ws = Workspace::new(
        &current_dir()?.join(matches.value_of("manifest").unwrap()),
        &config,
    )?;

    cargo::ops::compile_ws(&ws, &options, &executor)?;
    Ok(())
}
