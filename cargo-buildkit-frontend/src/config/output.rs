use std::path::Path;

use serde::Serialize;

use buildkit_llb::ops::source::ImageSource;
use buildkit_llb::prelude::*;

use super::base::OutputConfig;

#[derive(Debug, Serialize)]
pub struct OutputImage {
    #[serde(skip_serializing)]
    source: ImageSource,
}

impl OutputImage {
    pub fn new(config: OutputConfig) -> Self {
        Self {
            source: config.source(),
        }
    }

    pub fn layer_path<P>(&self, path: P) -> LayerPath<P>
    where
        P: AsRef<Path>,
    {
        // TODO: handle "scratch"
        LayerPath::Other(self.source.output(), path)
    }
}
