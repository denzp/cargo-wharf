use cargo::util::CargoResult;
use clap::{App, ArgMatches, SubCommand};

use crate::config::Config;

#[derive(Default)]
pub struct BuildCommand;

impl super::SubCommand for BuildCommand {
    fn api() -> App<'static, 'static> {
        SubCommand::with_name("build").about("Creates an image for a crate")
    }

    fn run(_config: &Config, _matches: &ArgMatches<'static>) -> CargoResult<()> {
        println!("TBD");

        Ok(())
    }
}
