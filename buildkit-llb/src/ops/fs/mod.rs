use std::fmt::Debug;
use std::path::Path;

use buildkit_proto::pb;

use crate::serialization::{SerializationResult, SerializedNode};
use crate::utils::OutputIdx;

mod copy;
mod mkdir;
mod path;
mod sequence;

pub use self::path::{LayerPath, UnsetPath};

/// Umbrella operation that handles file system related routines.
/// Dockerfile's `COPY` directive is a partial case of this.
pub struct FileSystem;

impl FileSystem {
    pub fn sequence() -> sequence::SequenceOperation<'static> {
        sequence::SequenceOperation::new()
    }

    pub fn copy() -> copy::CopyOperation<UnsetPath, UnsetPath> {
        copy::CopyOperation::new()
    }

    pub fn mkdir<'a, P>(output: OutputIdx, layer: LayerPath<'a, P>) -> mkdir::MakeDirOperation<'a>
    where
        P: AsRef<Path>,
    {
        mkdir::MakeDirOperation::new(output, layer)
    }
}

pub trait FileOperation: Debug + Send + Sync {
    fn output(&self) -> i64;

    fn serialize_tail(&self) -> SerializationResult<Vec<SerializedNode>>;
    fn serialize_inputs(&self) -> SerializationResult<Vec<pb::Input>>;
    fn serialize_action(
        &self,
        inputs_count: usize,
        inputs_offset: usize,
    ) -> SerializationResult<pb::FileAction>;
}
