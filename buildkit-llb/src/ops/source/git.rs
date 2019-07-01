use std::collections::HashMap;
use std::sync::Arc;

use buildkit_proto::pb::{self, op::Op, OpMetadata, SourceOp};

use crate::ops::{OperationBuilder, SingleBorrowedOutputOperation, SingleOwnedOutputOperation};
use crate::serialization::{Operation, SerializationResult, SerializedNode};
use crate::utils::{OperationOutput, OutputIdx};

#[derive(Default, Debug)]
pub struct GitSource {
    url: String,
    description: HashMap<String, String>,
    ignore_cache: bool,
}

impl GitSource {
    pub(crate) fn new<S>(url: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            url: url.into(),
            description: Default::default(),
            ignore_cache: false,
        }
    }
}

impl<'a> SingleBorrowedOutputOperation<'a> for GitSource {
    fn output(&'a self) -> OperationOutput<'a> {
        OperationOutput::Borrowed(self, OutputIdx(0))
    }
}

impl<'a> SingleOwnedOutputOperation<'static> for Arc<GitSource> {
    fn output(&self) -> OperationOutput<'static> {
        OperationOutput::Owned(self.clone(), OutputIdx(0))
    }
}

impl OperationBuilder<'static> for GitSource {
    fn custom_name<S>(mut self, name: S) -> Self
    where
        S: Into<String>,
    {
        self.description
            .insert("llb.customname".into(), name.into());

        self
    }

    fn ignore_cache(mut self, ignore: bool) -> Self {
        self.ignore_cache = ignore;
        self
    }
}

impl Operation for GitSource {
    fn serialize_head(&self) -> SerializationResult<SerializedNode> {
        let head = pb::Op {
            op: Some(Op::Source(SourceOp {
                identifier: format!("git://{}", self.url),
                attrs: Default::default(),
            })),

            ..Default::default()
        };

        let metadata = OpMetadata {
            description: self.description.clone(),
            ignore_cache: self.ignore_cache,

            ..Default::default()
        };

        Ok(SerializedNode::new(head, metadata))
    }

    fn serialize_tail(&self) -> SerializationResult<Vec<SerializedNode>> {
        Ok(Vec::with_capacity(0))
    }
}
