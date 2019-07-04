use std::fmt::Debug;

use super::{Context, OperationId};
use super::{Node, Output, Result};

pub(crate) trait Operation: Debug + Send + Sync {
    fn id(&self) -> &OperationId;

    fn serialize_head(&self, cx: &mut Context) -> Result<Node>;
    fn serialize_tail(&self, cx: &mut Context) -> Result<Vec<Node>>;

    fn serialize_head_cached(&self, cx: &mut Context) -> Result<Node> {
        cx.reuse(self.id(), |cx| self.serialize_head(cx))
    }

    fn serialize(&self, cx: &mut Context) -> Result<Output> {
        cx.enter(self.id(), |cx| {
            Ok(Output {
                head: self.serialize_head(cx)?,
                tail: self.serialize_tail(cx)?,
            })
        })
    }
}
