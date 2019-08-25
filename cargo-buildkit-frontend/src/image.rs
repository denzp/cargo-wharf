use std::collections::HashMap;
use std::path::{Path, PathBuf};

use failure::{format_err, Error, ResultExt};
use lazy_static::*;
use log::*;

use buildkit_frontend::Bridge;
use buildkit_llb::ops::source::ImageSource;
use buildkit_llb::prelude::*;

use crate::TARGET_PATH;

pub const BUILDSCRIPT_CAPTURE_EXEC: &str = "/usr/local/bin/cargo-buildscript-capture";
pub const BUILDSCRIPT_APPLY_EXEC: &str = "/usr/local/bin/cargo-buildscript-apply";

lazy_static! {
    pub static ref TOOLS_IMAGE: ImageSource = Source::image("denzp/cargo-container-tools:local");
}

#[derive(Debug)]
pub struct RustDockerImage {
    source: ImageSource,
    source_env: HashMap<String, String>,
    source_user: Option<String>,

    cargo_home_env: PathBuf,
}

impl RustDockerImage {
    pub async fn analyse(bridge: &mut Bridge, source: ImageSource) -> Result<Self, Error> {
        let (digest, spec) = {
            bridge
                .resolve_image_config(&source)
                .await
                .context("Unable to resolve image config")?
        };

        debug!("resolved builder image config: {:#?}", spec.config);

        let config = {
            spec.config
                .ok_or_else(|| format_err!("Missing source image config"))?
        };

        let source_env = config.env.unwrap_or_default();

        let cargo_home_env = {
            PathBuf::from(
                source_env
                    .get("CARGO_HOME")
                    .ok_or_else(|| format_err!("Unable to find CARGO_HOME env variable"))?,
            )
        };

        Ok(Self {
            source: source.with_digest(digest),
            source_env,
            source_user: config.user,
            cargo_home_env,
        })
    }

    pub fn cargo_home(&self) -> &Path {
        &self.cargo_home_env
    }

    pub fn source(&self) -> &ImageSource {
        &self.source
    }

    pub fn populate_env<'a>(&self, mut command: Command<'a>) -> Command<'a> {
        command = command.env("CARGO_TARGET_DIR", TARGET_PATH);

        if let Some(ref user) = self.source_user {
            command = command.user(user);
        }

        for (name, value) in &self.source_env {
            command = command.env(name, value);
        }

        command
            .env("CARGO_HOME", self.cargo_home().display().to_string())
            .mount(Mount::SharedCache(self.cargo_home().join("git")))
            .mount(Mount::SharedCache(self.cargo_home().join("registry")))
    }
}
