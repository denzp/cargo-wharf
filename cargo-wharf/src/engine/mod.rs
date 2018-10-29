pub mod docker;
pub use self::docker::DockerfilePrinter;

mod utils;

pub enum OutputMode {
    All,
    Binaries,
    Tests,
}
