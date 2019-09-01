use std::collections::BTreeMap;
use std::path::PathBuf;

use failure::{Error, ResultExt};
use semver::Version;
use serde::{Deserialize, Serialize};

use buildkit_frontend::Bridge;
use buildkit_llb::prelude::*;

use crate::image::{RustDockerImage, TOOLS_IMAGE};
use crate::CONTEXT_PATH;

const BUILD_PLAN_EXEC: &str = "/usr/local/bin/cargo-build-plan";
const OUTPUT_LAYER_PATH: &str = "/output";
const OUTPUT_NAME: &str = "build-plan.json";

#[derive(Debug, Deserialize, Serialize)]
pub struct RawInvocation {
    pub package_name: String,
    pub package_version: Version,
    pub target_kind: Vec<RawTargetKind>,
    pub deps: Vec<usize>,
    pub outputs: Vec<PathBuf>,
    pub links: BTreeMap<PathBuf, PathBuf>,
    pub program: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub cwd: PathBuf, // TODO(denzp): should this really be an "Option<PathBuf>" like in Cargo?
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RawTargetKind {
    Lib,
    Bin,
    Test,
    CustomBuild,
    ProcMacro,
    Example,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RawBuildPlan {
    pub invocations: Vec<RawInvocation>,
}

impl RawBuildPlan {
    pub async fn evaluate<'a, 'b: 'a>(
        bridge: &'a mut Bridge,
        image: &'b RustDockerImage,
    ) -> Result<Self, Error> {
        let context = {
            Source::local("context")
                .custom_name("Using context")
                .add_exclude_pattern("**/target")
        };

        let command = {
            image
                .populate_env(Command::run(BUILD_PLAN_EXEC))
                .args(&[
                    "--manifest-path",
                    &PathBuf::from(CONTEXT_PATH)
                        .join("Cargo.toml")
                        .to_string_lossy(),
                ])
                .args(&[
                    "--output",
                    &PathBuf::from(OUTPUT_LAYER_PATH)
                        .join(OUTPUT_NAME)
                        .to_string_lossy(),
                ])
                .cwd(CONTEXT_PATH)
                .mount(Mount::Layer(OutputIdx(0), image.source().output(), "/"))
                .mount(Mount::ReadOnlyLayer(context.output(), CONTEXT_PATH))
                .mount(Mount::ReadOnlySelector(
                    TOOLS_IMAGE.output(),
                    BUILD_PLAN_EXEC,
                    BUILD_PLAN_EXEC,
                ))
                .mount(Mount::Scratch(OutputIdx(1), OUTPUT_LAYER_PATH))
                .custom_name("Evaluating the build plan")
        };

        let build_plan_layer = {
            bridge
                .solve(Terminal::with(command.output(1)))
                .await
                .context("Unable to evaluate the build plan")?
        };

        let build_plan = {
            bridge
                .read_file(&build_plan_layer, OUTPUT_NAME, None)
                .await
                .context("Unable to read Cargo build plan")?
        };

        Ok(serde_json::from_slice(&build_plan).context("Unable to parse Cargo build plan")?)
    }
}
