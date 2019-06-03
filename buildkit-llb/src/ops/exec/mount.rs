use std::path::{Path, PathBuf};

use crate::utils::{OperationOutput, OutputIndex};

#[derive(Debug)]
pub enum Mount<'a, P: AsRef<Path>> {
    /// Read-only output of another operation.
    ReadOnlyLayer(OperationOutput<'a>, P),

    /// Read-only output of another operation with a selector.
    ReadOnlySelector(OperationOutput<'a>, P, P),

    /// Empty layer that produces an output.
    Scratch(OutputIndex, P),

    /// Writable output of another operation.
    Layer(OutputIndex, OperationOutput<'a>, P),
}

impl<'a, P: AsRef<Path>> Mount<'a, P> {
    pub fn into_owned(self) -> Mount<'a, PathBuf> {
        use Mount::*;

        match self {
            ReadOnlySelector(op, path, selector) => {
                ReadOnlySelector(op, path.as_ref().into(), selector.as_ref().into())
            }

            ReadOnlyLayer(op, path) => ReadOnlyLayer(op, path.as_ref().into()),
            Scratch(output, path) => Scratch(output, path.as_ref().into()),
            Layer(output, input, path) => Layer(output, input, path.as_ref().into()),
        }
    }
}
