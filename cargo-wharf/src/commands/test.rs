use cargo::util::CargoResult;
use clap::{App, ArgMatches, SubCommand};

#[derive(Default)]
pub struct TestCommand;

impl super::SubCommand for TestCommand {
    fn api() -> App<'static, 'static> {
        SubCommand::with_name("test").about("Tests a crate in container")
    }

    fn run(_matches: &ArgMatches<'static>) -> CargoResult<()> {
        eprintln!("TBD: test");

        Ok(())
    }
}
