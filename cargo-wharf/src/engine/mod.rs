pub mod docker;
pub use self::docker::DockerfilePrinter;

pub enum OutputMode {
    All,
    Binaries,
    Tests,
}
