use std::sync::Arc;

use crate::serialization::Operation;

#[derive(Copy, Clone, Debug)]
pub struct OutputIdx(pub u32);

#[derive(Copy, Clone, Debug)]
pub struct OwnOutputIdx(pub u32);

#[derive(Debug, Clone)]
pub enum OperationOutput<'a> {
    Owned(Arc<dyn Operation + 'a>, OutputIdx),
    Borrowed(&'a dyn Operation, OutputIdx),
}

impl<'a> OperationOutput<'a> {
    pub fn operation(&self) -> &dyn Operation {
        match self {
            OperationOutput::Owned(op, ..) => op.as_ref(),
            OperationOutput::Borrowed(op, ..) => *op,
        }
    }

    pub fn output(&self) -> OutputIdx {
        match self {
            OperationOutput::Owned(_, output) | OperationOutput::Borrowed(_, output) => *output,
        }
    }
}

impl Into<i64> for OutputIdx {
    fn into(self) -> i64 {
        self.0.into()
    }
}
impl Into<i64> for &OutputIdx {
    fn into(self) -> i64 {
        self.0.into()
    }
}

impl Into<i64> for OwnOutputIdx {
    fn into(self) -> i64 {
        self.0.into()
    }
}
impl Into<i64> for &OwnOutputIdx {
    fn into(self) -> i64 {
        self.0.into()
    }
}
