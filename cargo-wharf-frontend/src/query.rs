use std::convert::TryFrom;
use std::iter::{empty, once};
use std::path::{Path, PathBuf};

use chrono::prelude::*;
use either::Either;
use failure::{bail, Error};
use log::*;
use petgraph::prelude::*;
use petgraph::visit::{Reversed, Topo, Walker};
use serde::{Deserialize, Serialize};

use buildkit_frontend::oci::*;
use buildkit_frontend::{Bridge, OutputRef};
use buildkit_llb::prelude::*;
use buildkit_proto::pb;

use crate::config::{BaseImageConfig, Config, CustomCommand};
use crate::frontend::Options;
use crate::graph::{
    BuildGraph, Node, NodeCommand, NodeCommandDetails, NodeKind, PrimitiveNodeKind,
};
use crate::shared::{tools, CONTEXT, CONTEXT_PATH, TARGET_PATH};

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(try_from = "String")]
pub enum Profile {
    ReleaseBinaries,
    DebugBinaries,

    ReleaseTests,
    DebugTests,
}

pub struct GraphQuery<'a> {
    original_graph: &'a StableGraph<Node, ()>,
    reversed_graph: Reversed<&'a StableGraph<Node, ()>>,

    config: &'a Config,
    builder_source: Option<OperationOutput<'a>>,
    output_source: Option<OperationOutput<'a>>,
}

struct OutputMapping<'a> {
    from: LayerPath<'a, PathBuf>,
    to: PathBuf,
}

type NodesCache<'a> = Vec<Option<OperationOutput<'a>>>;
type BuildOutput<'a> = (NodeIndex, &'a Node, PathBuf);

impl<'a> GraphQuery<'a> {
    pub fn new(graph: &'a BuildGraph, config: &'a Config) -> Self {
        let builder_source = Self::source_llb(
            config.builder(),
            config
                .builder()
                .setup_commands()
                .map(Vec::as_ref)
                .unwrap_or_default(),
        );

        let output_source = Self::source_llb(
            config.output(),
            config
                .output()
                .pre_install_commands()
                .map(Vec::as_ref)
                .unwrap_or_default(),
        );

        Self {
            original_graph: graph.inner(),
            reversed_graph: Reversed(graph.inner()),

            config,
            builder_source,
            output_source,
        }
    }

    pub fn definition(&self) -> Result<pb::Definition, Error> {
        Ok(self.terminal()?.into_definition())
    }

    pub async fn solve(&self, bridge: &mut Bridge, options: &Options) -> Result<OutputRef, Error> {
        bridge
            .solve_with_cache(self.terminal()?, options.cache_entries())
            .await
    }

