use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use failure::{Error, ResultExt};
use serde::Serialize;

use buildkit_frontend::Bridge;
use buildkit_llb::ops::source::ImageSource;
use buildkit_llb::prelude::*;

use crate::query::Profile;
use crate::shared::{tools, DOCKERFILE, DOCKERFILE_PATH};

mod base;
mod builder;
mod output;

pub use self::base::{BaseConfig, BinaryDefinition, CustomCommand, CustomCommandKind, StaticAssetDefinition};
pub use self::builder::BuilderConfig;
pub use self::output::OutputConfig;
pub use crate::frontend::Options;

const OUTPUT_LAYER_PATH: &str = "/output";
const OUTPUT_NAME: &str = "build-config.json";

#[derive(Debug, Serialize)]
pub struct Config {
    builder: BuilderConfig,
    output: OutputConfig,
    profile: Profile,
    manifest_path: PathBuf,

    default_features: bool,
    enabled_features: Vec<String>,

    binaries: Vec<BinaryDefinition>,
}

pub trait BaseImageConfig {
    fn populate_env<'a>(&self, command: Command<'a>) -> Command<'a>;
    fn image_source(&self) -> Option<&ImageSource>;
}

impl Config {
    pub async fn analyse(bridge: &mut Bridge, options: &Options) -> Result<Self, Error> {
        let metadata_manifest_path = {
            options
                .filename
                .clone()
                .unwrap_or_else(|| PathBuf::from("Cargo.toml"))
        };

        let args = vec![
            String::from("--manifest-path"),
            PathBuf::from(DOCKERFILE_PATH)
                .join(&metadata_manifest_path)
                .to_string_lossy()
                .into(),
            String::from("--output"),
            PathBuf::from(OUTPUT_LAYER_PATH)
                .join(OUTPUT_NAME)
                .to_string_lossy()
                .into(),
        ];

        let command = {
            Command::run(tools::METADATA_COLLECTOR)
                .args(args)
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

        let base: BaseConfig = {
            serde_json::from_slice(&metadata).context("Unable to parse configuration metadata")?
        };

        let builder = {
            BuilderConfig::analyse(bridge, base.builder)
                .await
                .context("Unable to analyse builder image")?
        };

        let output = {
            OutputConfig::analyse(bridge, base.output)
                .await
                .context("Unable to analyse output image")?
        };

        let manifest_path = {
            options
                .manifest_path
                .clone()
                .unwrap_or(metadata_manifest_path)
        };

        Ok(Self {
            builder,
            output,
            manifest_path,

            profile: options.profile,
            default_features: !options.no_default_features,
            enabled_features: options.features.clone(),

            binaries: base.binaries,
        })
    }

    #[cfg(test)]
    pub fn mocked_new(
        builder: BuilderConfig,
        output: OutputConfig,
        profile: Profile,
        binaries: Vec<BinaryDefinition>,
    ) -> Self {
        Self {
            builder,
            output,
            profile,
            binaries,
            manifest_path: PathBuf::from("Cargo.toml"),
            default_features: false,
            enabled_features: vec![],
        }
    }

    pub fn builder(&self) -> &BuilderConfig {
        &self.builder
    }

    pub fn output(&self) -> &OutputConfig {
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

    pub fn manifest_path(&self) -> &Path {
        self.manifest_path.as_path()
    }
}

fn merge_spec_and_overriden_env(
    spec_env: &Option<BTreeMap<String, String>>,
    overriden_env: &Option<BTreeMap<String, String>>,
) -> BTreeMap<String, String> {
    match (spec_env.clone(), overriden_env.clone()) {
        (Some(mut spec), Some(mut config)) => {
            spec.append(&mut config);
            spec
        }

        (spec, config) => spec.or(config).unwrap_or_default(),
    }
}
