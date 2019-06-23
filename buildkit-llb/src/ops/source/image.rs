use std::collections::HashMap;

use buildkit_proto::pb::{self, op::Op, OpMetadata, SourceOp};

use crate::ops::OperationBuilder;
use crate::serialization::{Operation, Output, SerializedNode};
use crate::utils::{OperationOutput, OutputIdx};

#[derive(Default, Debug)]
pub struct ImageSource {
    name: String,
    description: HashMap<String, String>,
    ignore_cache: bool,
}

impl ImageSource {
    pub(crate) fn new<S>(name: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            name: name.into(),
            description: Default::default(),
            ignore_cache: false,
        }
    }

    pub fn output(&self) -> OperationOutput {
        OperationOutput(self, OutputIdx(0))
    }
}

impl OperationBuilder for ImageSource {
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

impl Operation for ImageSource {
    fn serialize(&self) -> Result<Output, ()> {
        let head = pb::Op {
            op: Some(Op::Source(SourceOp {
                identifier: format!("docker-image://docker.io/{}", self.name),
                attrs: Default::default(),
            })),

            ..Default::default()
        };

        let metadata = OpMetadata {
            description: self.description.clone(),
            ignore_cache: self.ignore_cache,

            ..Default::default()
        };

        Ok(Output {
            head: SerializedNode::new(head, metadata),
            tail: vec![],
        })
    }
}
