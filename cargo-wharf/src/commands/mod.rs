use cargo::CargoResult;
use clap::{App, ArgMatches};

use crate::config::Config;

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
