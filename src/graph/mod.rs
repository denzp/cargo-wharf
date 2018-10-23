use std::fmt;

use cargo::util::CargoResult;

use petgraph::dot;
use petgraph::graph::NodeIndex;
use petgraph::{Direction, Graph};

use crate::config::Config;
use crate::plan::Invocation;

mod node;
pub use self::node::Node;

mod command;
pub use self::command::CommandDetails;

mod source;
pub use self::source::SourceKind;

pub struct BuildGraph {
    graph: Graph<Node, usize>,
}

impl BuildGraph {
    pub fn from_invocations(invocations: &[Invocation], config: &Config) -> CargoResult<Self> {
        let mut graph = Graph::<Node, usize>::new();

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

        Ok(Self { graph })
    }

    pub fn nodes(&self) -> impl Iterator<Item = (NodeIndex<u32>, &Node)> {
        self.graph
            .node_indices()
            .map(move |index| (index, self.graph.node_weight(index).unwrap()))
    }

    pub fn dependencies(&self, index: NodeIndex<u32>) -> impl Iterator<Item = &Node> {
        self.graph
            .neighbors_directed(index, Direction::Outgoing)
            .map(move |index| self.graph.node_weight(index).unwrap())
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
