use crate::serialization::Operation;

#[derive(Copy, Clone, Debug)]
pub struct OutputIndex(pub u32);

#[derive(Copy, Clone, Debug)]
pub struct OwnOutputIndex(pub u32);

#[derive(Copy, Clone, Debug)]
pub struct OperationOutput<'a>(pub(crate) &'a Operation, pub(crate) OutputIndex);

impl Into<i64> for OutputIndex {
    fn into(self) -> i64 {
        self.0.into()
    }
}
impl Into<i64> for &OutputIndex {
    fn into(self) -> i64 {
        self.0.into()
    }
}

impl Into<i64> for OwnOutputIndex {
    fn into(self) -> i64 {
        self.0.into()
    }
}
impl Into<i64> for &OwnOutputIndex {
    fn into(self) -> i64 {
        self.0.into()
    }
}
