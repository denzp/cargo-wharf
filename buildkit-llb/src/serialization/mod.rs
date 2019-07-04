use std::collections::BTreeMap;

use log::*;

mod id;
mod operation;
mod output;

pub(crate) use self::id::OperationId;
pub(crate) use self::operation::Operation;
pub(crate) use self::output::{Node, Output};

pub(crate) type Result<T> = std::result::Result<T, ()>;

#[derive(Default)]
pub struct Context {
    chain: Vec<u64>,
    inner: BTreeMap<u64, Node>,
}

impl Context {
    pub(crate) fn enter<F>(&mut self, id: &OperationId, closure: F) -> Result<Output>
    where
        F: FnOnce(&mut Self) -> Result<Output>,
    {
        if self.chain.contains(&**id) {
            // TODO: return nice error
            error!("cicrular dependency...");
            return Err(());
        }

        self.chain.push(**id);
        let output = closure(self)?;

        self.chain.pop();
        Ok(output)
    }

    pub(crate) fn reuse<F>(&mut self, id: &OperationId, fallback: F) -> Result<Node>
    where
        F: FnOnce(&mut Self) -> Result<Node>,
    {
        if let Some(node) = self.inner.get(&*id) {
            return Ok(node.clone());
        }

        let node = fallback(self)?;

        self.inner.insert(**id, node.clone());
        Ok(node)
    }
}
