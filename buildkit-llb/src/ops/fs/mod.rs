use std::collections::HashMap;
use std::path::PathBuf;

use buildkit_proto::pb::{
    self, file_action::Action, op::Op, FileAction, FileActionCopy, FileOp, Input, OpMetadata,
};

use crate::serialization::{Operation, Output, SerializedNode};
use crate::utils::{OperationOutput, OutputIndex};

use super::OperationBuilder;

#[derive(Debug)]
enum OpKind<'a> {
    Copy {
        from: (OperationOutput<'a>, PathBuf),
        to: (OperationOutput<'a>, PathBuf),
    },
}

#[derive(Debug)]
pub struct FileSystem<'a> {
    kind: OpKind<'a>,
    description: HashMap<String, String>,
    caps: HashMap<String, bool>,
}

impl<'a> FileSystem<'a> {
    pub fn copy<P, Q>(
        from: OperationOutput<'a>,
        from_path: P,
        to: OperationOutput<'a>,
        to_path: Q,
    ) -> Self
    where
        P: Into<PathBuf>,
        Q: Into<PathBuf>,
    {
        let mut caps = HashMap::<String, bool>::new();
        caps.insert("file.base".into(), true);

        Self {
            kind: OpKind::Copy {
                from: (from, from_path.into()),
                to: (to, to_path.into()),
            },

            caps,
            description: Default::default(),
        }
    }

    pub fn output(&self) -> OperationOutput {
        OperationOutput(self, OutputIndex(0))
    }
}

impl<'a> OperationBuilder for FileSystem<'a> {
    fn custom_name<S>(mut self, name: S) -> Self
    where
        S: Into<String>,
    {
        self.description
            .insert("llb.customname".into(), name.into());

        self
    }
}

impl<'a> Operation for FileSystem<'a> {
    fn serialize(&self) -> Result<Output, ()> {
        let (inputs, tails) = match self.kind {
            OpKind::Copy { ref from, ref to } => {
                let serialized_from = (from.0).0.serialize()?;
                let serialized_to = (to.0).0.serialize()?;

                let inputs = vec![
                    Input {
                        digest: serialized_from.head.digest.clone(),
                        index: (from.0).1.into(),
                    },
                    Input {
                        digest: serialized_to.head.digest.clone(),
                        index: (to.0).1.into(),
                    },
                ];

                (
                    inputs,
                    serialized_from
                        .into_iter()
                        .chain(serialized_to.into_iter())
                        .collect(),
                )
            }
        };

        let (src, dest) = match self.kind {
            OpKind::Copy { ref from, ref to } => (
                from.1.to_string_lossy().into(),
                to.1.to_string_lossy().into(),
            ),
        };

        let head = pb::Op {
            inputs,

            op: Some(Op::File(FileOp {
                // TODO: support multiple actions
                actions: vec![FileAction {
                    input: 1,
                    secondary_input: 0,

                    // TODO: support specifying the output
                    output: 0,

                    action: Some(Action::Copy(FileActionCopy {
                        src,
                        dest,

                        mode: -1,
                        timestamp: -1,

                        follow_symlink: true,
                        dir_copy_contents: true,
                        create_dest_path: true,
                        allow_wildcard: true,

                        ..Default::default()
                    })),
                }],
            })),

            ..Default::default()
        };

        let metadata = OpMetadata {
            description: self.description.clone(),
            caps: self.caps.clone(),

            ..Default::default()
        };

        Ok(Output {
            head: SerializedNode::new(head, metadata),
            tail: tails,
        })
    }
}
