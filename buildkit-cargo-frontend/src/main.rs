#![deny(warnings)]
#![warn(clippy::all)]
#![feature(async_await, existential_type)]

use std::path::PathBuf;

use env_logger::Env;
use failure::{bail, Error, ResultExt};
use futures::prelude::*;
use log::*;

use buildkit_llb::frontend::{run_frontend, Bridge, Frontend, OutputRef};
use buildkit_llb::prelude::*;

#[runtime::main(runtime_tokio::Tokio)]
async fn main() {
    env_logger::init_from_env(Env::default().filter_or("RUST_LOG", "info,buildkit=debug"));

    for var in std::env::vars() {
        info!("env var: {:?}", var);
    }

    if let Err(error) = run_frontend(CargoFrontend).await {
        error!("{}", error);

        for cause in error.iter_causes() {
            error!("  caused by: {}", cause);
        }

        std::process::exit(1);
    }
}

struct CargoFrontend;

impl Frontend for CargoFrontend {
    existential type RunFuture: Future<Output = Result<OutputRef, Error>>;

    fn run(self, mut bridge: Bridge) -> Self::RunFuture {
        async move {
            let builder_image = {
                Source::image("rustlang/rust:nightly")
                    .custom_name("Using Nightly Rust as a builder")
            };

            let context = {
                Source::local("context")
                    .custom_name("Using context")
                    .add_exclude_pattern("**/target")
            };

            let cargo_home = "/usr/local/cargo";

            let command = {
                Command::run("/bin/sh")
                    .args(&[
                        "-c",
                        "cargo build -Z unstable-options --build-plan --all-targets > /output/build-plan.json",
                    ])
                    .env("PATH", "/usr/local/cargo/bin")  // TODO: get it from Rust image config
                    .env("RUSTUP_HOME", "/usr/local/rustup") // TODO: get it from Rust image config
                    .env("CARGO_HOME", cargo_home) // TODO: get it from Rust image config
                    .env("CARGO_TARGET_DIR", "/target")
                    .cwd("/context")
                    .mount(Mount::Layer(OutputIdx(0), builder_image.output(), "/"))
                    .mount(Mount::ReadOnlyLayer(context.output(), "/context"))
                    .mount(Mount::Scratch(OutputIdx(1), "/output"))
                    .mount(Mount::SharedCache(PathBuf::from(cargo_home).join("git")))
                    .mount(Mount::SharedCache(PathBuf::from(cargo_home).join("registry")))
                    .custom_name("Making a build plan")
            };

            let build_plan_layer = {
                bridge
                    .solve(Terminal::with(command.output(1)))
                    .await
                    .context("Unable to evaluate a build plan")
                    .map_err(Error::from)?
            };

            let build_plan = {
                bridge
                    .read_file(&build_plan_layer, "/build-plan.json", None)
                    .await
                    .context("Unable to read a build plan")
                    .map_err(Error::from)?
            };

            info!("{}", String::from_utf8_lossy(&build_plan));

            bail!("TBD");
        }
    }
}
