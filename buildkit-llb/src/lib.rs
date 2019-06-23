#![feature(existential_type)]
#![cfg_attr(feature = "frontend", feature(async_await))]
#![deny(warnings)]
#![deny(clippy::all)]

// FIXME: get rid of the unwraps

mod serialization;

/// Supported operations - building blocks of the LLB definition graph.
pub mod ops;

/// Various helpers and types.
pub mod utils;

#[cfg(feature = "frontend")]
pub mod frontend;

/// Convenient re-export of a commonly used things.
pub mod prelude {
    pub use crate::ops::*;
    pub use crate::utils::{OutputIdx, OwnOutputIdx};
}
