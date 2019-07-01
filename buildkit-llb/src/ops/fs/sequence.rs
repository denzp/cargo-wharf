use std::collections::HashMap;
use std::sync::Arc;

use buildkit_proto::pb::{self, op::Op};

use super::FileOperation;

use crate::ops::{MultiBorrowedOutputOperation, MultiOwnedOutputOperation, OperationBuilder};
use crate::serialization::{Operation, SerializationResult, SerializedNode};
use crate::utils::{OperationOutput, OutputIdx};

#[derive(Debug)]
pub struct SequenceOperation<'a> {
    inner: Vec<Box<dyn FileOperation + 'a>>,

    description: HashMap<String, String>,
    caps: HashMap<String, bool>,
    ignore_cache: bool,
}

impl<'a> SequenceOperation<'a> {
    pub(crate) fn new() -> Self {
        let mut caps = HashMap::<String, bool>::new();
        caps.insert("file.base".into(), true);

        Self {
            inner: vec![],

            caps,
            description: Default::default(),
            ignore_cache: false,
        }
    }

    pub fn append<T>(mut self, op: T) -> Self
    where
        T: FileOperation + 'a,
    {
        // TODO: verify no duplicated outputs

        self.inner.push(Box::new(op));
        self
    }
}

impl<'a, 'b: 'a> MultiBorrowedOutputOperation<'b> for SequenceOperation<'b> {
    fn output(&'b self, index: u32) -> OperationOutput<'b> {
        // TODO: check if the requested index available.
        OperationOutput::Borrowed(self, OutputIdx(index))
    }
}

impl<'a> MultiOwnedOutputOperation<'a> for Arc<SequenceOperation<'a>> {
    fn output(&self, index: u32) -> OperationOutput<'a> {
        // TODO: check if the requested index available.
        OperationOutput::Owned(self.clone(), OutputIdx(index))
    }
}

impl<'a> OperationBuilder<'a> for SequenceOperation<'a> {
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

impl<'a> Operation for SequenceOperation<'a> {
    fn serialize_tail(&self) -> SerializationResult<Vec<SerializedNode>> {
        let tail = {
            self.inner
                .iter()
                .map(|op| op.serialize_tail().unwrap().into_iter())
                .flatten()
                .collect()
        };

        Ok(tail)
    }

    fn serialize_head(&self) -> SerializationResult<SerializedNode> {
        let mut inputs = vec![];
        let mut input_offsets = vec![];

        for item in &self.inner {
            let mut inner_inputs = item.serialize_inputs()?;

            input_offsets.push(inputs.len());
            inputs.append(&mut inner_inputs);
        }

        let mut actions = vec![];

        for (item, offset) in self.inner.iter().zip(input_offsets.into_iter()) {
            actions.push(item.serialize_action(inputs.len(), offset)?);
        }

        let head = pb::Op {
            inputs,
            op: Some(Op::File(pb::FileOp { actions })),

            ..Default::default()
        };

        let metadata = pb::OpMetadata {
            description: self.description.clone(),
            caps: self.caps.clone(),
            ignore_cache: self.ignore_cache,

            ..Default::default()
        };

        Ok(SerializedNode::new(head, metadata))
    }
}
