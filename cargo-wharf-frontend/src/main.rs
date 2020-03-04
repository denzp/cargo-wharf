#![warn(clippy::all)]
#![deny(warnings)]

use env_logger::Env;
use log::*;

use buildkit_frontend::run_frontend;

mod config;
mod debug;
mod frontend;
mod graph;
mod plan;
mod query;
mod shared;

use self::frontend::CargoFrontend;

#[tokio::main]
async fn main() {
    env_logger::init_from_env(Env::default().filter_or("RUST_LOG", "info"));

    if let Err(error) = run_frontend(CargoFrontend).await {
        error!("{}", error);

        for cause in error.iter_causes() {
            error!("  caused by: {}", cause);
        }

        std::process::exit(1);
    }
}
