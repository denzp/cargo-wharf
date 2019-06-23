use std::fmt::Debug;
use std::iter::once;

use buildkit_proto::pb;
use prost::Message;
use sha2::{Digest, Sha256};

#[derive(Debug)]
pub struct Output {
    pub head: SerializedNode,
    pub tail: Vec<SerializedNode>,
}

#[derive(Debug, Default)]
pub struct SerializedNode {
    pub bytes: Vec<u8>,
    pub digest: String,
    pub metadata: pb::OpMetadata,
}

pub trait Operation: Debug + Send + Sync {
    fn serialize(&self) -> Result<Output, ()>;
}

impl SerializedNode {
    pub fn new(message: pb::Op, metadata: pb::OpMetadata) -> Self {
        let mut hasher = Sha256::new();
        let mut bytes = Vec::new();

        message.encode(&mut bytes).unwrap();
        hasher.input(&bytes);

        Self {
            bytes,
            metadata,
            digest: format!("sha256:{:x}", hasher.result()),
        }
    }
}

impl IntoIterator for Output {
    type Item = SerializedNode;
    existential type IntoIter: Iterator<Item = SerializedNode>;

    fn into_iter(self) -> Self::IntoIter {
        self.tail.into_iter().chain(once(self.head))
    }
}