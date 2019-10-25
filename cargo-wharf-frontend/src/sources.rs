use std::collections::BTreeMap;
use std::path::PathBuf;

use failure::{bail, Error, ResultExt};
use serde::{Deserialize, Serialize};

use buildkit_frontend::Bridge;
use buildkit_llb::prelude::*;

use crate::config::Config;
use crate::graph::Node;
use crate::plan::RawBuildPlan;
use crate::shared::{tools, CONTEXT, CONTEXT_PATH};

const BUILD_PLAN_LAYER_PATH: &str = "/build-plan";
const BUILD_PLAN_FILE_NAME: &str = "build-plan.json";

const OUTPUT_LAYER_PATH: &str = "/output";
const OUTPUT_FILE_NAME: &str = "sources.json";

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Sources {
    pub sources: BTreeMap<String, SourceKind>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceKind {
    RegistryUrl(String),
    Local,
    GitCheckout {
        repo: String,
        reference: Option<String>,
    },
}

impl Sources {
    pub async fn collect(
        bridge: &mut Bridge,
        config: &Config,
        build_plan: &RawBuildPlan,
    ) -> Result<Self, Error> {
        let builder = config.builder_image();

        let args = vec![
            String::from("--manifest-path"),
            PathBuf::from(CONTEXT_PATH)
                .join(config.manifest_path())
                .to_string_lossy()
                .into(),
            String::from("--build-plan-path"),
            PathBuf::from(BUILD_PLAN_LAYER_PATH)
                .join(BUILD_PLAN_FILE_NAME)
                .to_string_lossy()
                .into(),
            String::from("--output"),
            PathBuf::from(OUTPUT_LAYER_PATH)
                .join(OUTPUT_FILE_NAME)
                .to_string_lossy()
                .into(),
        ];

        let build_plan_layer =
            FileSystem::mkfile(OutputIdx(0), LayerPath::Scratch(BUILD_PLAN_FILE_NAME))
                .data(serde_json::to_vec(build_plan).context("Unable to serialize the build plan")?)
                .into_operation()
                .custom_name("Create a temp build plan");

        let command = {
            builder
                .populate_env(Command::run(tools::SOURCES))
                .args(&args)
                .cwd(CONTEXT_PATH)
                .mount(Mount::Layer(OutputIdx(0), builder.source().output(), "/"))
                .mount(Mount::ReadOnlyLayer(CONTEXT.output(), CONTEXT_PATH))
                .mount(Mount::ReadOnlyLayer(
                    build_plan_layer.output(0),
                    BUILD_PLAN_LAYER_PATH,
                ))
                .mount(Mount::ReadOnlySelector(
                    tools::IMAGE.output(),
                    tools::SOURCES,
                    tools::SOURCES,
                ))
                .mount(Mount::SharedCache(builder.cargo_home().join("git")))
                .mount(Mount::SharedCache(builder.cargo_home().join("registry")))
                .mount(Mount::Scratch(OutputIdx(1), OUTPUT_LAYER_PATH))
                .custom_name("Collecting the sources info")
        };

        let sources_layer = {
            bridge
                .solve(Terminal::with(command.output(1)))
                .await
                .context("Unable to collect sources info")?
        };

        let sources = {
            bridge
                .read_file(&sources_layer, OUTPUT_FILE_NAME, None)
                .await
                .context("Unable to read sources info")?
        };

        Ok(serde_json::from_slice(&sources).context("Unable to parse sources info")?)
    }

    pub fn find_for_node(&self, node: &Node) -> Result<&SourceKind, Error> {
        let id = format!("{}:{}", node.package_name(), node.package_version());

        match self.sources.get(&id) {
            Some(kind) => Ok(kind),
            None => {
                bail!("Unable to find sources location for crate '{}'", id);
            }
        }
    }
}
