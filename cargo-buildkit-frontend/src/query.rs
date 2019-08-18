use std::path::{Path, PathBuf};

use chrono::prelude::*;
use failure::Error;
use lazy_static::*;
use log::*;
use petgraph::prelude::*;
use petgraph::visit::{Reversed, Topo, Walker};

use buildkit_frontend::oci::*;
use buildkit_frontend::{Bridge, OutputRef};
use buildkit_llb::ops::source::LocalSource;
use buildkit_llb::prelude::*;
use buildkit_proto::pb;

use crate::graph::{BuildGraph, Node, NodeCommand, NodeCommandDetails, NodeKind};
use crate::image::{
    RustDockerImage, BUILDSCRIPT_APPLY_EXEC, BUILDSCRIPT_CAPTURE_EXEC, TOOLS_IMAGE,
};
use crate::{CONTEXT_PATH, TARGET_PATH};

lazy_static! {
    static ref CONTEXT: LocalSource = {
        Source::local("context")
            .custom_name("Using build context")
            .add_exclude_pattern("**/target")
    };
}

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
        self.terminal().into_definition()
    }

    pub async fn solve(&self, bridge: &mut Bridge) -> Result<OutputRef, Error> {
        bridge.solve(self.terminal()).await
    }

    pub fn image_spec(&self) -> Result<ImageSpecification, Error> {
        Ok(ImageSpecification {
            created: Some(Utc::now()),
            author: None,

            architecture: Architecture::Amd64,
            os: OperatingSystem::Linux,

            config: None,
            rootfs: None,
            history: None,
        })
    }

    fn terminal(&self) -> Terminal<'a> {
        debug!("serializing all nodes");
        let nodes = self.serialize_all_nodes();

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
                                .outputs_iter()
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

    fn serialize_all_nodes(&self) -> Vec<Option<OperationOutput<'a>>> {
        let mut nodes = vec![None; self.original_graph.capacity().0];
        let mut deps = vec![None; self.original_graph.capacity().0];

        let mut visitor = Topo::new(self.original_graph);

        while let Some(index) = visitor.next(self.original_graph) {
            self.maybe_cache_dependencies(&nodes, &mut deps, index);

            let (node_llb, output) = self.serialize_node(
                deps[index.index()].as_ref().unwrap(),
                self.original_graph.node_weight(index).unwrap(),
            );

            nodes[index.index()] = Some(node_llb.ref_counted().output(output.0));
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
                    .outputs_iter()
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

    fn serialize_node(
        &self,
        deps: &[Mount<'a, PathBuf>],
        node: &'a Node,
    ) -> (Command<'a>, OutputIdx) {
        let (mut command, index) = match node.command() {
            NodeCommand::Simple(ref details) => {
                self.serialize_command(self.create_target_dirs(node.output_dirs_iter()), details)
            }

            NodeCommand::WithBuildscript { compile, run } => {
                let (mut compile_command, compile_index) = {
                    self.serialize_command(
                        self.create_target_dirs(node.output_dirs_iter()),
                        compile,
                    )
                };

                for mount in deps {
                    compile_command = compile_command.mount(mount.clone());
                }

                self.serialize_command(compile_command.ref_counted().output(compile_index.0), run)
            }
        };

        for mount in deps {
            command = command.mount(mount.clone());
        }

        if let NodeKind::BuildScriptOutputConsumer(_) = node.kind() {
            command = command.mount(Mount::ReadOnlySelector(
                TOOLS_IMAGE.output(),
                BUILDSCRIPT_APPLY_EXEC,
                BUILDSCRIPT_APPLY_EXEC,
            ));
        }

        if let NodeKind::MergedBuildScript(_) = node.kind() {
            command = command.mount(Mount::ReadOnlySelector(
                TOOLS_IMAGE.output(),
                BUILDSCRIPT_CAPTURE_EXEC,
                BUILDSCRIPT_CAPTURE_EXEC,
            ));
        }

        (command, index)
    }

    fn create_target_dirs(
        &self,
        outputs: impl Iterator<Item = &'a Path>,
    ) -> OperationOutput<'static> {
        let mut operation = FileSystem::sequence();

        for output in outputs {
            let path = output.strip_prefix(TARGET_PATH).unwrap();

            let (index, layer_path) = match operation.last_output_index() {
                Some(index) => (index + 1, LayerPath::Own(OwnOutputIdx(index), path)),
                None => (0, LayerPath::Scratch(path)),
            };

            operation = operation
                .append(FileSystem::mkdir(OutputIdx(index), layer_path).make_parents(true));
        }

        operation.ref_counted().last_output().unwrap()
    }

    fn serialize_command<'b: 'a>(
        &self,
        target_layer: OperationOutput<'b>,
        command: &'b NodeCommandDetails,
    ) -> (Command<'a>, OutputIdx) {
        let mut command_llb = {
            self.image
                .populate_env(Command::run(&command.program))
                .cwd(&command.cwd)
                .args(&command.args)
                .env_iter(&command.env)
                .mount(Mount::ReadOnlyLayer(self.image.source().output(), "/"))
                .mount(Mount::Layer(OutputIdx(0), target_layer, TARGET_PATH))
                .mount(Mount::Scratch(OutputIdx(1), "/tmp"))
        };

        if command.cwd.starts_with(CONTEXT_PATH) {
            command_llb = command_llb.mount(Mount::ReadOnlyLayer(CONTEXT.output(), CONTEXT_PATH));
        }

        (command_llb, OutputIdx(0))
    }
}
