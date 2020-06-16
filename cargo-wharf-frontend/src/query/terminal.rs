use std::iter::empty;
use std::path::{Path, PathBuf};

use either::Either;
use failure::{bail, Error};
use log::*;
use petgraph::prelude::*;

use buildkit_llb::prelude::*;

use crate::config::BaseImageConfig;
use crate::graph::{Node, NodeKind, PrimitiveNodeKind};
use crate::shared::{CONTEXT, tools, TARGET_PATH};

use super::print::{PrettyPrintQuery, PrintKind};
use super::{Profile, SerializationQuery, WharfDatabase};

pub struct OutputMapping<'a> {
    from: LayerPath<'a, PathBuf>,
    to: PathBuf,
}

type NodesCache<'a> = Vec<Option<OperationOutput<'a>>>;

pub struct BuildOutput<'a> {
    pub index: NodeIndex,
    pub node: &'a Node,
    pub path: PathBuf,
}

pub trait TerminalQuery: WharfDatabase + SerializationQuery + PrettyPrintQuery {
    fn terminal(&self) -> Result<Terminal<'_>, Error> {
        debug!("serializing all nodes");
        let nodes = self.serialize_all_nodes();
        let outputs = self.mapped_outputs(nodes);

        if outputs.is_empty() {
            bail!("Nothing to do - no binaries were found");
        }

        debug!("preparing the final operation");

        let operation = FileSystem::sequence().custom_name("Composing the output image");
        let operation = {
            outputs.into_iter().fold(operation, |output, mapping| {
                let (index, layer_path) = match output.last_output_index() {
                    Some(index) => (index + 1, LayerPath::Own(OwnOutputIdx(index), mapping.to)),
                    None => (0, self.output_layer_path(mapping.to)),
                };

                output.append(
                    FileSystem::copy()
                        .from(mapping.from)
                        .to(OutputIdx(index), layer_path)
                        .create_path(true),
                )
            })
        };
        let operation = if let Some(copy_commands) = self.config().output().copy_commands() {
            copy_commands.into_iter().fold(operation, |output, asset| {
                let (index, layer_path) = match output.last_output_index() {
                    Some(index) => (index + 1, LayerPath::Own(OwnOutputIdx(index), asset.dst.as_path())),
                    None => (0, self.output_layer_path(asset.dst.as_path())),
                };
                output.append(
                    FileSystem::copy()
                        .from(LayerPath::Other(CONTEXT.output(), asset.src.as_path()))
                        .to(OutputIdx(index), layer_path)
                )
            })
        } else {
            operation
        };

        let mut commands_iter = {
            self.config()
                .output()
                .post_install_commands()
                .map(|commands| Either::Left(commands.iter().map(From::from)))
                .unwrap_or_else(|| Either::Right(empty()))
        };

        if let Some((name, args, display)) = commands_iter.next() {
            let mut output = {
                self.config()
                    .output()
                    .populate_env(Command::run(name))
                    .args(args.iter())
                    .mount(Mount::Layer(
                        OutputIdx(0),
                        operation.ref_counted().last_output().unwrap(),
                        "/",
                    ))
                    .custom_name(self.pretty_print(PrintKind::CustomCommand(display)))
                    .ref_counted()
                    .output(0)
            };

            for (name, args, display) in commands_iter {
                output = {
                    self.config()
                        .output()
                        .populate_env(Command::run(name))
                        .args(args.iter())
                        .mount(Mount::Layer(OutputIdx(0), output, "/"))
                        .custom_name(self.pretty_print(PrintKind::CustomCommand(display)))
                        .ref_counted()
                        .output(0)
                };
            }

            return Ok(Terminal::with(output));
        }

        Ok(Terminal::with(
            operation.ref_counted().last_output().unwrap(),
        ))
    }

    fn output_layer_path<P>(&self, path: P) -> LayerPath<'_, P>
    where
        P: AsRef<Path>,
    {
        match self.output_source() {
            Some(ref output) => LayerPath::Other(output.clone(), path),
            None => LayerPath::Scratch(path),
        }
    }

    fn outputs(&self) -> Box<dyn Iterator<Item = BuildOutput<'_>> + '_> {
        Box::new(match self.config().profile() {
            Profile::ReleaseBinaries | Profile::DebugBinaries => Either::Left(
                self.graph()
                    .node_indices()
                    .map(move |index| (index, self.graph().node_weight(index).unwrap()))
                    .filter_map(move |(index, node)| {
                        match self.config().find_binary(node.binary_name()?) {
                            Some(found) => {
                                Some(BuildOutput::new(index, node, found.destination.clone()))
                            }

                            None => None,
                        }
                    }),
            ),

            Profile::ReleaseTests | Profile::DebugTests => Either::Right(
                self.graph()
                    .node_indices()
                    .map(move |index| (index, self.graph().node_weight(index).unwrap()))
                    .filter(|(_, node)| match node.kind() {
                        NodeKind::Primitive(PrimitiveNodeKind::Test) => true,
                        NodeKind::BuildScriptOutputConsumer(PrimitiveNodeKind::Test, _) => true,

                        _ => false,
                    })
                    .map(|(index, node)| {
                        let to: PathBuf = {
                            node.outputs_iter()
                                .next()
                                .unwrap()
                                .strip_prefix(TARGET_PATH)
                                .unwrap()
                                .into()
                        };

                        BuildOutput::new(index, node, PathBuf::from("/test").join(to))
                    }),
            ),
        })
    }

    fn mapped_outputs<'a>(&self, nodes: NodesCache<'a>) -> Vec<OutputMapping<'a>> {
        let profile = self.config().profile();
        let mut mapped_outputs: Vec<_> = {
            self.outputs()
                .map(move |BuildOutput { index, node, path }| {
                    let from = LayerPath::Other(
                        nodes[index.index()].clone().unwrap(),
                        node.outputs_iter()
                            .next()
                            .unwrap()
                            .strip_prefix(TARGET_PATH)
                            .unwrap()
                            .into(),
                    );

                    OutputMapping { from, to: path }
                })
                .collect()
        };

        if profile == Profile::ReleaseTests || profile == Profile::DebugTests {
            mapped_outputs.push(OutputMapping {
                from: LayerPath::Other(tools::IMAGE.output(), tools::TEST_RUNNER.into()),
                to: tools::TEST_RUNNER.into(),
            });
        }

        mapped_outputs
    }
}

impl<'a> BuildOutput<'a> {
    pub fn new(index: NodeIndex, node: &'a Node, path: PathBuf) -> Self {
        Self { index, node, path }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::MockStorage;

    #[test]
    fn query_binaries() {
        let storage = MockStorage::mocked(Profile::ReleaseBinaries);

        assert_eq!(
            storage
                .outputs()
                .map(|BuildOutput { index, path, .. }| (index, path))
                .collect::<Vec<_>>(),
            vec![(NodeIndex::new(15), "/usr/bin/mock-binary-1".into())]
        );
    }

    #[test]
    fn query_tests() {
        let storage = MockStorage::mocked(Profile::ReleaseTests);

        assert_eq!(
            storage
                .outputs()
                .map(|BuildOutput { index, path, .. }| (index, path))
                .collect::<Vec<_>>(),
            vec![
                (
                    NodeIndex::new(16),
                    "/test/x86_64-unknown-linux-musl/debug/deps/bin_1-5b5e8a9adfa6ccf4".into()
                ),
                (
                    NodeIndex::new(18),
                    "/test/x86_64-unknown-linux-musl/debug/deps/bin_2-92b8326325c2f547".into()
                ),
            ]
        );
    }
}
