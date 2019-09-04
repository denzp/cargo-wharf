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
                .resolve_image_config(&source, Some("resolving output image"))
                .await
                .context("Unable to resolve image config")?
        };

        debug!("resolved output image config: {:#?}", spec.config);

        let config = {
            spec.config
                .ok_or_else(|| format_err!("Missing source image config"))?
        };

        Ok(Self {
            source: source.with_digest(digest),

            env: config.env,
            user: config.user,
            workdir: config.working_dir,
            entrypoint: config.entrypoint,
            cmd: config.cmd,
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
