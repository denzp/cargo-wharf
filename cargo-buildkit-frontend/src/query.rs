use std::path::{Path, PathBuf};

use chrono::prelude::*;
use failure::Error;
use log::*;
use petgraph::prelude::*;
use petgraph::visit::{Reversed, Topo, Walker};

use buildkit_frontend::oci::*;
use buildkit_frontend::{Bridge, OutputRef};
use buildkit_llb::prelude::*;
use buildkit_proto::pb;

use crate::config::Config;
use crate::graph::{BuildGraph, Node, NodeCommand, NodeCommandDetails, NodeKind};
use crate::shared::{tools, CONTEXT, CONTEXT_PATH, TARGET_PATH};

pub struct GraphQuery<'a> {
    original_graph: &'a StableGraph<Node, ()>,
    reversed_graph: Reversed<&'a StableGraph<Node, ()>>,

    config: &'a Config,
}

struct OutputMapping<'a> {
    from: LayerPath<'a, PathBuf>,
    to: PathBuf,
}

type NodesCache<'a> = Vec<Option<OperationOutput<'a>>>;

impl<'a> GraphQuery<'a> {
    pub fn new(graph: &'a BuildGraph, config: &'a Config) -> Self {
        Self {
            original_graph: graph.inner(),
            reversed_graph: Reversed(graph.inner()),

            config,
        }
    }

    pub fn definition(&self) -> pb::Definition {
        self.terminal().into_definition()
    }

    pub async fn solve(&self, bridge: &mut Bridge) -> Result<OutputRef, Error> {
        bridge.solve(self.terminal()).await
    }

    pub fn image_spec(&self) -> Result<ImageSpecification, Error> {
        let output = self.config.output_image();

        Ok(ImageSpecification {
            created: Some(Utc::now()),
            author: None,

            // TODO: don't hardcode this
            architecture: Architecture::Amd64,
            os: OperatingSystem::Linux,

            config: Some(ImageConfig {
                entrypoint: output.entrypoint.clone(),
                cmd: output.cmd.clone(),
                env: output.env.clone(),
                user: output.user.clone(),
                working_dir: output.workdir.clone(),

                labels: None,
                volumes: None,
                exposed_ports: None,
                stop_signal: None,
            }),

            rootfs: None,
            history: None,
        })
    }

    fn terminal(&self) -> Terminal<'a> {
        debug!("serializing all nodes");
        let nodes = self.serialize_all_nodes();
        let outputs = self.outputs(nodes);

        debug!("preparing the final operation");

        let operation = FileSystem::sequence().custom_name("Composing the output image");
        let operation = {
            outputs.into_iter().fold(operation, |output, mapping| {
                let (index, layer_path) = match output.last_output_index() {
                    Some(index) => (index + 1, LayerPath::Own(OwnOutputIdx(index), mapping.to)),
                    None => (0, self.config.output_image().layer_path(mapping.to)),
                };

                output.append(
                    FileSystem::copy()
                        .from(mapping.from)
                        .to(OutputIdx(index), layer_path)
                        .create_path(true),
                )
            })
        };

        Terminal::with(operation.ref_counted().last_output().unwrap())
    }

    fn outputs(&self, nodes: NodesCache<'a>) -> Vec<OutputMapping<'a>> {
        self.original_graph
            .node_indices()
            .map(move |index| (index, self.original_graph.node_weight(index).unwrap()))
            .filter(|(_, node)| node.kind() == NodeKind::Binary)
            .filter_map(
                move |(index, node)| match self.config.find_binary(node.package_name()) {
                    Some(found) => Some((index, node, found.destination.clone())),
                    None => None,
                },
            )
            .map(move |(index, node, to)| {
                let from = LayerPath::Other(
                    nodes[index.index()].clone().unwrap(),
                    node.outputs_iter()
                        .next()
                        .unwrap()
                        .strip_prefix(TARGET_PATH)
                        .unwrap()
                        .into(),
                );

                OutputMapping { from, to }
            })
            .collect()
    }

    fn serialize_all_nodes(&self) -> NodesCache<'a> {
        let mut nodes = vec![None; self.original_graph.capacity().0];
        let mut deps = vec![None; self.original_graph.capacity().0];

        let mut visitor = Topo::new(self.original_graph);

        while let Some(index) = visitor.next(self.original_graph) {
            self.maybe_cache_dependencies(&nodes, &mut deps, index);

            let (node_llb, output) = serialize_node(
                &self.config,
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
}

fn serialize_node<'a>(
    config: &'a Config,
    deps: &[Mount<'a, PathBuf>],
    node: &'a Node,
) -> (Command<'a>, OutputIdx) {
    let (mut command, index) = match node.command() {
        NodeCommand::Simple(ref details) => {
            serialize_command(config, create_target_dirs(node.output_dirs_iter()), details)
        }

        NodeCommand::WithBuildscript { compile, run } => {
            let (mut compile_command, compile_index) =
                serialize_command(config, create_target_dirs(node.output_dirs_iter()), compile);

            for mount in deps {
                compile_command = compile_command.mount(mount.clone());
            }

            serialize_command(
                config,
                compile_command.ref_counted().output(compile_index.0),
                run,
            )
        }
    };

    for mount in deps {
        command = command.mount(mount.clone());
    }

    if let NodeKind::BuildScriptOutputConsumer(_) = node.kind() {
        command = command.mount(Mount::ReadOnlySelector(
            tools::IMAGE.output(),
            tools::BUILDSCRIPT_APPLY,
            tools::BUILDSCRIPT_APPLY,
        ));
    }

    if let NodeKind::MergedBuildScript(_) = node.kind() {
        command = command.mount(Mount::ReadOnlySelector(
            tools::IMAGE.output(),
            tools::BUILDSCRIPT_CAPTURE,
            tools::BUILDSCRIPT_CAPTURE,
        ));
    }

    (command, index)
}

fn serialize_command<'a, 'b: 'a>(
    config: &'a Config,
    target_layer: OperationOutput<'b>,
    command: &'b NodeCommandDetails,
) -> (Command<'a>, OutputIdx) {
    let builder = config.builder_image();

    let mut command_llb = {
        builder
            .populate_env(Command::run(&command.program))
            .cwd(&command.cwd)
            .args(&command.args)
            .env_iter(&command.env)
            .mount(Mount::ReadOnlyLayer(builder.source().output(), "/"))
            .mount(Mount::Layer(OutputIdx(0), target_layer, TARGET_PATH))
            .mount(Mount::Scratch(OutputIdx(1), "/tmp"))
    };

    if command.cwd.starts_with(CONTEXT_PATH) {
        command_llb = command_llb.mount(Mount::ReadOnlyLayer(CONTEXT.output(), CONTEXT_PATH));
    }

    (command_llb, OutputIdx(0))
}

fn create_target_dirs<'a>(outputs: impl Iterator<Item = &'a Path>) -> OperationOutput<'static> {
    let mut operation = FileSystem::sequence();

    for output in outputs {
        let path = output.strip_prefix(TARGET_PATH).unwrap();

        let (index, layer_path) = match operation.last_output_index() {
            Some(index) => (index + 1, LayerPath::Own(OwnOutputIdx(index), path)),
            None => (0, LayerPath::Scratch(path)),
        };

        let inner = FileSystem::mkdir(OutputIdx(index), layer_path).make_parents(true);

        operation = operation.append(inner);
    }

    operation.ref_counted().last_output().unwrap()
}
