use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use failure::{format_err, Error, ResultExt};
use log::*;
use serde::Serialize;

use buildkit_frontend::Bridge;
use buildkit_llb::ops::source::ImageSource;
use buildkit_llb::prelude::*;

use super::base::BuilderConfig;
use crate::shared::TARGET_PATH;

#[derive(Debug, Serialize)]
pub struct BuilderImage {
    #[serde(skip_serializing)]
    source: ImageSource,
    cargo_home: PathBuf,

    env: BTreeMap<String, String>,
    user: Option<String>,
    target: Option<String>,
}

impl BuilderImage {
    pub async fn analyse(bridge: &mut Bridge, config: BuilderConfig) -> Result<Self, Error> {
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

        let env = match (spec.env, config.env) {
            (Some(mut spec), Some(mut config)) => {
                spec.append(&mut config);
                spec
            }

            (spec, config) => spec.or(config).unwrap_or_default(),
        };

        let user = config.user.or(spec.user);
        let cargo_home = PathBuf::from(
            env.get("CARGO_HOME")
                .cloned()
                .or_else(|| guess_cargo_home(&user))
                .ok_or_else(|| format_err!("Unable to find or guess CARGO_HOME env variable"))?,
        );

        Ok(Self {
            source: source.with_digest(digest),
            cargo_home,

            env,
            user,

            target: config.target,
        })
    }

    #[cfg(test)]
    pub fn new(source: ImageSource, cargo_home: PathBuf) -> Self {
        BuilderImage {
            source,
            cargo_home,

            env: Default::default(),
            user: Default::default(),
            target: Default::default(),
        }
    }

    pub fn cargo_home(&self) -> &Path {
        &self.cargo_home
    }

    pub fn source(&self) -> &ImageSource {
        &self.source
    }

    pub fn target(&self) -> Option<&str> {
        self.target.as_ref().map(String::as_str)
    }

    pub fn populate_env<'a>(&self, mut command: Command<'a>) -> Command<'a> {
        command = command.env("CARGO_TARGET_DIR", TARGET_PATH);

        if let Some(ref user) = self.user {
            command = command.user(user);
        }

        for (name, value) in &self.env {
            command = command.env(name, value);
        }

        command
            .env("CARGO_HOME", self.cargo_home().display().to_string())
            .mount(Mount::SharedCache(self.cargo_home().join("git")))
            .mount(Mount::SharedCache(self.cargo_home().join("registry")))
    }
}

fn guess_cargo_home(user: &Option<String>) -> Option<String> {
    match user.as_ref().map(String::as_str) {
        Some("root") => Some("/root/.cargo".into()),
        Some(user) => Some(format!("/home/{}/.cargo", user)),
        None => None,
    }
}

#[test]
fn cargo_home_guessing() {
    assert_eq!(guess_cargo_home(&None), None);
    assert_eq!(
        guess_cargo_home(&Some("root".into())),
        Some("/root/.cargo".into())
    );
    assert_eq!(
        guess_cargo_home(&Some("den".into())),
        Some("/home/den/.cargo".into())
    );
}
