use std::path::{Path, PathBuf};

use crate::utils::{OperationOutput, OwnOutputIndex};

#[derive(Debug)]
pub struct UnsetPath;

#[derive(Debug)]
pub enum Destination<'a, P: AsRef<Path>> {
    Layer(OperationOutput<'a>, P),
    OwnOutput(OwnOutputIndex, P),
    Scratch(P),
}

impl<'a, P: AsRef<Path>> Destination<'a, P> {
    pub fn into_owned(self) -> Destination<'a, PathBuf> {
        use Destination::*;

        match self {
            Layer(input, path) => Layer(input, path.as_ref().into()),
            Scratch(path) => Scratch(path.as_ref().into()),
            OwnOutput(output, path) => OwnOutput(output, path.as_ref().into()),
        }
    }
}
