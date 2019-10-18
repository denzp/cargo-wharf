use petgraph::prelude::*;
use serde::Serialize;

use crate::plan::RawBuildPlan;

mod node;
mod ops;

pub use self::node::*;

#[derive(Debug, Serialize)]
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
        let mut graph = StableGraph::from(plan);

        self::ops::merge_buildscript_nodes(&mut graph);
        self::ops::apply_buildscript_outputs(&mut graph);

        Self { graph }
    }
}

impl From<RawBuildPlan> for StableGraph<Node, ()> {
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

        graph
    }
}
