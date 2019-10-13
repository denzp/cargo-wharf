use std::convert::TryFrom;
use std::path::PathBuf;

use failure::{Error, ResultExt};
use serde::Serialize;

use buildkit_frontend::{Bridge, Options};
use buildkit_llb::prelude::*;

use crate::query::Mode;
use crate::shared::{tools, DOCKERFILE, DOCKERFILE_PATH};

mod base;
mod builder;
mod output;

pub use self::base::{BinaryDefinition, ConfigBase};
pub use self::builder::BuilderImage;
pub use self::output::OutputImage;

const OUTPUT_LAYER_PATH: &str = "/output";
const OUTPUT_NAME: &str = "build-config.json";

#[derive(Debug, Serialize)]
pub struct Config {
    builder: BuilderImage,
    output: OutputImage,
    mode: Mode,

    binaries: Vec<BinaryDefinition>,
}

impl Config {
    pub async fn analyse(bridge: &mut Bridge, options: &Options) -> Result<Self, Error> {
        let manifest_path = options.get("filename").unwrap_or("Cargo.toml");

        let command = {
            Command::run(tools::METADATA_COLLECTOR)
                .args(&[
                    "--manifest-path",
                    &PathBuf::from(DOCKERFILE_PATH)
                        .join(manifest_path)
                        .to_string_lossy(),
                ])
                .args(&[
                    "--output",
                    &PathBuf::from(OUTPUT_LAYER_PATH)
                        .join(OUTPUT_NAME)
                        .to_string_lossy(),
                ])
                .cwd(DOCKERFILE_PATH)
                .mount(Mount::Layer(OutputIdx(0), tools::IMAGE.output(), "/"))
                .mount(Mount::ReadOnlyLayer(DOCKERFILE.output(), DOCKERFILE_PATH))
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
                .context("Unable to analyse builder image")?
        };

        let output = {
            OutputImage::analyse(bridge, base.output)
                .await
                .context("Unable to analyse output image")?
        };

        let mode = {
            options
                .get("mode")
                .map(Mode::try_from)
                .unwrap_or(Ok(Mode::Binaries))
                .context("Unable to parse the mode")?
        };

        Ok(Self {
            builder,
            output,
            mode,

            binaries: base.binaries,
        })
    }

    #[cfg(test)]
    pub fn new(
        builder: BuilderImage,
        output: OutputImage,
        mode: Mode,
        binaries: Vec<BinaryDefinition>,
    ) -> Self {
        Self {
            builder,
            output,
            mode,
            binaries,
        }
    }

    pub fn builder_image(&self) -> &BuilderImage {
        &self.builder
    }

    pub fn output_image(&self) -> &OutputImage {
        &self.output
    }

    pub fn find_binary(&self, name: &str) -> Option<&BinaryDefinition> {
        self.binaries.iter().find(|bin| bin.name == name)
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }
}
