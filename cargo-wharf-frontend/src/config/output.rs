use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use failure::{format_err, Error, ResultExt};
use log::*;
use serde::Serialize;

use buildkit_frontend::oci::{self, Signal};
use buildkit_frontend::Bridge;
use buildkit_llb::ops::source::ImageSource;
use buildkit_llb::prelude::*;

use super::base::{BaseOutputConfig, CustomCommand};
use super::{merge_spec_and_overriden_env, BaseImageConfig, StaticAssetDefinition};

#[derive(Debug, Serialize)]
pub struct OutputConfig {
    #[serde(skip_serializing)]
    source: Option<ImageSource>,

    overrides: BaseOutputConfig,
    defaults: OutputConfigDefaults,
    merged_env: BTreeMap<String, String>,
}

#[derive(Debug, Serialize, Default)]
struct OutputConfigDefaults {
    env: Option<BTreeMap<String, String>>,
    user: Option<String>,
    workdir: Option<PathBuf>,
    entrypoint: Option<Vec<String>>,
    cmd: Option<Vec<String>>,
    stop_signal: Option<Signal>,
}

impl OutputConfig {
    pub async fn analyse(bridge: &mut Bridge, config: BaseOutputConfig) -> Result<Self, Error> {
        if config.image == "scratch" {
            return Ok(Self::scratch(config));
        }

        let source = config.source();

        let (digest, spec) = {
            bridge
                .resolve_image_config(&source, Some("Resolving output image"))
                .await
                .context("Unable to resolve image config")?
        };

        debug!("resolved output image config: {:#?}", spec.config);

        let spec = {
            spec.config
                .ok_or_else(|| format_err!("Missing source image config"))?
        };

        let merged_env = merge_spec_and_overriden_env(&spec.env, &config.env);

        let source = if !digest.is_empty() {
            source.with_digest(digest)
        } else {
            source
        };

        Ok(Self {
            source: Some(source),
            overrides: config,
            defaults: spec.into(),
            merged_env,
        })
    }

    fn scratch(config: BaseOutputConfig) -> Self {
        Self {
            source: None,
            merged_env: config.env.clone().unwrap_or_default(),
            overrides: config,
            defaults: Default::default(),
        }
    }

    #[cfg(test)]
    pub fn mocked_new() -> Self {
        Self {
            source: None,

            overrides: Default::default(),
            defaults: Default::default(),
            merged_env: Default::default(),
        }
    }

    pub fn layer_path<P>(&self, path: P) -> LayerPath<P>
    where
        P: AsRef<Path>,
    {
        match self.source {
            Some(ref source) => LayerPath::Other(source.output(), path),
            None => LayerPath::Scratch(path),
        }
    }

    pub fn user(&self) -> Option<&str> {
        self.overrides
            .user
            .as_ref()
            .or_else(|| self.defaults.user.as_ref())
            .map(String::as_str)
    }

    pub fn env(&self) -> impl Iterator<Item = (&str, &str)> {
        self.merged_env
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
    }

    pub fn pre_install_commands(&self) -> Option<&Vec<CustomCommand>> {
        self.overrides.pre_install_commands.as_ref()
    }

    pub fn post_install_commands(&self) -> Option<&Vec<CustomCommand>> {
        self.overrides.post_install_commands.as_ref()
    }

    pub fn copy_commands(&self) -> Option<&Vec<StaticAssetDefinition>> {
        self.overrides.copy.as_ref()
    }
}

impl BaseImageConfig for OutputConfig {
    fn populate_env<'a>(&self, mut command: Command<'a>) -> Command<'a> {
        if let Some(user) = self.user() {
            command = command.user(user);
        }

        for (name, value) in self.env() {
            command = command.env(name, value);
        }

        command
    }

    fn image_source(&self) -> Option<&ImageSource> {
        self.source.as_ref()
    }
}

impl From<oci::ImageConfig> for OutputConfigDefaults {
    fn from(config: oci::ImageConfig) -> Self {
        Self {
            env: config.env,
            user: config.user,
            entrypoint: config.entrypoint,
            cmd: config.cmd,
            workdir: config.working_dir,
            stop_signal: config.stop_signal,
        }
    }
}

impl<'a> Into<oci::ImageConfig> for &'a OutputConfig {
    fn into(self) -> oci::ImageConfig {
        oci::ImageConfig {
            entrypoint: self
                .overrides
                .entrypoint
                .clone()
                .or_else(|| self.defaults.entrypoint.clone()),

            cmd: self
                .overrides
                .args
                .clone()
                .or_else(|| self.defaults.cmd.clone()),

            user: self
                .overrides
                .user
                .clone()
                .or_else(|| self.defaults.user.clone()),

            working_dir: self
                .overrides
                .workdir
                .clone()
                .or_else(|| self.defaults.workdir.clone()),

            env: Some(self.merged_env.clone()),
            labels: self.overrides.labels.clone(),
            volumes: self.overrides.volumes.clone(),
            exposed_ports: self.overrides.expose.clone(),
            stop_signal: self.overrides.stop_signal.or(self.defaults.stop_signal),
        }
    }
}
