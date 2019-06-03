mod exec;
mod fs;
mod source;
mod terminal;

pub use self::exec::{Command, Mount};
pub use self::fs::{Destination, FileSystem};
pub use self::source::Source;
pub use self::terminal::Terminal;

pub trait OperationBuilder {
    fn custom_name<S>(self, name: S) -> Self
    where
        S: Into<String>;
}
