use std::iter::once;

use async_trait::*;
use chrono::prelude::*;
use failure::Error;

use petgraph::prelude::*;
use petgraph::visit::Reversed;

use buildkit_frontend::oci::*;
use buildkit_frontend::{Bridge, OutputRef};
use buildkit_proto::pb;

use crate::config::Config;
use crate::frontend::Options;
use crate::graph::{BuildGraph, Node};
use crate::shared::tools;

mod print;
mod profile;
mod serialization;
mod source;
mod terminal;

pub use self::profile::Profile;

use self::print::PrettyPrintQuery;
use self::serialization::SerializationQuery;
use self::source::SourceQuery;
use self::terminal::{BuildOutput, TerminalQuery};

pub trait WharfDatabase {
    fn config(&self) -> &Config;

    fn graph(&self) -> &StableGraph<Node, ()>;

    fn reversed_graph(&self) -> Reversed<&StableGraph<Node, ()>> {
        Reversed(self.graph())
    }
}

pub struct WharfStorage<'a> {
    graph: &'a StableGraph<Node, ()>,
    config: &'a Config,
}

#[async_trait]
pub trait WharfQuery: TerminalQuery {
    fn definition(&self) -> Result<pb::Definition, Error> {
        Ok(self.terminal()?.into_definition())
    }

    async fn solve(&self, bridge: &mut Bridge, options: &Options) -> Result<OutputRef, Error> {
        bridge
            .solve_with_cache(self.terminal()?, options.cache_entries())
            .await
    }

    fn image_spec(&self) -> Result<ImageSpecification, Error> {
        let output = self.config().output();

        let config = match self.config().profile() {
            Profile::ReleaseBinaries | Profile::DebugBinaries => self.config().output().into(),
            Profile::ReleaseTests | Profile::DebugTests => ImageConfig {
                entrypoint: Some(
                    once(tools::TEST_RUNNER.into())
                        .chain(
                            self.outputs()
                                .map(|BuildOutput { path, .. }| path.to_string_lossy().into()),
                        )
                        .collect(),
                ),

                env: Some(
                    output
                        .env()
                        .map(|(name, value)| (name.into(), value.into()))
                        .collect(),
                ),

                cmd: None,
                user: output.user().map(String::from),
                working_dir: None,

                labels: None,
                volumes: None,
                exposed_ports: None,
                stop_signal: None,
            },
        };

        Ok(ImageSpecification {
            created: Some(Utc::now()),
            author: None,

            // TODO: don't hardcode this
            architecture: Architecture::Amd64,
            os: OperatingSystem::Linux,

            config: Some(config),
            rootfs: None,
            history: None,
        })
    }
}

impl<'a> WharfDatabase for WharfStorage<'a> {
    fn config(&self) -> &Config {
        self.config
    }

    fn graph(&self) -> &StableGraph<Node, ()> {
        self.graph
    }
}

impl<'a> WharfQuery for WharfStorage<'a> {}
impl<'a> TerminalQuery for WharfStorage<'a> {}
impl<'a> SerializationQuery for WharfStorage<'a> {}
impl<'a> SourceQuery for WharfStorage<'a> {}
impl<'a> PrettyPrintQuery for WharfStorage<'a> {}

impl<'a> WharfStorage<'a> {
    pub fn new(graph: &'a BuildGraph, config: &'a Config) -> Self {
        Self {
            graph: graph.inner(),
            config,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::from_slice;

    use buildkit_llb::prelude::*;

    use super::*;
    use crate::config::{BinaryDefinition, BuilderConfig, OutputConfig};
    use crate::plan::RawBuildPlan;

    pub struct MockStorage {
        config: Config,
        graph: StableGraph<Node, ()>,
    }

    impl MockStorage {
        pub fn mocked(profile: Profile) -> Self {
            let graph = BuildGraph::from(
                from_slice::<RawBuildPlan>(include_bytes!("../../tests/build-plan.json")).unwrap(),
            );

            let builder = BuilderConfig::mocked_new(Source::image("rust"), "/root/.cargo".into());
            let output = OutputConfig::mocked_new();

            let binaries = vec![
                BinaryDefinition {
                    name: "bin-1".into(),
                    destination: "/usr/bin/mock-binary-1".into(),
                },
                BinaryDefinition {
                    name: "bin-3".into(),
                    destination: "/bin/binary-3".into(),
                },
            ];

            let config = Config::mocked_new(builder, output, profile, binaries);

            Self {
                graph: graph.into_inner(),
                config,
            }
        }
    }

    impl WharfDatabase for MockStorage {
        fn config(&self) -> &Config {
            &self.config
        }

        fn graph(&self) -> &StableGraph<Node, ()> {
            &self.graph
        }
    }

    impl WharfQuery for MockStorage {}
    impl TerminalQuery for MockStorage {}
    impl SerializationQuery for MockStorage {}
    impl SourceQuery for MockStorage {}
    impl PrettyPrintQuery for MockStorage {}
}
