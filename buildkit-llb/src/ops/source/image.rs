use std::collections::HashMap;
use std::sync::Arc;

use buildkit_proto::pb::{self, op::Op, OpMetadata, SourceOp};

use crate::ops::{OperationBuilder, SingleBorrowedOutput, SingleOwnedOutput};
use crate::serialization::{Context, Node, Operation, OperationId, Result};
use crate::utils::{OperationOutput, OutputIdx};

#[derive(Default, Debug)]
pub struct ImageSource {
    id: OperationId,
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
            id: OperationId::default(),
            name: name.into(),
            description: Default::default(),
            ignore_cache: false,
        }
    }
}

impl<'a> SingleBorrowedOutput<'a> for ImageSource {
    fn output(&'a self) -> OperationOutput<'a> {
        OperationOutput::borrowed(self, OutputIdx(0))
    }
}

impl<'a> SingleOwnedOutput<'static> for Arc<ImageSource> {
    fn output(&self) -> OperationOutput<'static> {
        OperationOutput::owned(self.clone(), OutputIdx(0))
    }
}

impl OperationBuilder<'static> for ImageSource {
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
    fn id(&self) -> &OperationId {
        &self.id
    }

    fn serialize(&self, _: &mut Context) -> Result<Node> {
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

        Ok(Node::new(head, metadata))
    }
}

#[test]
fn serialization() {
    crate::check_op!(
        ImageSource::new("rustlang/rust:nightly"),
        |digest| { "sha256:dee2a3d7dd482dd8098ba543ff1dcb01efd29fcd16fdb0979ef556f38564543a" },
        |description| { vec![] },
        |caps| { vec![] },
        |cached_tail| { vec![] },
        |inputs| { vec![] },
        |op| {
            Op::Source(SourceOp {
                identifier: "docker-image://docker.io/rustlang/rust:nightly".into(),
                attrs: Default::default(),
            })
        },
    );

    crate::check_op!(
        ImageSource::new("library/alpine:latest"),
        |digest| { "sha256:0e6b31ceed3e6dc542018f35a53a0e857e6a188453d32a2a5bbe7aa2971c1220" },
        |description| { vec![] },
        |caps| { vec![] },
        |cached_tail| { vec![] },
        |inputs| { vec![] },
        |op| {
            Op::Source(SourceOp {
                identifier: "docker-image://docker.io/library/alpine:latest".into(),
                attrs: Default::default(),
            })
        },
    );

    crate::check_op!(
        ImageSource::new("rustlang/rust:nightly").custom_name("image custom name"),
        |digest| { "sha256:dee2a3d7dd482dd8098ba543ff1dcb01efd29fcd16fdb0979ef556f38564543a" },
        |description| { vec![("llb.customname", "image custom name")] },
        |caps| { vec![] },
        |cached_tail| { vec![] },
        |inputs| { vec![] },
        |op| {
            Op::Source(SourceOp {
                identifier: "docker-image://docker.io/rustlang/rust:nightly".into(),
                attrs: Default::default(),
            })
        },
    );
}
