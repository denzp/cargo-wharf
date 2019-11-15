use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use failure::{format_err, Error, ResultExt};
use log::*;
use serde::Serialize;

use buildkit_frontend::oci;
use buildkit_frontend::Bridge;
use buildkit_llb::ops::source::ImageSource;
use buildkit_llb::prelude::*;

use super::base::BaseBuilderConfig;
use crate::shared::TARGET_PATH;

#[derive(Debug, Serialize)]
pub struct BuilderConfig {
    #[serde(skip_serializing)]
    source: ImageSource,

    overrides: BaseBuilderConfig,
    defaults: BuilderConfigDefaults,

    merged_env: BTreeMap<String, String>,
    cargo_home: PathBuf,
}

#[derive(Debug, Serialize, Default)]
struct BuilderConfigDefaults {
    env: Option<BTreeMap<String, String>>,
    user: Option<String>,
}

impl BuilderConfig {
    pub async fn analyse(bridge: &mut Bridge, config: BaseBuilderConfig) -> Result<Self, Error> {
        let source = config.source();

        let (digest, spec) = {
            bridge
                .resolve_image_config(&source, Some("Resolving builder image"))
                .await
                .context("Unable to resolve image config")?
        };

        debug!("resolved builder image config: {:#?}", spec.config);

        let spec = {
            spec.config
                .ok_or_else(|| format_err!("Missing source image config"))?
        };

        let source = if !digest.is_empty() {
            source.with_digest(digest)
        } else {
            source
        };

        let merged_env = super::merge_spec_and_overriden_env(&spec.env, &config.env);
        let user = {
            config
                .user
                .as_ref()
                .or_else(|| spec.user.as_ref())
                .map(String::as_str)
        };

        let cargo_home = PathBuf::from(
            merged_env
                .get("CARGO_HOME")
                .cloned()
                .or_else(|| guess_cargo_home(user))
                .ok_or_else(|| format_err!("Unable to find or guess CARGO_HOME env variable"))?,
        );

        Ok(Self {
            source,
            overrides: config,
            defaults: spec.into(),

            cargo_home,
            merged_env,
        })
    }

    #[cfg(test)]
    pub fn mocked_new(source: ImageSource, cargo_home: PathBuf) -> Self {
        BuilderConfig {
            source,

            defaults: Default::default(),
            overrides: Default::default(),

            cargo_home,
            merged_env: Default::default(),
        }
    }

    pub fn cargo_home(&self) -> &Path {
        &self.cargo_home
    }

    pub fn source(&self) -> &ImageSource {
        &self.source
    }

    pub fn target(&self) -> Option<&str> {
        self.overrides.target.as_ref().map(String::as_str)
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

    pub fn populate_env<'a>(&self, mut command: Command<'a>) -> Command<'a> {
        command = command.env("CARGO_TARGET_DIR", TARGET_PATH);

        if let Some(user) = self.user() {
            command = command.user(user);
        }

        for (name, value) in self.env() {
            command = command.env(name, value);
        }

        command
            .env("CARGO_HOME", self.cargo_home().display().to_string())
            .mount(Mount::SharedCache(self.cargo_home().join("git")))
            .mount(Mount::SharedCache(self.cargo_home().join("registry")))
    }
}

fn guess_cargo_home(user: Option<&str>) -> Option<String> {
    match user {
        Some("root") => Some("/root/.cargo".into()),
        Some(user) => Some(format!("/home/{}/.cargo", user)),
        None => None,
    }
}

impl From<oci::ImageConfig> for BuilderConfigDefaults {
    fn from(config: oci::ImageConfig) -> Self {
        Self {
            env: config.env,
            user: config.user,
        }
    }
}

#[test]
fn cargo_home_guessing() {
    assert_eq!(guess_cargo_home(None), None);
    assert_eq!(guess_cargo_home(Some("root")), Some("/root/.cargo".into()));
    assert_eq!(
        guess_cargo_home(Some("den")),
        Some("/home/den/.cargo".into())
    );
}
