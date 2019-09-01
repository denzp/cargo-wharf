#![warn(clippy::all)]
#![allow(clippy::needless_lifetimes, dead_code)]
#![deny(warnings)]
#![feature(type_alias_impl_trait)]

use env_logger::Env;
use log::*;

use buildkit_frontend::run_frontend;

mod config;
mod frontend;
mod graph;
mod image;
mod plan;
mod query;

const CONTEXT_PATH: &str = "/context";
const TARGET_PATH: &str = "/target";

use self::frontend::CargoFrontend;

const DEFAULT_LOG_FILTER: &str = "info,cargo_buildkit=debug,buildkit=debug";

#[runtime::main(runtime_tokio::Tokio)]
async fn main() {
    env_logger::init_from_env(Env::default().filter_or("RUST_LOG", DEFAULT_LOG_FILTER));

    if let Err(error) = run_frontend(CargoFrontend).await {
        error!("{}", error);

        for cause in error.iter_causes() {
            error!("  caused by: {}", cause);
        }

        std::process::exit(1);
    }
}
