use std::fs::File;
use std::io::{stdin, stdout, Read, Write};

use cargo::util::CargoResult;
use clap::{App, Arg, ArgMatches, SubCommand};
use either::Either;
use failure::{Error, ResultExt};

use crate::config::Config;
use crate::engine::{DockerfilePrinter, OutputMode};

use super::construct_build_graph;

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
                        .value_name("OUTPUT")
                        .default_value("stdout")
                        .help("Output Dockerfile to a file")
                },
                {
                    Arg::with_name("template")
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
        let input = open_input(matches.value_of("build_plan"))?;
        let mut output = open_output(matches.value_of("output"))?;
        let graph = construct_build_graph(config, input)?;

        if matches.is_present("dump_graph") {
            return writeln!(output, "{:?}", graph).map_err(Error::from);
        }

        let dockerfile_template = matches.value_of("template").unwrap();
        let dockerfile = {
            DockerfilePrinter::from_template(OutputMode::All, dockerfile_template)
                .context("Unable to initialize Dockerfile template")?
        };

        dockerfile
            .write(graph, &mut output)
            .context("Unable to generate Dockerfile")?;

        Ok(())
    }
}

fn open_input(path: Option<&str>) -> CargoResult<impl Read> {
    match path {
        None | Some("stdin") | Some("-") => Ok(Either::Left(stdin())),

        Some(path) => {
            let file = {
                File::open(path).with_context(|_| {
                    format!("Unable to open Cargo build plan input file '{}'", path)
                })?
            };

            Ok(Either::Right(file))
        }
    }
}

fn open_output(path: Option<&str>) -> CargoResult<impl Write> {
    match path {
        None | Some("stdout") | Some("-") => Ok(Either::Left(stdout())),

        Some(path) => {
            let file = {
                File::create(path).with_context(|_| {
                    format!("Unable to create Dockerfile output file '{}'", path)
                })?
            };

            Ok(Either::Right(file))
        }
    }
}
