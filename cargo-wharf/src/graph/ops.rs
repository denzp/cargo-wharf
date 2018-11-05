use log::*;
use petgraph::prelude::*;

use super::node::{Node, NodeKind};
use super::Command;

pub fn merge_build_script_nodes(graph: &mut StableGraph<Node, usize>) {
    let indices = graph.node_indices().collect::<Vec<_>>();

    for index in indices {
        match graph.node_weight(index) {
            Some(node) if node.kind() == &NodeKind::BuildScript => {
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

fn move_edges(graph: &mut StableGraph<Node, usize>, from: NodeIndex<u32>, to: NodeIndex<u32>) {
    debug!("moving edges from '{:?}' to '{:?}'", from, to);

    let mut dependencies = graph.neighbors_directed(from, Direction::Outgoing).detach();

    while let Some(dependency) = dependencies.next_node(&graph) {
        graph.add_edge(to, dependency, 0);
    }
}

fn merge_buildscript_node_into(
    graph: &mut StableGraph<Node, usize>,
    from: NodeIndex<u32>,
    to: NodeIndex<u32>,
) {
    debug!("merging buildscript node '{:?}' into '{:?}'", from, to);

    if let Command::Simple(command) = graph[from].command().clone() {
        graph[to].add_buildscript_command(command);
    }
}
