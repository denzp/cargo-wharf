use std::io::Read;

use cargo::CargoResult;
use clap::{App, ArgMatches};
use failure::ResultExt;

use crate::config::Config;
use crate::graph::BuildGraph;
use crate::plan::invocations_from_reader;

mod generate;
pub use self::generate::GenerateCommand;

mod build;
pub use self::build::BuildCommand;

mod test;
pub use self::test::TestCommand;

pub trait SubCommand {
    fn api() -> App<'static, 'static>;

    fn run(config: &Config, matches: &ArgMatches<'static>) -> CargoResult<()>;
}

fn construct_build_graph(config: &Config, input: impl Read) -> CargoResult<BuildGraph> {
    let graph_result = BuildGraph::from_invocations(
        &invocations_from_reader(input).context("Unable to parse Cargo build plan")?,
        &config,
    );

    Ok(graph_result.context("Unable to construct Build Graph")?)
}
