#![warn(clippy::all)]
#![allow(clippy::needless_lifetimes, dead_code)]
#![deny(warnings)]
#![feature(async_await, existential_type)]

use env_logger::Env;
use failure::{bail, Error, ResultExt};
use futures::prelude::*;
use log::*;

use buildkit_llb::frontend::{run_frontend, Bridge, Frontend, OutputRef};
use buildkit_llb::prelude::*;

mod graph;
mod image;
mod plan;

const CONTEXT_PATH: &str = "/context";
const TARGET_PATH: &str = "/target";

use crate::graph::BuildGraph;
use crate::image::RustDockerImage;
use crate::plan::RawBuildPlan;

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
                RustDockerImage::analyse(&mut bridge, Source::image("rustlang/rust:nightly"))
                    .await
                    .context("Unable to analyse Rust builder image")?
            };

            let graph: BuildGraph = {
                RawBuildPlan::evaluate(&mut bridge, &builder_image)
                    .await
                    .context("Unable to evaluate the Cargo build plan")?
                    .into()
            };

            info!("{:#?}", graph);
            bail!("TBD");
        }
    }
}
