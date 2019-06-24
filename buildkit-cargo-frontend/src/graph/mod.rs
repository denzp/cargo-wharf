use petgraph::prelude::*;

use crate::plan::RawBuildPlan;

mod node;
pub use self::node::{Node, NodeKind};

mod command;
pub use self::command::{Command, CommandDetails};

mod ops;
use self::ops::merge_build_script_nodes;

type NodeRef<'a> = (NodeIndex<u32>, &'a Node);

#[derive(Debug)]
pub struct BuildGraph {
    graph: StableGraph<Node, usize>,
}

impl From<RawBuildPlan> for BuildGraph {
    fn from(plan: RawBuildPlan) -> Self {
        let mut graph = StableGraph::<Node, usize>::new();

        let nodes = {
            plan.invocations
                .iter()
                .map(|item| graph.add_node(item.into()))
                .collect::<Vec<_>>()
        };

        for (item, index) in plan.invocations.iter().zip(0..) {
            let mut deps = item.deps.clone();

            deps.sort();
            for dep in deps.iter() {
                graph.add_edge(nodes[index], nodes[*dep as usize], 0);
            }
        }

        merge_build_script_nodes(&mut graph);

        Self { graph }
    }
}

impl BuildGraph {
    pub fn nodes(&self) -> impl Iterator<Item = NodeRef> {
        self.graph
            .node_indices()
            .map(move |index| (index, self.graph.node_weight(index).unwrap()))
    }

    pub fn dependencies(&self, index: NodeIndex<u32>) -> impl Iterator<Item = NodeRef> {
        self.graph
            .neighbors_directed(index, Direction::Outgoing)
            .map(move |index| (index, self.graph.node_weight(index).unwrap()))
    }
}
