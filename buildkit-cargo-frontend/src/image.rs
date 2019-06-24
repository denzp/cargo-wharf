use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use buildkit_llb::frontend::Bridge;
use buildkit_llb::ops::{source::ImageSource, Command};
use failure::Error;

use crate::TARGET_PATH;

pub struct RustDockerImage {
    source: ImageSource,

    cargo_home_env: PathBuf,
    other_env: BTreeMap<String, String>,
}

impl RustDockerImage {
    pub async fn analyse(_bridge: &mut Bridge, source: ImageSource) -> Result<Self, Error> {
        // TODO: evaluate the properties with bridge `resolve_image_config` method

        let mut other_env = BTreeMap::default();

        other_env.insert("PATH".into(), "/usr/local/cargo/bin".into());
        other_env.insert("RUSTUP_HOME".into(), "/usr/local/rustup".into());

        Ok(Self {
            source,
            other_env,
            cargo_home_env: "/usr/local/cargo".into(),
        })
    }

    pub fn cargo_home(&self) -> &Path {
        &self.cargo_home_env
    }

    pub fn source(&self) -> &ImageSource {
        &self.source
    }

    pub fn populate_env<'a>(&self, mut command: Command<'a>) -> Command<'a> {
        command = command
            .env("CARGO_HOME", self.cargo_home().display().to_string())
            .env("CARGO_TARGET_DIR", TARGET_PATH);

        for (name, value) in &self.other_env {
            command = command.env(name, value);
        }

        command
    }
}
