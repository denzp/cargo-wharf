use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use failure::{format_err, Error, ResultExt};
use log::*;
use serde::Serialize;

use buildkit_frontend::Bridge;
use buildkit_llb::ops::source::ImageSource;
use buildkit_llb::prelude::*;

use super::base::OutputConfig;

#[derive(Debug, Serialize)]
pub struct OutputImage {
    #[serde(skip_serializing)]
    source: ImageSource,

    pub env: Option<BTreeMap<String, String>>,
    pub user: Option<String>,
    pub workdir: Option<PathBuf>,
    pub entrypoint: Option<Vec<String>>,
    pub cmd: Option<Vec<String>>,
}

impl OutputImage {
    pub async fn analyse(bridge: &mut Bridge, config: OutputConfig) -> Result<Self, Error> {
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

        let env = match (spec.env, config.env) {
            (Some(mut spec), Some(mut config)) => {
                spec.append(&mut config);
                Some(spec)
            }

            (spec, config) => spec.or(config),
        };

        let (entrypoint, cmd) = match (config.entrypoint, config.args) {
            (None, _) => (spec.entrypoint, spec.cmd),
            (entrypoint, cmd) => (entrypoint, cmd),
        };

        Ok(Self {
            source: source.with_digest(digest),

            user: config.user.or(spec.user),
            workdir: config.workdir.or(spec.working_dir),

            env,
            entrypoint,
            cmd,
        })
    }

    pub fn layer_path<P>(&self, path: P) -> LayerPath<P>
    where
        P: AsRef<Path>,
    {
        // TODO: handle "scratch"
        LayerPath::Other(self.source.output(), path)
    }
}
