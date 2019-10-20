use std::convert::TryFrom;
use std::iter::empty;
use std::path::PathBuf;

use either::Either;
use failure::{Error, ResultExt};
use serde::Serialize;

use buildkit_frontend::{Bridge, Options};
use buildkit_llb::prelude::*;

use crate::query::Profile;
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
    profile: Profile,

    default_features: bool,
    enabled_features: Vec<String>,

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

        let profile = {
            options
                .get("profile")
                .map(Profile::try_from)
                .unwrap_or(Ok(Profile::ReleaseBinaries))
                .context("Unable to parse the mode")?
        };

        let enabled_features = {
            options
                .iter("features")
                .map(Either::Left)
                .unwrap_or_else(|| Either::Right(empty()))
                .map(String::from)
                .collect()
        };

        Ok(Self {
            builder,
            output,
            profile,

            default_features: !options.is_flag_set("no-default-features"),
            enabled_features,

            binaries: base.binaries,
        })
    }

    #[cfg(test)]
    pub fn new(
        builder: BuilderImage,
        output: OutputImage,
        profile: Profile,
        binaries: Vec<BinaryDefinition>,
    ) -> Self {
        Self {
            builder,
            output,
            profile,
            binaries,
            default_features: false,
            enabled_features: vec![],
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

    pub fn profile(&self) -> Profile {
        self.profile
    }

    pub fn default_features(&self) -> bool {
        self.default_features
    }

    pub fn enabled_features(&self) -> impl Iterator<Item = &str> {
        self.enabled_features.iter().map(String::as_str)
    }
}
