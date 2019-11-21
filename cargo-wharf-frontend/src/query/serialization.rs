use std::path::{Path, PathBuf};

use petgraph::prelude::*;
use petgraph::visit::{Topo, Walker};

use buildkit_llb::prelude::*;

use crate::config::{BaseImageConfig, Config};
use crate::graph::{Node, NodeCommand, NodeCommandDetails, NodeKind, PrimitiveNodeKind};
use crate::shared::{tools, CONTEXT, CONTEXT_PATH, TARGET_PATH};

use super::print::{PrettyPrintQuery, PrintKind};
use super::{SourceQuery, WharfDatabase};

type NodesCache<'a> = Vec<Option<OperationOutput<'a>>>;

pub trait SerializationQuery: WharfDatabase + SourceQuery + PrettyPrintQuery {
    fn serialize_all_nodes(&self) -> NodesCache<'_> {
        let mut nodes = vec![None; self.graph().capacity().0];
        let mut deps = vec![None; self.graph().capacity().0];

        let mut visitor = Topo::new(self.graph());

        while let Some(index) = visitor.next(self.graph()) {
            self.maybe_cache_dependencies(&nodes, &mut deps, index);

            let (node_llb, output) = self.serialize_node(
                &self.config(),
                self.builder_source().clone().unwrap(),
                deps[index.index()].as_ref().unwrap(),
                self.graph().node_weight(index).unwrap(),
            );

            nodes[index.index()] = Some(node_llb.ref_counted().output(output.0));
        }

        nodes
    }

    fn maybe_cache_dependencies<'a>(
        &self,
        nodes: &[Option<OperationOutput<'a>>],
        deps: &mut Vec<Option<Vec<Mount<'a, PathBuf>>>>,
        index: NodeIndex,
    ) {
        if deps[index.index()].is_some() {
            return;
        }

        let local_deps = DfsPostOrder::new(&self.reversed_graph(), index)
            .iter(self.reversed_graph())
            .filter(|dep_index| dep_index.index() != index.index())
            .flat_map(|dep_index| {
                self.graph()
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

    fn serialize_node<'a>(
        &self,
        config: &'a Config,
        source: OperationOutput<'a>,
        deps: &[Mount<'a, PathBuf>],
        node: &'a Node,
    ) -> (Command<'a>, OutputIdx) {
        let (mut command, index) = match node.command() {
            NodeCommand::Simple(ref details) => self.serialize_command(
                config,
                source,
                self.create_target_dirs(node.output_dirs_iter()),
                details,
            ),

            NodeCommand::WithBuildscript { compile, run } => {
                let (mut compile_command, compile_index) = self.serialize_command(
                    config,
                    source.clone(),
                    self.create_target_dirs(node.output_dirs_iter()),
                    compile,
                );

                compile_command = compile_command.custom_name(
                    self.pretty_print(PrintKind::CompileBuildScript(node.package_name())),
                );

                for mount in deps {
                    compile_command = compile_command.mount(mount.clone());
                }

                self.serialize_command(
                    config,
                    source,
                    compile_command.ref_counted().output(compile_index.0),
                    run,
                )
            }
        };

        for mount in deps {
            command = command.mount(mount.clone());
        }

        if let NodeKind::BuildScriptOutputConsumer(_, _) = node.kind() {
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

        let print_kind = match node.kind() {
            NodeKind::BuildScriptOutputConsumer(PrimitiveNodeKind::Binary, _) => {
                PrintKind::CompileBinary(node.binary_name().unwrap())
            }

            NodeKind::Primitive(PrimitiveNodeKind::Binary) => {
                PrintKind::CompileBinary(node.binary_name().unwrap())
            }

            NodeKind::BuildScriptOutputConsumer(PrimitiveNodeKind::Test, _) => {
                PrintKind::CompileTest(node.test_name().unwrap())
            }

            NodeKind::Primitive(PrimitiveNodeKind::Test) => {
                PrintKind::CompileTest(node.test_name().unwrap())
            }

            NodeKind::MergedBuildScript(_) => PrintKind::RunBuildScript(node.package_name()),

            _ => PrintKind::CompileCrate(node.package_name()),
        };

        (command.custom_name(self.pretty_print(print_kind)), index)
    }

    fn serialize_command<'a, 'b: 'a>(
        &self,
        config: &'a Config,
        source: OperationOutput<'a>,
        target_layer: OperationOutput<'b>,
        command: &'b NodeCommandDetails,
    ) -> (Command<'a>, OutputIdx) {
        let builder = config.builder();

        let mut command_llb = {
            builder
                .populate_env(Command::run(&command.program))
                .cwd(&command.cwd)
                .args(&command.args)
                .env_iter(&command.env)
                .mount(Mount::ReadOnlyLayer(source, "/"))
                .mount(Mount::Layer(OutputIdx(0), target_layer, TARGET_PATH))
                .mount(Mount::Scratch(OutputIdx(1), "/tmp"))
        };

        if command.cwd.starts_with(CONTEXT_PATH) {
            command_llb = command_llb.mount(Mount::ReadOnlyLayer(CONTEXT.output(), CONTEXT_PATH));
        }

        (command_llb, OutputIdx(0))
    }

    fn create_target_dirs<'a>(
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

            let inner = FileSystem::mkdir(OutputIdx(index), layer_path).make_parents(true);

            operation = operation.append(inner);
        }

        operation.ref_counted().last_output().unwrap()
    }
}
