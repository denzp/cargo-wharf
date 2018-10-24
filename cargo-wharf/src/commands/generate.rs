use std::fs::File;
use std::io::stdin;

use cargo::util::CargoResult;
use clap::{App, Arg, ArgMatches, SubCommand};

use crate::config::Config;
use crate::engine::DockerfilePrinter;
use crate::graph::BuildGraph;
use crate::plan::invocations_from_reader;

#[derive(Default)]
pub struct GenerateCommand;

impl super::SubCommand for GenerateCommand {
    fn api() -> App<'static, 'static> {
        SubCommand::with_name("generate")
            .about("Generates a Dockerfile for a crate")
            .args(&[
                {
                    Arg::with_name("build_plan")
                        .takes_value(true)
                        .value_name("INPUT")
                        .default_value("stdin")
                        .help("Input build plan location")
                },
                // {
                //     Arg::with_name("output")
                //         .long("output")
                //         .short("o")
                //         .takes_value(true)
                //         .value_name("PATH")
                //         .help("Output Dockerfile to a file")
                // },
                // {
                //     Arg::with_name("template")
                //         .short("f")
                //         .long("template")
                //         .takes_value(true)
                //         .value_name("PATH")
                //         .default_value("Dockerfile.template") // TODO(denzp): change extension after a template engine decision
                //         .help("Dockerfile template location")
                // },
                {
                    Arg::with_name("dump_graph")
                        .long("dump-graph")
                        .help("Only dump build graph to stdout")
                },
            ])
    }

    fn run(config: &Config, matches: &ArgMatches<'static>) -> CargoResult<()> {
        let invocations = match matches.value_of("build_plan") {
            None | Some("stdin") | Some("-") => invocations_from_reader(stdin())?,
            Some(path) => invocations_from_reader(File::open(path)?)?,
        };

        let graph = BuildGraph::from_invocations(&invocations, &config)?;

        if matches.is_present("dump_graph") {
            println!("{:?}", graph);
            return Ok(());
        }

        DockerfilePrinter::default().print(&graph);
        Ok(())
    }
}
