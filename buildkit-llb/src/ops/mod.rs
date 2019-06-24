pub mod exec;
pub mod fs;
pub mod source;
pub mod terminal;

pub use self::exec::Command;
pub use self::fs::FileSystem;
pub use self::source::Source;
pub use self::terminal::Terminal;

/// Common operation methods.
pub trait OperationBuilder {
    /// Sets an operation display name.
    fn custom_name<S>(self, name: S) -> Self
    where
        S: Into<String>;

    /// Sets caching behavior.
    fn ignore_cache(self, ignore: bool) -> Self;
}
