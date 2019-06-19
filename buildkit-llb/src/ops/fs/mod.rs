use std::fmt::Debug;

use buildkit_proto::pb;

use crate::serialization::SerializedNode;

mod copy;
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
}

pub trait FileOperation: Debug + Send + Sync {
    fn output(&self) -> i64;

    fn serialize_inputs(&self) -> Result<(Vec<pb::Input>, Vec<SerializedNode>), ()>;
    fn serialize_action(
        &self,
        inputs_count: usize,
        inputs_offset: usize,
    ) -> Result<pb::FileAction, ()>;
}
