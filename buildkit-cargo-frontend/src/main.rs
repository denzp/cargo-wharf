#![deny(warnings)]
#![warn(clippy::all)]
#![feature(async_await, existential_type)]

use env_logger::Env;
use failure::Error;
use futures::prelude::*;
use log::*;

use buildkit_llb::frontend::{run_frontend, Bridge, Frontend};
use buildkit_llb::prelude::*;

#[runtime::main(runtime_tokio::Tokio)]
async fn main() {
    env_logger::init_from_env(
        Env::default().filter_or("RUST_LOG", "info,buildkit_cargo_frontend=debug"),
    );

    if let Err(error) = run_frontend(CargoFrontend).await {
        error!("error: {:?}", error);
        std::process::exit(1);
    }
}

struct CargoFrontend;

impl Frontend for CargoFrontend {
    existential type RunFuture: Future<Output = Result<(), Error>>;

    fn run(self, mut bridge: Bridge) -> Self::RunFuture {
        async move {
            let builder_image = {
                Source::image("library/alpine:latest")
                    .custom_name("Using alpine:latest as a builder")
            };

            let command = {
                Command::run("/bin/sh")
                    .args(&["-c", "echo 'test string 5' > /out/file0"])
                    .custom_name("create a dummy file")
                    .mount(Mount::ReadOnlyLayer(builder_image.output(), "/"))
                    .mount(Mount::Scratch(OutputIdx(0), "/out"))
            };

            let fs = {
                FileSystem::sequence()
                    .custom_name("do multiple file system manipulations")
                    .append(
                        FileSystem::copy()
                            .from(LayerPath::Other(command.output(0), "/file0"))
                            .to(OutputIdx(0), LayerPath::Other(command.output(0), "/file1")),
                    )
                    .append(
                        FileSystem::copy()
                            .from(LayerPath::Own(OwnOutputIdx(0), "/file0"))
                            .to(OutputIdx(1), LayerPath::Own(OwnOutputIdx(0), "/file2")),
                    )
            };

            bridge.solve(Terminal::with(fs.output(1))).await;
            Ok(())
        }
    }
}
