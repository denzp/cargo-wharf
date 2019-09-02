use std::path::Path;

use failure::{Error, ResultExt};
use futures::prelude::*;
use prost::Message;

use buildkit_frontend::{Bridge, Frontend, FrontendOutput, Options};
use buildkit_llb::ops::fs::SequenceOperation;
use buildkit_llb::prelude::*;
use buildkit_proto::pb;

use crate::config::Config;
use crate::graph::BuildGraph;
use crate::plan::RawBuildPlan;
use crate::query::GraphQuery;

pub struct CargoFrontend;

impl Frontend for CargoFrontend {
    type RunFuture = impl Future<Output = Result<FrontendOutput, Error>>;

    fn run(self, mut bridge: Bridge, options: Options) -> Self::RunFuture {
        async move {
            let mut debug_op = FileSystem::sequence().custom_name("Writing the debug output");

            let config = {
                Config::analyse(&mut bridge)
                    .await
                    .context("Unable to analyse config")?
            };

            if options.has_value("debug", "config") {
                debug_op = append_debug_output(debug_op, "config.json", &config)?;
            }

            let build_plan = {
                RawBuildPlan::evaluate(&mut bridge, &config)
                    .await
                    .context("Unable to evaluate the Cargo build plan")?
            };

            if options.has_value("debug", "build-plan") {
                debug_op = append_debug_output(debug_op, "build-plan.json", &build_plan)?;
            }

            let graph: BuildGraph = build_plan.into();
            let query = GraphQuery::new(&graph, &config);

            if options.has_value("debug", "build-graph") {
                debug_op = append_debug_output(debug_op, "build-graph.json", &graph)?;
            }

            if options.has_value("debug", "llb") {
                debug_op = append_debug_output(debug_op, "llb.pb", &query.definition())?;
            }

            if options.has("debug") {
                return Ok(FrontendOutput::with_ref(
                    bridge
                        .solve(Terminal::with(debug_op.last_output().unwrap()))
                        .await
                        .context("Unable to write debug output")?,
                ));
            }

            Ok(FrontendOutput::with_spec_and_ref(
                query.image_spec().context("Unable to build image spec")?,
                query
                    .solve(&mut bridge)
                    .await
                    .context("Unable to build the crate")?,
            ))
        }
    }
}

trait DebugOutput {
    fn as_bytes(&self) -> Result<Vec<u8>, Error>;
}

fn append_debug_output<'a, P, O>(
    op: SequenceOperation<'a>,
    path: P,
    output: &O,
) -> Result<SequenceOperation<'a>, Error>
where
    P: AsRef<Path>,
    O: DebugOutput,
{
    let (index, layer_path) = match op.last_output_index() {
        Some(index) => (index + 1, LayerPath::Own(OwnOutputIdx(index), path)),
        None => (0, LayerPath::Scratch(path)),
    };

    Ok(op.append(FileSystem::mkfile(OutputIdx(index), layer_path).data(output.as_bytes()?)))
}

impl DebugOutput for Config {
    fn as_bytes(&self) -> Result<Vec<u8>, Error> {
        Ok(serde_json::to_vec_pretty(self)?)
    }
}

impl DebugOutput for RawBuildPlan {
    fn as_bytes(&self) -> Result<Vec<u8>, Error> {
        Ok(serde_json::to_vec_pretty(self)?)
    }
}

impl DebugOutput for BuildGraph {
    fn as_bytes(&self) -> Result<Vec<u8>, Error> {
        Ok(serde_json::to_vec_pretty(self)?)
    }
}

impl DebugOutput for pb::Definition {
    fn as_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut bytes = Vec::new();
        self.encode(&mut bytes)?;

        Ok(bytes)
    }
}
