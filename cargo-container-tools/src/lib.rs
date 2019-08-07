#![deny(warnings)]
#![deny(clippy::all)]

mod env;
pub use self::env::RuntimeEnv;

mod output;
pub use self::output::BuildScriptOutput;
