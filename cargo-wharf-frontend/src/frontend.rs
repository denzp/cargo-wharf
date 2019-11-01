use std::path::PathBuf;

use async_trait::async_trait;
use failure::{Error, ResultExt};
use serde::Deserialize;

use buildkit_frontend::options::common::CacheOptionsEntry;
use buildkit_frontend::{Bridge, Frontend, FrontendOutput};

use crate::config::Config;
use crate::debug::{DebugKind, DebugOperation};
use crate::graph::BuildGraph;
use crate::plan::RawBuildPlan;
use crate::query::{GraphQuery, Profile};

pub struct CargoFrontend;

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Options {
    /// Path to the `Dockerfile` - in our case it's a path to `Cargo.toml`.
    pub filename: Option<PathBuf>,

    /// Overriden crate manifest path.
    pub manifest_path: Option<PathBuf>,

    pub features: Vec<String>,
    pub no_default_features: bool,
    pub profile: Profile,

    /// Debugging features of the frontend.
    pub debug: Vec<DebugKind>,

    /// New approach to specify cache imports.
    pub cache_imports: Vec<CacheOptionsEntry>,

    /// Legacy convention to specify cache imports.
    #[serde(deserialize_with = "CacheOptionsEntry::from_legacy_list")]
    pub cache_from: Vec<CacheOptionsEntry>,
}

#[async_trait]
impl Frontend<Options> for CargoFrontend {
    async fn run(self, mut bridge: Bridge, options: Options) -> Result<FrontendOutput, Error> {
        let mut debug = DebugOperation::new();

        let config = {
            Config::analyse(&mut bridge, &options)
                .await
                .context("Unable to analyse config")?
        };

        debug.maybe(&options, || &config);

        let plan = {
            RawBuildPlan::evaluate(&mut bridge, &config)
                .await
                .context("Unable to evaluate the Cargo build plan")?
        };

        debug.maybe(&options, || &plan);

        let graph: BuildGraph = plan.into();
        let query = GraphQuery::new(&graph, &config);

        debug.maybe(&options, || &graph);
        debug.maybe(&options, || query.definition().unwrap());

        if !options.debug.is_empty() {
            return Ok(FrontendOutput::with_ref(
                bridge
                    .solve(debug.terminal())
                    .await
                    .context("Unable to write debug output")?,
            ));
        }

        Ok(FrontendOutput::with_spec_and_ref(
            query.image_spec().context("Unable to build image spec")?,
            query
                .solve(&mut bridge, &options)
                .await
                .context("Unable to build the crate")?,
        ))
    }
}

impl Options {
    pub fn cache_entries(&self) -> &[CacheOptionsEntry] {
        if !self.cache_imports.is_empty() {
            return &self.cache_imports;
        }

        &self.cache_from
    }
}
