#![feature(existential_type)]
#![deny(warnings)]
#![deny(clippy::all)]

// FIXME: get rid of the unwraps

mod serialization;

pub mod ops;
pub mod utils;

pub mod prelude {
    pub use crate::ops::*;
    pub use crate::utils::{OutputIndex, OwnOutputIndex};
}