    pub fn image_spec(&self) -> Result<ImageSpecification, Error> {
        let output = self.config.output();

        let config = match self.config.profile() {
            Profile::ReleaseBinaries | Profile::DebugBinaries => self.config.output().into(),
            Profile::ReleaseTests | Profile::DebugTests => ImageConfig {
                entrypoint: Some(
                    once(tools::TEST_RUNNER.into())
                        .chain(
                            self.outputs()
                                .map(|(_, _, path)| path.to_string_lossy().into()),
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

    fn source_llb(
        config: &'a dyn BaseImageConfig,
        commands: &'a [CustomCommand],
    ) -> Option<OperationOutput<'a>> {
        if !commands.is_empty() {
            let mut last_output = config.image_source().map(|source| source.output());

            for (name, args, display) in commands.iter().map(From::from) {
                last_output = Some(
                    config
                        .populate_env(Command::run(name))
                        .args(args.iter())
                        .mount(match last_output {
                            Some(output) => Mount::Layer(OutputIdx(0), output, "/"),
                            None => Mount::Scratch(OutputIdx(0), "/"),
                        })
                        .custom_name(format!("Running   `{}`", display))
                        .ref_counted()
                        .output(0),
                );
            }

            last_output
        } else {
            config.image_source().map(|source| source.output())
        }
    }

    fn terminal(&self) -> Result<Terminal<'a>, Error> {
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

        let mut commands_iter = {
            self.config
                .output()
                .post_install_commands()
                .map(|commands| Either::Left(commands.iter().map(From::from)))
                .unwrap_or_else(|| Either::Right(empty()))
        };

        if let Some((name, args, display)) = commands_iter.next() {
            let mut output = {
                self.config
                    .output()
                    .populate_env(Command::run(name))
                    .args(args.iter())
                    .mount(Mount::Layer(
                        OutputIdx(0),
                        operation.ref_counted().last_output().unwrap(),
                        "/",
                    ))
                    .custom_name(format!("Running   `{}`", display))
                    .ref_counted()
                    .output(0)
            };

            for (name, args, display) in commands_iter {
                output = {
                    self.config
                        .output()
                        .populate_env(Command::run(name))
                        .args(args.iter())
                        .mount(Mount::Layer(OutputIdx(0), output, "/"))
                        .custom_name(format!("Running   `{}`", display))
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

    fn output_layer_path<P>(&self, path: P) -> LayerPath<'a, P>
    where
        P: AsRef<Path>,
    {
        match self.output_source {
            Some(ref output) => LayerPath::Other(output.clone(), path),
            None => LayerPath::Scratch(path),
        }
    }

    fn outputs(&self) -> impl Iterator<Item = BuildOutput<'_>> {
        match self.config.profile() {
            Profile::ReleaseBinaries | Profile::DebugBinaries => Either::Left(
                self.original_graph
                    .node_indices()
                    .map(move |index| (index, self.original_graph.node_weight(index).unwrap()))
                    .filter_map(move |(index, node)| {
                        match self.config.find_binary(node.binary_name()?) {
                            Some(found) => Some((index, node, found.destination.clone())),
                            None => None,
                        }
                    }),
            ),

            Profile::ReleaseTests | Profile::DebugTests => Either::Right(
                self.original_graph
                    .node_indices()
                    .map(move |index| (index, self.original_graph.node_weight(index).unwrap()))
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

                        (index, node, PathBuf::from("/test").join(to))
                    }),
            ),
        }
    }

    fn mapped_outputs(&self, nodes: NodesCache<'a>) -> Vec<OutputMapping<'a>> {
        match self.config.profile() {
            Profile::ReleaseBinaries | Profile::DebugBinaries => {
                self.binaries_mapped_outputs(nodes)
            }

            Profile::ReleaseTests | Profile::DebugTests => self.tests_mapped_outputs(nodes),
        }
    }

    fn binaries_mapped_outputs(&self, nodes: NodesCache<'a>) -> Vec<OutputMapping<'a>> {
        self.outputs()
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

    fn tests_mapped_outputs(&self, nodes: NodesCache<'a>) -> Vec<OutputMapping<'a>> {
        self.outputs()
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
            .chain(once(OutputMapping {
                from: LayerPath::Other(tools::IMAGE.output(), tools::TEST_RUNNER.into()),
                to: tools::TEST_RUNNER.into(),
            }))
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
                self.builder_source.clone().unwrap(),
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
    source: OperationOutput<'a>,
    deps: &[Mount<'a, PathBuf>],
    node: &'a Node,
) -> (Command<'a>, OutputIdx) {
    let (mut command, index) = match node.command() {
        NodeCommand::Simple(ref details) => serialize_command(
            config,
            source,
            create_target_dirs(node.output_dirs_iter()),
            details,
        ),

        NodeCommand::WithBuildscript { compile, run } => {
            let (mut compile_command, compile_index) = serialize_command(
                config,
                source.clone(),
                create_target_dirs(node.output_dirs_iter()),
                compile,
            );

            compile_command = compile_command
                .custom_name(format!("Compiling {} [build script]", node.package_name()));

            for mount in deps {
                compile_command = compile_command.mount(mount.clone());
            }

            serialize_command(
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

    let pretty_name = match node.kind() {
        NodeKind::BuildScriptOutputConsumer(PrimitiveNodeKind::Binary, _) => {
            format!("Compiling binary {}", node.binary_name().unwrap())
        }

        NodeKind::Primitive(PrimitiveNodeKind::Binary) => {
            format!("Compiling binary {}", node.binary_name().unwrap())
        }

        NodeKind::BuildScriptOutputConsumer(PrimitiveNodeKind::Test, _) => {
            format!("Compiling test {}", node.test_name().unwrap())
        }

        NodeKind::Primitive(PrimitiveNodeKind::Test) => {
            format!("Compiling test {}", node.test_name().unwrap())
        }

        NodeKind::MergedBuildScript(_) => {
            format!("Running   {} [build script]", node.package_name())
        }

        _ => format!("Compiling {}", node.package_name()),
    };

    (command.custom_name(pretty_name), index)
}

fn serialize_command<'a, 'b: 'a>(
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

impl TryFrom<String> for Profile {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "release" | "release-binaries" => Ok(Profile::ReleaseBinaries),
            "debug" | "debug-binaries" => Ok(Profile::DebugBinaries),
            "test" | "release-test" => Ok(Profile::ReleaseTests),
            "debug-test" => Ok(Profile::DebugTests),

            other => bail!("Unknown mode: {}", other),
        }
    }
}

impl Default for Profile {
    fn default() -> Self {
        Profile::ReleaseBinaries
    }
}

#[cfg(test)]
mod tests {
    use serde_json::from_slice;

    use super::*;
    use crate::config::{BinaryDefinition, BuilderConfig, OutputConfig};
    use crate::plan::RawBuildPlan;

    #[test]
    fn query_binaries() {
        let graph = create_graph();
        let config = create_config(Profile::ReleaseBinaries);
        let query = GraphQuery::new(&graph, &config);

        assert_eq!(
            query
                .outputs()
                .map(|(index, _, path)| (index, path))
                .collect::<Vec<_>>(),
            vec![(NodeIndex::new(15), "/usr/bin/mock-binary-1".into())]
        );
    }

    #[test]
    fn query_tests() {
        let graph = create_graph();
        let config = create_config(Profile::ReleaseTests);
        let query = GraphQuery::new(&graph, &config);

        assert_eq!(
            query
                .outputs()
                .map(|(index, _, path)| (index, path))
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

    fn create_graph() -> BuildGraph {
        BuildGraph::from(
            from_slice::<RawBuildPlan>(include_bytes!("../tests/build-plan.json")).unwrap(),
        )
    }

    fn create_config(profile: Profile) -> Config {
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

        Config::mocked_new(builder, output, profile, binaries)
    }
}
