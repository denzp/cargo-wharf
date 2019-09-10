use cargo::CargoResult;
use clap::{App, ArgMatches};

mod build;
pub use self::build::BuildCommand;

mod test;
pub use self::test::TestCommand;

pub trait SubCommand {
    fn api() -> App<'static, 'static>;

    fn run(matches: &ArgMatches<'static>) -> CargoResult<()>;
}
