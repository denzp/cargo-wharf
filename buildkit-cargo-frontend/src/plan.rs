use std::collections::BTreeMap;
use std::path::PathBuf;

use buildkit_llb::frontend::Bridge;
use buildkit_llb::prelude::*;
use failure::{Error, ResultExt};
use semver::Version;
use serde::Deserialize;

use crate::image::RustDockerImage;
use crate::CONTEXT_PATH;

const PLAN_EVALUATION_COMMAND: &str = "cargo build -Z unstable-options --build-plan --all-targets";
const PLAN_OUTPUT_LAYER_PATH: &str = "/output";
const PLAN_OUTPUT_NAME: &str = "/build-plan.json";

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum RawTargetKind {
    Lib,
    Bin,
    Test,
    CustomBuild,
    ProcMacro,
    Example,
}

#[derive(Debug, Deserialize)]
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

        let args = &[
            "-c",
            &format!(
                "{} > {}/{}",
                PLAN_EVALUATION_COMMAND, PLAN_OUTPUT_LAYER_PATH, PLAN_OUTPUT_NAME
            ),
        ];

        let command = {
            image
                .populate_env(Command::run("/bin/sh").args(args))
                .cwd(CONTEXT_PATH)
                .mount(Mount::Layer(OutputIdx(0), image.source().output(), "/"))
                .mount(Mount::ReadOnlyLayer(context.output(), CONTEXT_PATH))
                .mount(Mount::Scratch(OutputIdx(1), PLAN_OUTPUT_LAYER_PATH))
                .mount(Mount::SharedCache(image.cargo_home().join("git")))
                .mount(Mount::SharedCache(image.cargo_home().join("registry")))
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
                .read_file(&build_plan_layer, "/build-plan.json", None)
                .await
                .context("Unable to read Cargo build plan")?
        };

        Ok(serde_json::from_slice(&build_plan).context("Unable to parse Cargo build plan")?)
    }
}
