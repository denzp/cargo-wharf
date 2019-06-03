use std::collections::HashMap;

use buildkit_proto::pb::{self, op::Op};

use super::FileOperation;

use crate::ops::OperationBuilder;
use crate::serialization::{Operation, Output, SerializedNode};
use crate::utils::{OperationOutput, OutputIdx};

#[derive(Debug)]
pub struct SequenceOperation<'a> {
    inner: Vec<Box<FileOperation + 'a>>,

    description: HashMap<String, String>,
    caps: HashMap<String, bool>,
}

impl<'a> SequenceOperation<'a> {
    pub(crate) fn new() -> Self {
        let mut caps = HashMap::<String, bool>::new();
        caps.insert("file.base".into(), true);

        Self {
            inner: vec![],

            caps,
            description: Default::default(),
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

    pub fn output(&self, index: u32) -> OperationOutput {
        // TODO: check if the requested index available.

        OperationOutput(self, OutputIdx(index))
    }
}

impl<'a> OperationBuilder for SequenceOperation<'a> {
    fn custom_name<S>(mut self, name: S) -> Self
    where
        S: Into<String>,
    {
        self.description
            .insert("llb.customname".into(), name.into());

        self
    }
}

impl<'a> Operation for SequenceOperation<'a> {
    fn serialize(&self) -> Result<Output, ()> {
        let mut inputs = vec![];
        let mut input_offsets = vec![];
        let mut tail = vec![];

        for item in &self.inner {
            let (mut inner_inputs, mut inner_tail) = item.serialize_inputs()?;

            input_offsets.push(inputs.len());
            inputs.append(&mut inner_inputs);
            tail.append(&mut inner_tail);
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

            ..Default::default()
        };

        Ok(Output {
            head: SerializedNode::new(head, metadata),
            tail,
        })
    }
}
