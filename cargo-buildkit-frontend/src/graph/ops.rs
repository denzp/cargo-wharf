use std::path::{Path, PathBuf};

use log::*;
use petgraph::prelude::*;

use super::node::{Node, NodeKind};

pub fn merge_buildscript_nodes(graph: &mut StableGraph<Node, ()>) {
    debug!("merging build script nodes");
    let indices = graph.node_indices().collect::<Vec<_>>();

    for index in indices {
        match graph.node_weight(index) {
            Some(node) if node.kind() == NodeKind::BuildScript => {
                let mut dependenents = {
                    graph
                        .neighbors_directed(index, Direction::Incoming)
                        .detach()
                };

                while let Some(dependent) = dependenents.next_node(&graph) {
                    move_edges(graph, index, dependent);
                    merge_buildscript_node_into(graph, index, dependent);
                }

                debug!("removing old buildscript node: {:#?}", index);
                graph.remove_node(index);
            }

            _ => {
                debug!("skipping non-buildscript node: {:?}", index);
            }
        }
    }
}

pub fn apply_buildscript_outputs(graph: &mut StableGraph<Node, ()>) {
    debug!("applying build script output");
    let indices = graph.node_indices().collect::<Vec<_>>();

    for index in indices {
        if let Some(node) = graph.node_weight(index) {
            if let NodeKind::MergedBuildScript(path) = node.kind() {
                let path = PathBuf::from(path);
                let mut dependenents = {
                    graph
                        .neighbors_directed(index, Direction::Outgoing)
                        .detach()
                };

                while let Some(dependent) = dependenents.next_node(&graph) {
                    debug!("transforming buildscript consumer: {:#?}", dependent);
                    graph[dependent].transform_into_buildscript_consumer(&path);
                }
            } else {
                debug!("skipping non-buildscript node: {:?}", index);
            }
        }
    }
}

fn move_edges(graph: &mut StableGraph<Node, ()>, from: NodeIndex<u32>, to: NodeIndex<u32>) {
    debug!("moving edges from '{:?}' to '{:?}'", from, to);

    let mut dependencies = graph.neighbors_directed(from, Direction::Outgoing).detach();

    while let Some(dependency) = dependencies.next_node(&graph) {
        graph.add_edge(to, dependency, ());
    }
}

