use std::collections::BTreeSet;
use std::path::PathBuf;

use log::*;
use petgraph::prelude::*;

use super::node::BuildScriptMergeResult::*;
use super::node::{Node, NodeKind, PrimitiveNodeKind};

pub fn merge_buildscript_nodes(graph: &mut StableGraph<Node, ()>) {
    debug!("merging build script nodes");
    let indices = graph.node_indices().collect::<Vec<_>>();
    let mut nodes_for_removal = BTreeSet::new();

    for index in indices {
        let mut dependency_build_scripts = BTreeSet::new();

        match graph.node_weight(index) {
            Some(node) if node.kind() == NodeKind::Primitive(PrimitiveNodeKind::BuildScriptRun) => {
                let mut compile_indexes = {
                    graph
                        .neighbors_directed(index, Direction::Incoming)
                        .detach()
                };

                let run_index = index;
                while let Some(compile_index) = compile_indexes.next_node(&graph) {
                    let compile_node = graph[compile_index].clone();

                    match graph[run_index].add_buildscript_compile_node(compile_node) {
                        Ok => {
                            debug!(
                                "merged buildscript compile '{:?}' with run '{:?}'",
                                compile_index, run_index
                            );

                            move_edges(graph, compile_index, run_index);
                            nodes_for_removal.insert(compile_index);
                            break;
                        }

                        DependencyBuildScript => {
                            dependency_build_scripts.insert(compile_index);
                        }

                        AlreadyMerged => {
                            break;
                        }
                    }
                }
            }

            _ => {
                debug!("skipping non-buildscript node: {:?}", index);
            }
        }

        for dep_index in dependency_build_scripts {
            debug!("adding dependency buildscript: {:?}", index);

            let dep = graph[dep_index].clone();
            graph[index].add_dependency_buildscript(dep);
            graph.add_edge(dep_index, index, ());
        }
    }

    for index in nodes_for_removal {
        debug!("removing old buildscript node: {:#?}", index);
        graph.remove_node(index);
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
                    match graph[dependent].kind() {
                        NodeKind::MergedBuildScript { .. } => {}

                        _ => {
                            debug!("transforming buildscript consumer: {:#?}", dependent);
                            graph[dependent].transform_into_buildscript_consumer(&path);
                        }
                    }
                }
            } else {
                debug!("skipping non-buildscript node: {:?}", index);
            }
        }
    }
}

fn move_edges(graph: &mut StableGraph<Node, ()>, from: NodeIndex<u32>, to: NodeIndex<u32>) {
    debug!("moving edges from '{:?}' to '{:?}'", from, to);

    let mut dependencies = graph.neighbors_directed(from, Direction::Incoming).detach();

    while let Some(dependency) = dependencies.next_node(&graph) {
        graph.add_edge(dependency, to, ());
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use serde_json::from_slice;

    use super::*;
    use crate::graph::NodeKind;
    use crate::plan::RawBuildPlan;

    #[test]
    fn buildscript_merging() {
        let mut graph = mock_buildscript_graph();
        assert_eq!(graph.node_count(), 19);

        merge_buildscript_nodes(&mut graph);
        assert_eq!(graph.node_count(), 16);

        let buildscript_nodes = {
            graph
                .node_indices()
                .filter_map(|index| match graph[index].kind() {
                    NodeKind::MergedBuildScript { .. } => Some(graph[index].clone()),
                    _ => None,
                })
        };

        assert_eq!(
            buildscript_nodes.collect::<Vec<Node>>(),
            from_slice::<Vec<Node>>(include_bytes!(
                "../../tests/merged-buildscript-producers.json"
            ))
            .unwrap(),
        );
    }

    #[test]
    fn buildscript_result_applying() {
        let mut graph = mock_buildscript_graph();
        assert_eq!(graph.node_count(), 19);

        merge_buildscript_nodes(&mut graph);
        apply_buildscript_outputs(&mut graph);
        assert_eq!(graph.node_count(), 16);

        let consumer_nodes = {
            graph
                .node_indices()
                .filter_map(|index| match graph[index].kind() {
                    NodeKind::BuildScriptOutputConsumer(_, _) => Some(graph[index].clone()),
                    _ => None,
                })
        };

        assert_eq!(
            consumer_nodes.collect::<Vec<Node>>(),
            from_slice::<Vec<Node>>(include_bytes!(
                "../../tests/merged-buildscript-consumers.json"
            ))
            .unwrap(),
        );
    }

    fn mock_buildscript_graph() -> StableGraph<Node, ()> {
        StableGraph::from(
            from_slice::<RawBuildPlan>(include_bytes!("../../tests/build-plan.json")).unwrap(),
        )
    }
}
