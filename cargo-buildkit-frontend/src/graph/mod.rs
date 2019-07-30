use petgraph::prelude::*;

use crate::plan::RawBuildPlan;

mod node;
mod ops;

pub use self::node::*;

#[derive(Debug)]
pub struct BuildGraph {
    graph: StableGraph<Node, ()>,
}

impl BuildGraph {
    pub fn inner(&self) -> &StableGraph<Node, ()> {
        &self.graph
    }
}

impl From<RawBuildPlan> for BuildGraph {
    fn from(plan: RawBuildPlan) -> Self {
        let mut graph = StableGraph::<Node, ()>::new();

        let nodes = {
            plan.invocations
                .iter()
                .map(|item| graph.add_node(item.into()))
                .collect::<Vec<_>>()
        };

        for (item, index) in plan.invocations.iter().zip(0..) {
            let mut deps = item.deps.clone();

            deps.sort();
            for dep in item.deps.iter() {
                graph.add_edge(nodes[*dep as usize], nodes[index], ());
            }
        }

        self::ops::merge_build_script_nodes(&mut graph);

        Self { graph }
    }
}