fn merge_buildscript_node_into(
    graph: &mut StableGraph<Node, ()>,
    from: NodeIndex<u32>,
    to: NodeIndex<u32>,
) {
    debug!("merging buildscript node '{:?}' into '{:?}'", from, to);

    if graph[from].command().is_simple() {
        let mut run_node = graph.remove_node(from).unwrap().into_command_details();

        let real_buildscript_path = {
            graph[to]
                .links_iter()
                .filter(|(to, _)| *to == Path::new(&run_node.program))
                .map(|(_, from)| from)
                .next()
        };

        if let Some(path) = real_buildscript_path {
            run_node.program = path.to_string_lossy().into();
        }

        graph[to].add_buildscript_run_command(run_node);
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use maplit::btreemap;

    use super::*;
    use crate::graph::{NodeCommand, NodeCommandDetails, NodeKind};
    use crate::plan::{RawInvocation, RawTargetKind};

    #[test]
    fn buildscript_merging() {
        let mut graph = mock_buildscript_graph();

        merge_buildscript_nodes(&mut graph);
        assert_eq!(graph.node_count(), 2);

        let buildscript_node = graph.node_weight(NodeIndex::from(0)).unwrap();

        assert_eq!(
            buildscript_node.kind(),
            NodeKind::MergedBuildScript(Path::new(
                "/target/debug/build/lib-1-c181ff77de97ab79/out"
            ))
        );

        assert_eq!(buildscript_node.links_iter().collect::<Vec<_>>(), vec![]);
        assert_eq!(
            buildscript_node.outputs_iter().collect::<Vec<_>>(),
            vec![Path::new(
                "/target/debug/build/lib-1-b110aa734b821ac2/build_script_build-b110aa734b821ac2"
            )],
        );
        assert_eq!(
            buildscript_node.output_dirs_iter().collect::<Vec<_>>(),
            vec![
                Path::new("/target/debug/build/lib-1-b110aa734b821ac2"),
                Path::new("/target/debug/build/lib-1-c181ff77de97ab79/out")
            ],
        );

        assert_eq!(
            buildscript_node.command(),
            &NodeCommand::WithBuildscript {
                compile: NodeCommandDetails {
                    program: "rustc".into(),
                    cwd: "/context".into(),
                    args: vec![
                        "--edition=2018".into(),
                        "--crate-name".into(),
                        "build_script_build".into(),
                        "lib-1/build.rs".into(),
                    ],
                    env: btreemap! {
                        "CARGO_MANIFEST_DIR".into() => "/context/lib-1".into(),
                        "CARGO_PKG_NAME".into() => "lib-1".into(),
                    },
                },

                run: NodeCommandDetails {
                    program: "/usr/local/bin/cargo-buildscript-capture".into(),
                    cwd: "/context/lib-1".into(),
                    args: vec![
                        "--".into(),
                        "/target/debug/build/lib-1-b110aa734b821ac2/build_script_build-b110aa734b821ac2".into()
                    ],
                    env: btreemap! {
                        "CARGO_MANIFEST_DIR".into() => "/context/lib-1".into(),
                        "OUT_DIR".into() => "/target/debug/build/lib-1-c181ff77de97ab79/out".into(),
                        "HOST".into() => "x86_64-unknown-linux-gnu".into(),
                        "TARGET".into() => "x86_64-unknown-linux-gnu".into(),
                    },
                },
            }
        );

        let rustc_node = graph.node_weight(NodeIndex::from(2)).unwrap();

        assert_eq!(rustc_node.kind(), NodeKind::Other);
        assert_eq!(
            rustc_node.command(),
            &NodeCommand::Simple(NodeCommandDetails {
                program: "rustc".into(),
                cwd: "/context".into(),
                args: vec![
                    "--edition=2018".into(),
                    "--crate-name".into(),
                    "lib_1".into(),
                    "lib-1/src/lib.rs".into(),
                ],
                env: btreemap! {
                    "CARGO_MANIFEST_DIR".into() => "/context/lib-1".into(),
                    "CARGO_PKG_NAME".into() => "lib-1".into(),
                    "OUT_DIR".into() => "/target/debug/build/lib-1-c181ff77de97ab79/out".into(),
                },
            })
        );
    }

    #[test]
    fn buildscript_result_applying() {
        let mut graph = mock_buildscript_graph();

        merge_buildscript_nodes(&mut graph);
        apply_buildscript_outputs(&mut graph);
        assert_eq!(graph.node_count(), 2);

        let rustc_node = graph.node_weight(NodeIndex::from(2)).unwrap();

        assert_eq!(
            rustc_node.kind(),
            NodeKind::BuildScriptOutputConsumer(Path::new(
                "/target/debug/build/lib-1-c181ff77de97ab79/out"
            ))
        );

        assert_eq!(
            rustc_node.command(),
            &NodeCommand::Simple(NodeCommandDetails {
                program: "/usr/local/bin/cargo-buildscript-apply".into(),
                cwd: "/context".into(),
                args: vec![
                    "--".into(),
                    "rustc".into(),
                    "--edition=2018".into(),
                    "--crate-name".into(),
                    "lib_1".into(),
                    "lib-1/src/lib.rs".into(),
                ],
                env: btreemap! {
                    "CARGO_MANIFEST_DIR".into() => "/context/lib-1".into(),
                    "CARGO_PKG_NAME".into() => "lib-1".into(),
                    "OUT_DIR".into() => "/target/debug/build/lib-1-c181ff77de97ab79/out".into(),
                },
            })
        );
    }

    fn mock_buildscript_graph() -> StableGraph<Node, ()> {
        let mut graph = StableGraph::new();

        let compile_idx = graph.add_node(Node::from(&RawInvocation {
            package_name: "lib-1".into(),
            package_version: "0.1.0".parse().unwrap(),

            target_kind: vec![RawTargetKind::CustomBuild],
            deps: vec![],

            outputs: vec![
                "/target/debug/build/lib-1-b110aa734b821ac2/build_script_build-b110aa734b821ac2".into(),
            ],
            links: btreemap! {
                "/target/debug/build/lib-1-b110aa734b821ac2/build-script-build".into() => "/target/debug/build/lib-1-b110aa734b821ac2/build_script_build-b110aa734b821ac2".into()
            },

            program: "rustc".into(),
            cwd: "/context".into(),
            args: vec![
                "--edition=2018".into(),
                "--crate-name".into(),
                "build_script_build".into(),
                "lib-1/build.rs".into(),
            ],
            env: btreemap! {
                "CARGO_MANIFEST_DIR".into() => "/context/lib-1".into(),
                "CARGO_PKG_NAME".into() => "lib-1".into(),
            },
        }));

        let run_idx = graph.add_node(Node::from(&RawInvocation {
            package_name: "lib-1".into(),
            package_version: "0.1.0".parse().unwrap(),

            target_kind: vec![RawTargetKind::CustomBuild],
            deps: vec![0],

            outputs: vec![],
            links: btreemap! {},

            program: "/target/debug/build/lib-1-b110aa734b821ac2/build-script-build".into(),
            cwd: "/context/lib-1".into(),
            args: vec![],
            env: btreemap! {
                "CARGO_MANIFEST_DIR".into() => "/context/lib-1".into(),
                "OUT_DIR".into() => "/target/debug/build/lib-1-c181ff77de97ab79/out".into(),
                "HOST".into() => "x86_64-unknown-linux-gnu".into(),
                "TARGET".into() => "x86_64-unknown-linux-gnu".into(),
            },
        }));

        let rustc_idx = graph.add_node(Node::from(&RawInvocation {
            package_name: "lib-1".into(),
            package_version: "0.1.0".parse().unwrap(),

            target_kind: vec![RawTargetKind::Lib],
            deps: vec![1],

            outputs: vec![
                "/target/debug/deps/liblib_1-b8a5ab4c34b4b2c1.rlib".into(),
                "/target/debug/deps/liblib_1-b8a5ab4c34b4b2c1.rmeta".into(),
            ],
            links: btreemap! {
                "/target/debug/liblib_1.rlib".into() => "/target/debug/deps/liblib_1-b8a5ab4c34b4b2c1.rlib".into()
            },

            program: "rustc".into(),
            cwd: "/context".into(),
            args: vec![
                "--edition=2018".into(),
                "--crate-name".into(),
                "lib_1".into(),
                "lib-1/src/lib.rs".into(),
            ],
            env: btreemap! {
                "CARGO_MANIFEST_DIR".into() => "/context/lib-1".into(),
                "CARGO_PKG_NAME".into() => "lib-1".into(),
                "OUT_DIR".into() => "/target/debug/build/lib-1-c181ff77de97ab79/out".into(),
            },
        }));

        graph.add_edge(compile_idx, run_idx, ());
        graph.add_edge(run_idx, rustc_idx, ());
        graph
    }
}
