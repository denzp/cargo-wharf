mod exec;
mod fs;
mod source;
mod terminal;

pub use self::exec::{Command, Mount};
pub use self::fs::{FileSystem, LayerPath};
pub use self::source::Source;
pub use self::terminal::Terminal;

/// Common operation methods.
pub trait OperationBuilder {
    /// Sets an operation display name.
    fn custom_name<S>(self, name: S) -> Self
    where
        S: Into<String>;
}
