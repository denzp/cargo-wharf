use std::collections::BTreeMap;
use std::fs::File;
use std::path::Path;
use std::process::{exit, Command, Stdio};

use cargo::core::{compiler::BuildOutput, Shell};
use cargo::util::CargoResult;
use clap::{crate_version, App, Arg, ArgMatches};
use failure::{bail, ResultExt};
use serde_json::from_reader;

fn main() {
    let matches = get_cli_app().get_matches();

    if let Err(error) = run_build(&matches) {
        cargo::handle_error(&error, &mut Shell::new());
        exit(1);
    }
}

fn get_cli_app() -> App<'static, 'static> {
    App::new("cargo-buildscript")
        .version(crate_version!())
        .author("Denys Zariaiev <denys.zariaiev@gmail.com>")
        .about("Tiny Rust buildscript controller for Cargo-less environments.")
        .args(&[
            {
                Arg::with_name("buildscript_path")
                    .takes_value(true)
                    .required(true)
                    .value_name("BUILDSCRIPT_PATH")
                    .help("Path to a buildscript binary")
            },
            {
                Arg::with_name("buildscript_env")
                    .long("buildscript-env")
                    .takes_value(true)
                    .required(true)
                    .value_name("PATH")
                    .help("Path to a file with buildscript env variables")
            },
            {
                Arg::with_name("rustc_env")
                    .long("rustc-env")
                    .takes_value(true)
                    .required(true)
                    .value_name("PATH")
                    .help("Path to a file with Rustc env variables")
            },
            {
                Arg::with_name("rustc_args")
                    .long("rustc-args")
                    .takes_value(true)
                    .required(true)
                    .value_name("PATH")
                    .help("Path to a file with Rustc arguments")
            },
        ])
}

fn run_build(matches: &ArgMatches<'static>) -> CargoResult<()> {
    let buildscript_env_file = {
        File::open(matches.value_of("buildscript_env").unwrap())
            .context("Unable to open buildscript environment variables file")?
    };

    let buildscript_env: BTreeMap<String, String> = {
        from_reader(buildscript_env_file)
            .context("Unable to parse buildscript environment variables file")?
    };

    let buildscript_output = get_buildscript_output(
        matches.value_of("buildscript_path").unwrap(),
        buildscript_env,
    )?;

    let rustc_env_file = {
        File::open(matches.value_of("rustc_env").unwrap())
            .context("Unable to open Rust environment variables file")?
    };

    let rustc_args_file = {
        File::open(matches.value_of("rustc_args").unwrap())
            .context("Unable to open Rust arguments file")?
    };

    let rustc_env: BTreeMap<String, String> =
        from_reader(rustc_env_file).context("Unable to parse Rust environment variables file")?;

    let rustc_args: Vec<String> =
        from_reader(rustc_args_file).context("Unable to parse Rust arguments file")?;

    invoke_rustc(&rustc_args, rustc_env.into_iter(), buildscript_output)
}

fn get_buildscript_output(path: &str, envs: BTreeMap<String, String>) -> CargoResult<BuildOutput> {
    let package_name = {
        envs.get("CARGO_PKG_NAME")
            .cloned()
            .unwrap_or_else(|| String::from("buildscript"))
    };

    let out_dir = {
        envs.get("OUT_DIR")
            .cloned()
            .unwrap_or_else(|| String::from("/rust-src"))
    };

    let buildscript_stdout = {
        invoke_buildscript(path, envs.into_iter())
            .context("Unable to collect buildscript output")?
    };

    BuildOutput::parse(
        &buildscript_stdout,
        &package_name,
        &Path::new(&out_dir),
        &Path::new(&out_dir),
    )
}

fn invoke_buildscript(
    path: &str,
    envs: impl Iterator<Item = (String, String)>,
) -> CargoResult<Vec<u8>> {
    let mut command = Command::new(path);

    command.stderr(Stdio::inherit());
    command.stdout(Stdio::piped());

    let output = {
        command
            .envs(envs)
            .output()
            .with_context(|_| format!("Unable to spawn '{}'", path))?
    };

    if output.status.success() {
        Ok(output.stdout)
    } else {
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        bail!("Buildscript failed. Exit status: {}", output.status);
    }
}

fn invoke_rustc(
    args: &[String],
    envs: impl Iterator<Item = (String, String)>,
    overrides: BuildOutput,
) -> CargoResult<()> {
    let mut command = Command::new("rustc");

    command.stderr(Stdio::inherit());
    command.stdout(Stdio::inherit());

    command.envs(envs);
    command.envs(overrides.env.into_iter());
    command.args(args);

    for cfg in overrides.cfgs {
        command.arg("--cfg").arg(cfg);
    }

    for path in overrides.library_paths {
        command.arg("-L").arg(path);
    }

    for library in overrides.library_links {
        command.arg("-l").arg(library);
    }

    let output = command.output().context("Unable to spawn 'rustc'")?;

    if output.status.success() {
        Ok(())
    } else {
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        bail!("Compilation failed. Exit status: {}", output.status);
    }
}
