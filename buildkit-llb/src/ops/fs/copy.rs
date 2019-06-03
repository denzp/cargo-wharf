use std::collections::HashMap;
use std::fmt::Debug;
use std::path::{Path, PathBuf};

use buildkit_proto::pb;
use either::Either;

use super::path::{Destination, UnsetPath};
use super::FileOperation;

use crate::serialization::SerializedNode;
use crate::utils::{OperationOutput, OutputIndex};

#[derive(Debug)]
pub struct CopyOperation<From: Debug, To: Debug> {
    source: From,
    destination: To,

    follow_symlinks: bool,
    recursive: bool,
    create_path: bool,
    wildcard: bool,

    description: HashMap<String, String>,
    caps: HashMap<String, bool>,
}

type OpWithoutSource = CopyOperation<UnsetPath, UnsetPath>;
type OpWithSource<'a> = CopyOperation<(OperationOutput<'a>, PathBuf), UnsetPath>;
type OpWithDestination<'a> =
    CopyOperation<(OperationOutput<'a>, PathBuf), (OutputIndex, Destination<'a, PathBuf>)>;

impl OpWithoutSource {
    pub(crate) fn new() -> OpWithoutSource {
        let mut caps = HashMap::<String, bool>::new();
        caps.insert("file.base".into(), true);

        CopyOperation {
            source: UnsetPath,
            destination: UnsetPath,

            follow_symlinks: false,
            recursive: false,
            create_path: false,
            wildcard: false,

            caps,
            description: Default::default(),
        }
    }

    pub fn from<'a, P>(self, source: OperationOutput<'a>, path: P) -> OpWithSource<'a>
    where
        P: Into<PathBuf>,
    {
        CopyOperation {
            source: (source, path.into()),
            destination: UnsetPath,

            follow_symlinks: self.follow_symlinks,
            recursive: self.recursive,
            create_path: self.create_path,
            wildcard: self.wildcard,

            description: self.description,
            caps: self.caps,
        }
    }
}

impl<'a> OpWithSource<'a> {
    pub fn to<P>(
        self,
        output: OutputIndex,
        destination: Destination<'a, P>,
    ) -> OpWithDestination<'a>
    where
        P: AsRef<Path>,
    {
        CopyOperation {
            source: self.source,
            destination: (output, destination.into_owned()),

            follow_symlinks: self.follow_symlinks,
            recursive: self.recursive,
            create_path: self.create_path,
            wildcard: self.wildcard,

            description: self.description,
            caps: self.caps,
        }
    }
}

impl<From, To> CopyOperation<From, To>
where
    From: Debug,
    To: Debug,
{
    pub fn follow_symlinks(mut self, value: bool) -> Self {
        self.follow_symlinks = value;
        self
    }

    pub fn recursive(mut self, value: bool) -> Self {
        self.recursive = value;
        self
    }

    pub fn create_path(mut self, value: bool) -> Self {
        self.create_path = value;
        self
    }

    pub fn wildcard(mut self, value: bool) -> Self {
        self.wildcard = value;
        self
    }
}

impl<'a> FileOperation for OpWithDestination<'a> {
    fn output(&self) -> i64 {
        self.destination.0.into()
    }

    fn serialize_inputs(&self) -> Result<(Vec<pb::Input>, Vec<SerializedNode>), ()> {
        let serialized_from = (self.source.0).0.serialize()?;

        let mut inputs = vec![pb::Input {
            digest: serialized_from.head.digest.clone(),
            index: (self.source.0).1.into(),
        }];

        let tail = if let Destination::Layer(ref op, ..) = self.destination.1 {
            let serialized_to = op.0.serialize()?;

            inputs.push(pb::Input {
                digest: serialized_to.head.digest.clone(),
                index: op.1.into(),
            });

            Either::Left(serialized_from.into_iter().chain(serialized_to.into_iter()))
        } else {
            Either::Right(serialized_from.into_iter())
        };

        Ok((inputs, tail.collect()))
    }

    fn serialize_action(
        &self,
        inputs_count: usize,
        inputs_offset: usize,
    ) -> Result<pb::FileAction, ()> {
        let (dest_idx, dest) = match self.destination.1 {
            Destination::Scratch(ref path) => (-1, path.to_string_lossy().into()),

            Destination::Layer(_, ref path) => {
                (inputs_offset as i64 + 1, path.to_string_lossy().into())
            }

            Destination::OwnOutput(ref output, ref path) => {
                let output: i64 = output.into();

                (inputs_count as i64 + output, path.to_string_lossy().into())
            }
        };

        Ok(pb::FileAction {
            input: dest_idx,
            secondary_input: 0,

            output: self.output().into(),

            action: Some(pb::file_action::Action::Copy(pb::FileActionCopy {
                src: self.source.1.to_string_lossy().into(),
                dest,

                follow_symlink: self.follow_symlinks,
                dir_copy_contents: self.recursive,
                create_dest_path: self.create_path,
                allow_wildcard: self.wildcard,

                // TODO: make this configurable
                mode: -1,

                // TODO: make this configurable
                timestamp: -1,

                ..Default::default()
            })),
        })
    }
}
