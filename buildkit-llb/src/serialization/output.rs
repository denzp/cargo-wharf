use std::iter::once;

use buildkit_proto::pb;
use prost::Message;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub(crate) struct Output {
    pub head: Node,
    pub tail: Vec<Node>,
}

// TODO: make me `pub(crate)`
#[derive(Debug, Default, Clone)]
pub struct Node {
    pub bytes: Vec<u8>,
    pub digest: String,
    pub metadata: pb::OpMetadata,
}

impl Node {
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
    type Item = Node;
    existential type IntoIter: Iterator<Item = Node>;

    fn into_iter(self) -> Self::IntoIter {
        self.tail.into_iter().chain(once(self.head))
    }
}
