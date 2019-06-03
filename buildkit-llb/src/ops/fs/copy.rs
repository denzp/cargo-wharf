use std::collections::HashMap;
use std::fmt::Debug;
use std::iter::empty;
use std::path::{Path, PathBuf};

use buildkit_proto::pb;
use either::Either;

use super::path::{LayerPath, UnsetPath};
use super::FileOperation;

use crate::serialization::SerializedNode;
use crate::utils::OutputIdx;

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
type OpWithSource<'a> = CopyOperation<LayerPath<'a, PathBuf>, UnsetPath>;
type OpWithDestination<'a> =
    CopyOperation<LayerPath<'a, PathBuf>, (OutputIdx, LayerPath<'a, PathBuf>)>;

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

    pub fn from<P>(self, source: LayerPath<'_, P>) -> OpWithSource
    where
        P: AsRef<Path>,
    {
        CopyOperation {
            source: source.into_owned(),
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
    pub fn to<P>(self, output: OutputIdx, destination: LayerPath<'a, P>) -> OpWithDestination<'a>
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
        let (mut inputs, from_tail_iter) = if let LayerPath::Other(ref op, ..) = self.source {
            let serialized_from = op.0.serialize()?;

            let inputs = vec![pb::Input {
                digest: serialized_from.head.digest.clone(),
                index: op.1.into(),
            }];

            (inputs, Either::Left(serialized_from.into_iter()))
        } else {
            (vec![], Either::Right(empty()))
        };

        let to_tail_iter = if let LayerPath::Other(ref op, ..) = self.destination.1 {
            let serialized_to = op.0.serialize()?;

            inputs.push(pb::Input {
                digest: serialized_to.head.digest.clone(),
                index: op.1.into(),
            });

            Either::Left(serialized_to.into_iter())
        } else {
            Either::Right(empty())
        };

        Ok((inputs, from_tail_iter.chain(to_tail_iter).collect()))
    }

    fn serialize_action(
        &self,
        inputs_count: usize,
        inputs_offset: usize,
    ) -> Result<pb::FileAction, ()> {
        let (src_idx, src_offset, src) = match self.source {
            LayerPath::Scratch(ref path) => (-1, 0, path.to_string_lossy().into()),

            LayerPath::Other(_, ref path) => {
                (inputs_offset as i64, 1, path.to_string_lossy().into())
            }

            LayerPath::Own(ref output, ref path) => {
                let output: i64 = output.into();

                (
                    inputs_count as i64 + output,
                    0,
                    path.to_string_lossy().into(),
                )
            }
        };

        let (dest_idx, dest) = match self.destination.1 {
            LayerPath::Scratch(ref path) => (-1, path.to_string_lossy().into()),

            LayerPath::Other(_, ref path) => (
                inputs_offset as i64 + src_offset,
                path.to_string_lossy().into(),
            ),

            LayerPath::Own(ref output, ref path) => {
                let output: i64 = output.into();

                (inputs_count as i64 + output, path.to_string_lossy().into())
            }
        };

        Ok(pb::FileAction {
            input: dest_idx,
            secondary_input: src_idx,

            output: self.output().into(),

            action: Some(pb::file_action::Action::Copy(pb::FileActionCopy {
                src,
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
