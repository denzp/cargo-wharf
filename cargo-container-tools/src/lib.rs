#![deny(warnings)]
#![deny(clippy::all)]

mod env;
pub use self::env::RuntimeEnv;

mod output;
pub use self::output::BuildScriptOutput;

pub mod metadata;
pub mod source;
