use crate::serialization::Operation;

#[derive(Copy, Clone, Debug)]
pub struct OutputIdx(pub u32);

#[derive(Copy, Clone, Debug)]
pub struct OwnOutputIdx(pub u32);

#[derive(Copy, Clone, Debug)]
pub struct OperationOutput<'a>(pub(crate) &'a Operation, pub(crate) OutputIdx);

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
