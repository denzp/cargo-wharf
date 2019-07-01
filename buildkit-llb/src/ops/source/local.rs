use std::collections::HashMap;
use std::sync::Arc;

use buildkit_proto::pb::{self, op::Op, OpMetadata, SourceOp};

use crate::ops::{OperationBuilder, SingleBorrowedOutputOperation, SingleOwnedOutputOperation};
use crate::serialization::{Operation, SerializationResult, SerializedNode};
use crate::utils::{OperationOutput, OutputIdx};

#[derive(Default, Debug)]
pub struct LocalSource {
    name: String,
    description: HashMap<String, String>,
    ignore_cache: bool,

    exclude: Vec<String>,
    include: Vec<String>,
}

impl LocalSource {
    pub(crate) fn new<S>(name: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            name: name.into(),
            ignore_cache: false,

            ..Default::default()
        }
    }

    pub fn add_include_pattern<S>(mut self, include: S) -> Self
    where
        S: Into<String>,
    {
        // TODO: add `source.local.includepatterns` capability
        self.include.push(include.into());
        self
    }

    pub fn add_exclude_pattern<S>(mut self, exclude: S) -> Self
    where
        S: Into<String>,
    {
        // TODO: add `source.local.excludepatterns` capability
        self.exclude.push(exclude.into());
        self
    }
}

impl<'a> SingleBorrowedOutputOperation<'a> for LocalSource {
    fn output(&'a self) -> OperationOutput<'a> {
        OperationOutput::Borrowed(self, OutputIdx(0))
    }
}

impl<'a> SingleOwnedOutputOperation<'static> for Arc<LocalSource> {
    fn output(&self) -> OperationOutput<'static> {
        OperationOutput::Owned(self.clone(), OutputIdx(0))
    }
}

impl OperationBuilder<'static> for LocalSource {
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

impl Operation for LocalSource {
    fn serialize_head(&self) -> SerializationResult<SerializedNode> {
        let mut attrs = HashMap::default();

        if !self.exclude.is_empty() {
            attrs.insert(
                "local.excludepatterns".into(),
                serde_json::to_string(&self.exclude).unwrap(),
            );
        }

        if !self.include.is_empty() {
            attrs.insert(
                "local.includepattern".into(),
                serde_json::to_string(&self.include).unwrap(),
            );
        }

        let head = pb::Op {
            op: Some(Op::Source(SourceOp {
                identifier: format!("local://{}", self.name),
                attrs,
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
