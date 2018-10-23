use cargo::core::Workspace;
use cargo::util::Config as CargoConfig;

mod config;
mod graph;
mod path;
mod plan;
mod printer;

use crate::config::Config;
use crate::graph::BuildGraph;
use crate::plan::invocations_from_stdio;
use crate::printer::DockerPrinter;

fn main() {
    let cargo_config = CargoConfig::default().unwrap();
    let cargo_ws = Workspace::new(&cargo_config.cwd().join("Cargo.toml"), &cargo_config).unwrap();

    let config = Config::from_cargo_workspace(&cargo_ws).unwrap();

    let invocations = invocations_from_stdio().unwrap();
    let graph = BuildGraph::from_invocations(&invocations, &config).unwrap();

    DockerPrinter::default().print(&graph);

    // TODO(denzp): provide the dump via CLI later
    // println!("{:?}", graph);
}
