use cargo::util::CargoResult;
use clap::{App, Arg, ArgMatches, SubCommand};

#[derive(Default)]
pub struct BuildCommand;

impl super::SubCommand for BuildCommand {
    fn api() -> App<'static, 'static> {
        SubCommand::with_name("build")
            .about("Creates a Docker image for the crate")
            .arg(
                Arg::with_name("tag")
                    .short("t")
                    .long("tag")
                    .takes_value(true)
                    .value_name("NAME")
                    .multiple(true)
                    .required(true)
                    .number_of_values(1)
                    .help("Output image tag"),
            )
    }

    fn run(_matches: &ArgMatches<'static>) -> CargoResult<()> {
        eprintln!("TBD: build");

        Ok(())
    }
}
