use std::fmt;

use cargo::util::CargoResult;

use petgraph::dot;
use petgraph::prelude::*;

use crate::config::Config;
use crate::plan::Invocation;

mod node;
pub use self::node::{Node, NodeKind};

mod command;
pub use self::command::{Command, CommandDetails};

mod source;
pub use self::source::SourceKind;

mod ops;
use self::ops::merge_build_script_nodes;

#[allow(dead_code)]
type NodeRef<'a> = (NodeIndex<u32>, &'a Node);

pub struct BuildGraph {
    graph: StableGraph<Node, usize>,
}

impl BuildGraph {
    pub fn from_invocations(invocations: &[Invocation], config: &Config) -> CargoResult<Self> {
        let mut graph = StableGraph::<Node, usize>::new();

        let nodes = {
            invocations
                .iter()
                .map(|item| Ok(graph.add_node(Node::from_invocation(item, config)?)))
                .collect::<CargoResult<Vec<_>>>()?
        };

        for (item, index) in invocations.iter().zip(0..) {
            let mut deps = item.deps.clone();

            deps.sort();
            for dep in deps.iter() {
                graph.add_edge(nodes[index], nodes[*dep as usize], 0);
            }
        }

        merge_build_script_nodes(&mut graph);

        Ok(Self { graph })
    }

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

impl fmt::Debug for BuildGraph {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "{}",
            dot::Dot::with_config(&self.graph, &[dot::Config::EdgeNoLabel])
        )
    }
}
