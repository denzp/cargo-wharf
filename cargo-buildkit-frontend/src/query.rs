use std::path::{Path, PathBuf};

use failure::Error;
use log::*;
use petgraph::prelude::*;
use petgraph::visit::{Reversed, Topo, Walker};

use buildkit_frontend::{Bridge, OutputRef};
use buildkit_llb::ops::source::LocalSource;
use buildkit_llb::prelude::*;
use buildkit_proto::pb;

use crate::graph::{BuildGraph, Node, NodeCommand, NodeCommandDetails, NodeKind};
use crate::image::RustDockerImage;
use crate::{CONTEXT_PATH, TARGET_PATH};

pub struct GraphQuery<'a> {
    original_graph: &'a StableGraph<Node, ()>,
    reversed_graph: Reversed<&'a StableGraph<Node, ()>>,

    image: &'a RustDockerImage,
}

impl<'a> GraphQuery<'a> {
    pub fn new(graph: &'a BuildGraph, image: &'a RustDockerImage) -> Self {
        Self {
            original_graph: graph.inner(),
            reversed_graph: Reversed(graph.inner()),

            image,
        }
    }

    pub fn definition(&self) -> pb::Definition {
        let context = {
            Source::local("context")
                .custom_name("Using context")
                .add_exclude_pattern("**/target")
        };

        self.terminal(&context).into_definition()
    }

    pub async fn solve(&self, bridge: &mut Bridge) -> Result<OutputRef, Error> {
        let context = {
            Source::local("context")
                .custom_name("Using context")
                .add_exclude_pattern("**/target")
        };

        bridge.solve(self.terminal(&context)).await
    }

    fn terminal<'b: 'a>(&self, context: &'b LocalSource) -> Terminal<'a> {
        let nodes = self.serialize_all_nodes(context);

        debug!("preparing the final operation");

        let (result, result_output) = {
            self.original_graph
                .node_indices()
                .map(move |index| (index, self.original_graph.node_weight(index).unwrap()))
                .filter(|node| node.1.kind() == NodeKind::Binary)
                .zip(0..)
                .fold(
                    (FileSystem::sequence(), 0),
                    |(output, last_idx), (node, idx)| {
                        let from = LayerPath::Other(
                            nodes[node.0.index()].clone().unwrap(),
                            node.1
                                .get_outputs_iter()
                                .next()
                                .unwrap()
                                .strip_prefix(TARGET_PATH)
                                .unwrap(),
                        );

                        let output = if idx == 0 {
                            output.append(FileSystem::copy().from(from).to(
                                OutputIdx(idx),
                                LayerPath::Scratch(format!("/binary-{}", idx)),
                            ))
                        } else {
                            output.append(FileSystem::copy().from(from).to(
                                OutputIdx(idx),
                                LayerPath::Own(OwnOutputIdx(last_idx), format!("/binary-{}", idx)),
                            ))
                        };

                        (output, idx)
                    },
                )
        };

        Terminal::with(result.ref_counted().output(result_output))
    }

    fn serialize_all_nodes<'b: 'a>(
        &self,
        context: &'b LocalSource,
    ) -> Vec<Option<OperationOutput<'a>>> {
        let mut nodes = vec![None; self.original_graph.capacity().0];
        let mut deps = vec![None; self.original_graph.capacity().0];

        let mut visitor = Topo::new(self.original_graph);

        while let Some(index) = visitor.next(self.original_graph) {
            self.maybe_cache_dependencies(&nodes, &mut deps, index);

            let (raw_node_llb, output) =
                self.serialize_node(context, self.original_graph.node_weight(index).unwrap());

            let node_llb = {
                deps[index.index()]
                    .as_ref()
                    .unwrap()
                    .iter()
                    .fold(raw_node_llb, |output, mount| output.mount(mount.clone()))
                    .ref_counted()
            };

            nodes[index.index()] = Some(node_llb.output(output.0));
        }

        nodes
    }

    fn maybe_cache_dependencies(
        &self,
        nodes: &[Option<OperationOutput<'a>>],
        deps: &mut Vec<Option<Vec<Mount<'a, PathBuf>>>>,
        index: NodeIndex,
    ) {
        if deps[index.index()].is_some() {
            return;
        }

        let local_deps = DfsPostOrder::new(&self.reversed_graph, index)
            .iter(&self.reversed_graph)
            .filter(|dep_index| dep_index.index() != index.index())
            .flat_map(|dep_index| {
                self.original_graph
                    .node_weight(dep_index)
                    .unwrap()
                    .get_outputs_iter()
                    .map(move |path| {
                        Mount::ReadOnlySelector(
                            nodes[dep_index.index()].clone().unwrap(),
                            path.into(),
                            path.strip_prefix(TARGET_PATH).unwrap().into(),
                        )
                    })
            });

        deps[index.index()] = Some(local_deps.collect());
    }

    pub fn serialize_node<'b: 'a>(
        &self,
        context: &'b LocalSource,
        node: &'b Node,
    ) -> (Command<'a>, OutputIdx) {
        match node.command() {
            NodeCommand::Simple(ref details) => {
                self.serialize_command(context, node.get_outputs_iter(), details)
            }

            NodeCommand::WithBuildscript { command, .. } => {
                self.serialize_command(context, node.get_outputs_iter(), command)
            }
        }
    }

    fn serialize_command<'b: 'a>(
        &self,
        context: &'b LocalSource,
        mut outputs: impl Iterator<Item = &'b Path>,
        command: &NodeCommandDetails,
    ) -> (Command<'a>, OutputIdx) {
        let out_path = {
            // TODO: go through all outputs
            FileSystem::mkdir(
                OutputIdx(0),
                LayerPath::Scratch(
                    outputs
                        .next()
                        .unwrap()
                        .strip_prefix(TARGET_PATH)
                        .unwrap()
                        .parent()
                        .unwrap(),
                ),
            )
            .make_parents(true)
            .into_operation()
            .ref_counted()
        };

        // TODO: mount the context only when it's needed.

        let command_llb = {
            self.image
                .populate_env(Command::run(&command.program))
                .cwd(CONTEXT_PATH)
                .args(&command.args)
                .env_iter(&command.env)
                .mount(Mount::ReadOnlyLayer(self.image.source().output(), "/"))
                .mount(Mount::ReadOnlyLayer(context.output(), CONTEXT_PATH))
                .mount(Mount::Scratch(OutputIdx(0), "/tmp"))
                .mount(Mount::Layer(OutputIdx(1), out_path.output(0), TARGET_PATH))
        };

        (command_llb, OutputIdx(1))
    }
}
