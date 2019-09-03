use std::path::PathBuf;

use failure::{Error, ResultExt};
use serde::Serialize;

use buildkit_frontend::Bridge;
use buildkit_llb::prelude::*;

use crate::shared::{tools, CONTEXT, CONTEXT_PATH};

mod base;
mod builder;

use self::base::{BinaryDefinition, ConfigBase, OutputConfig};
use self::builder::BuilderImage;

const OUTPUT_LAYER_PATH: &str = "/output";
const OUTPUT_NAME: &str = "build-plan.json";

#[derive(Debug, Serialize)]
pub struct Config {
    builder: BuilderImage,
    output: OutputConfig,

    binaries: Vec<BinaryDefinition>,
}

impl Config {
    pub async fn analyse(bridge: &mut Bridge) -> Result<Self, Error> {
        let command = {
            Command::run(tools::METADATA_COLLECTOR)
                .args(&[
                    "--manifest-path",
                    &PathBuf::from(CONTEXT_PATH)
                        .join("Cargo.toml")
                        .to_string_lossy(),
                ])
                .args(&[
                    "--output",
                    &PathBuf::from(OUTPUT_LAYER_PATH)
                        .join(OUTPUT_NAME)
                        .to_string_lossy(),
                ])
                .cwd(CONTEXT_PATH)
                .mount(Mount::Layer(OutputIdx(0), tools::IMAGE.output(), "/"))
                .mount(Mount::ReadOnlyLayer(CONTEXT.output(), CONTEXT_PATH))
                .mount(Mount::Scratch(OutputIdx(1), OUTPUT_LAYER_PATH))
                .custom_name("Collecting configuration metadata")
        };

        let metadata_layer = {
            bridge
                .solve(Terminal::with(command.output(1)))
                .await
                .context("Unable to collect metadata")?
        };

        let metadata = {
            bridge
                .read_file(&metadata_layer, OUTPUT_NAME, None)
                .await
                .context("Unable to read metadata output")?
        };

        let base: ConfigBase = {
            serde_json::from_slice(&metadata).context("Unable to parse configuration metadata")?
        };

        let builder = {
            BuilderImage::analyse(bridge, base.builder)
                .await
                .context("Unable to analyse Rust builder image")?
        };

        Ok(Self {
            builder,

            output: base.output,
            binaries: base.binaries,
        })
    }

    pub fn builder_image(&self) -> &BuilderImage {
        &self.builder
    }
}
