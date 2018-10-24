use cargo::util::CargoResult;
use clap::{App, ArgMatches, SubCommand};

use crate::config::Config;

#[derive(Default)]
pub struct TestCommand;

impl super::SubCommand for TestCommand {
    fn api() -> App<'static, 'static> {
        SubCommand::with_name("test").about("Tests a crate in container")
    }

    fn run(_config: &Config, _matches: &ArgMatches<'static>) -> CargoResult<()> {
        println!("TBD");

        Ok(())
    }
}
