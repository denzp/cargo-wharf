use std::sync::Arc;

use crate::serialization::Operation;

#[derive(Copy, Clone, Debug)]
pub struct OutputIdx(pub u32);

#[derive(Copy, Clone, Debug)]
pub struct OwnOutputIdx(pub u32);

#[derive(Debug, Clone)]
pub struct OperationOutput<'a> {
    kind: OperationOutputKind<'a>,
}

#[derive(Debug, Clone)]
enum OperationOutputKind<'a> {
    Owned(Arc<dyn Operation + 'a>, OutputIdx),
    Borrowed(&'a dyn Operation, OutputIdx),
}

impl<'a> OperationOutput<'a> {
    pub(crate) fn owned(op: Arc<dyn Operation + 'a>, idx: OutputIdx) -> Self {
        Self {
            kind: OperationOutputKind::Owned(op, idx),
        }
    }

    pub(crate) fn borrowed(op: &'a dyn Operation, idx: OutputIdx) -> Self {
        Self {
            kind: OperationOutputKind::Borrowed(op, idx),
        }
    }

    pub(crate) fn operation(&self) -> &dyn Operation {
        match self.kind {
            OperationOutputKind::Owned(ref op, ..) => op.as_ref(),
            OperationOutputKind::Borrowed(ref op, ..) => *op,
        }
    }

    pub(crate) fn output(&self) -> OutputIdx {
        match self.kind {
            OperationOutputKind::Owned(_, output) | OperationOutputKind::Borrowed(_, output) => {
                output
            }
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
