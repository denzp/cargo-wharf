use std::path::Path;

use failure::{Error, ResultExt};
use futures::prelude::*;
use prost::Message;

use buildkit_frontend::{Bridge, Frontend, OutputRef};
use buildkit_llb::prelude::*;
use buildkit_proto::pb;

use crate::graph::BuildGraph;
use crate::image::RustDockerImage;
use crate::plan::RawBuildPlan;
use crate::query::GraphQuery;

pub struct CargoFrontend;

impl Frontend for CargoFrontend {
    existential type RunFuture: Future<Output = Result<OutputRef, Error>>;

    fn run(self, mut bridge: Bridge, options: Vec<String>) -> Self::RunFuture {
        async move {
            let builder_image = {
                RustDockerImage::analyse(&mut bridge, Source::image("rustlang/rust:nightly"))
                    .await
                    .context("Unable to analyse Rust builder image")?
            };

            let build_plan = {
                RawBuildPlan::evaluate(&mut bridge, &builder_image)
                    .await
                    .context("Unable to evaluate the Cargo build plan")?
            };

            if options.iter().any(|x| x == "debug=build-plan") {
                return self
                    .debug_output(&mut bridge, "/build-plan.json", build_plan)
                    .await;
            }

            let graph: BuildGraph = build_plan.into();
            let query = GraphQuery::new(&graph, &builder_image);

            if options.iter().any(|x| x == "debug=llb") {
                return self
                    .debug_output(&mut bridge, "/llb.pb", query.into_definition())
                    .await;
            }

            query
                .solve(&mut bridge)
                .await
                .context("Unable to build the crate")
                .map_err(Error::from)
        }
    }
}

trait DebugOutput {
    fn as_bytes(&self) -> Result<Vec<u8>, Error>;
}

impl CargoFrontend {
    async fn debug_output<P, O>(
        &self,
        bridge: &mut Bridge,
        path: P,
        output: O,
    ) -> Result<OutputRef, Error>
    where
        P: AsRef<Path>,
        O: DebugOutput,
    {
        let action = FileSystem::mkfile(OutputIdx(0), LayerPath::Scratch(path))
            .data(output.as_bytes()?)
            .into_operation()
            .custom_name("Writing the debug output");

        bridge
            .solve(Terminal::with(action.output(0)))
            .await
            .context("Unable to write output")
            .map_err(Error::from)
    }
}

impl DebugOutput for RawBuildPlan {
    fn as_bytes(&self) -> Result<Vec<u8>, Error> {
        Ok(serde_json::to_string_pretty(self)?.into_bytes())
    }
}

impl DebugOutput for pb::Definition {
    fn as_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut bytes = Vec::new();
        self.encode(&mut bytes)?;

        Ok(bytes)
    }
}
