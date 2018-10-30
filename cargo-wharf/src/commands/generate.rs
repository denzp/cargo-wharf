use std::fs::File;
use std::io::{stdin, stdout, Write};

use cargo::util::CargoResult;
use clap::{App, Arg, ArgMatches, SubCommand};
use either::Either;
use failure::Error;

use crate::config::Config;
use crate::engine::{DockerfilePrinter, OutputMode};
use crate::graph::BuildGraph;
use crate::plan::invocations_from_reader;

#[derive(Default)]
pub struct GenerateCommand;

impl super::SubCommand for GenerateCommand {
    fn api() -> App<'static, 'static> {
        SubCommand::with_name("generate")
            .about("Generates a Dockerfile for a crate")
            .aliases(&["gen"])
            .args(&[
                {
                    Arg::with_name("build_plan")
                        .takes_value(true)
                        .value_name("INPUT")
                        .default_value("stdin")
                        .help("Input build plan location")
                },
                {
                    Arg::with_name("output")
                        .long("output")
                        .short("o")
                        .takes_value(true)
                        .value_name("PATH")
                        .help("Output Dockerfile to a file")
                },
                {
                    Arg::with_name("template")
                        .short("t")
                        .long("template")
                        .takes_value(true)
                        .value_name("PATH")
                        .default_value("Dockerfile.hbs")
                        .help("Dockerfile template location")
                },
                {
                    Arg::with_name("dump_graph")
                        .long("dump-graph")
                        .help("Only dump build graph")
                },
            ])
    }

    fn run(config: &Config, matches: &ArgMatches<'static>) -> CargoResult<()> {
        let input = match matches.value_of("build_plan") {
            None | Some("stdin") | Some("-") => Either::Left(stdin()),
            Some(path) => Either::Right(File::open(path)?),
        };

        let mut output = match matches.value_of("output") {
            None | Some("stdout") | Some("-") => Either::Left(stdout()),
            Some(path) => Either::Right(File::create(path)?),
        };

        let invocations = invocations_from_reader(input)?;
        let graph = BuildGraph::from_invocations(&invocations, &config)?;

        if matches.is_present("dump_graph") {
            return writeln!(output, "{:?}", graph).map_err(Error::from);
        }

        DockerfilePrinter::new(OutputMode::All, &graph, output)
            .write(matches.value_of("template").unwrap())
    }
}
