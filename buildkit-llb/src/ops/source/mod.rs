use std::collections::HashMap;

use buildkit_proto::pb::{self, op::Op, OpMetadata, SourceOp};

use super::OperationBuilder;
use crate::serialization::{Operation, Output, SerializedNode};
use crate::utils::{OperationOutput, OutputIndex};

#[derive(Debug)]
enum SourceKind {
    DockerImage(String),
    GitRepo(String),
}

#[derive(Debug)]
pub struct Source {
    kind: SourceKind,
    description: HashMap<String, String>,
}

impl Source {
    pub fn image<S>(name: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            kind: SourceKind::DockerImage(name.into()),
            description: Default::default(),
        }
    }

    pub fn git<S>(url: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            kind: SourceKind::GitRepo(url.into()),
            description: Default::default(),
        }
    }

    pub fn output(&self) -> OperationOutput {
        OperationOutput(self, OutputIndex(0))
    }
}

impl OperationBuilder for Source {
    fn custom_name<S>(mut self, name: S) -> Self
    where
        S: Into<String>,
    {
        self.description
            .insert("llb.customname".into(), name.into());

        self
    }
}

impl Operation for Source {
    fn serialize(&self) -> Result<Output, ()> {
        let head = pb::Op {
            op: Some(Op::Source(match self.kind {
                SourceKind::DockerImage(ref name) => SourceOp {
                    identifier: format!("docker-image://docker.io/{}", name),
                    attrs: Default::default(),
                },

                SourceKind::GitRepo(ref url) => SourceOp {
                    identifier: format!("git://{}", url),
                    attrs: Default::default(),
                },
            })),

            ..Default::default()
        };

        let metadata = OpMetadata {
            description: self.description.clone(),

            ..Default::default()
        };

        Ok(Output {
            head: SerializedNode::new(head, metadata),
            tail: vec![],
        })
    }
}
