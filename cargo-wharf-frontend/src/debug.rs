use std::mem::replace;
use std::path::Path;

use prost::Message;
use serde::Deserialize;

use buildkit_llb::ops::fs::SequenceOperation;
use buildkit_llb::prelude::*;
use buildkit_proto::pb;

use crate::frontend::Options;

pub struct DebugOperation {
    inner: SequenceOperation<'static>,
}

#[derive(Debug, Deserialize, PartialEq, Clone, Copy)]
#[serde(untagged)]
#[serde(field_identifier, rename_all = "kebab-case")]
pub enum DebugKind {
    All,
    Config,
    BuildPlan,
    BuildGraph,

    #[serde(rename = "llb")]
    LLB,
}

impl DebugOperation {
    pub fn new() -> Self {
        Self {
            inner: FileSystem::sequence().custom_name("Writing the debug output"),
        }
    }

    pub fn maybe<G, O>(&mut self, options: &Options, getter: G)
    where
        G: FnOnce() -> O,
        O: DebugOutput,
    {
        if options.debug.contains(&O::KEY) || options.debug.contains(&DebugKind::All) {
            self.append_debug_output(O::PATH, &getter());
        }
    }

    pub fn terminal(&self) -> Terminal<'_> {
        Terminal::with(self.inner.last_output().unwrap())
    }

    fn append_debug_output<P, O>(&mut self, path: P, output: &O)
    where
        P: AsRef<Path>,
        O: DebugOutput,
    {
        let (index, layer_path) = match self.inner.last_output_index() {
            Some(index) => (index + 1, LayerPath::Own(OwnOutputIdx(index), path)),
            None => (0, LayerPath::Scratch(path)),
        };

        self.inner = replace(&mut self.inner, FileSystem::sequence())
            .append(FileSystem::mkfile(OutputIdx(index), layer_path).data(output.as_bytes()));
    }
}

pub trait DebugOutput {
    const KEY: DebugKind;
    const PATH: &'static str;

    fn as_bytes(&self) -> Vec<u8>;
}

impl<'a> DebugOutput for &'a crate::config::Config {
    const KEY: DebugKind = DebugKind::Config;
    const PATH: &'static str = "config.json";

    fn as_bytes(&self) -> Vec<u8> {
        serde_json::to_vec_pretty(self).unwrap()
    }
}

impl<'a> DebugOutput for &'a crate::plan::RawBuildPlan {
    const KEY: DebugKind = DebugKind::BuildPlan;
    const PATH: &'static str = "build-plan.json";

    fn as_bytes(&self) -> Vec<u8> {
        serde_json::to_vec_pretty(self).unwrap()
    }
}

impl<'a> DebugOutput for &'a crate::graph::BuildGraph {
    const KEY: DebugKind = DebugKind::BuildGraph;
    const PATH: &'static str = "build-graph.json";

    fn as_bytes(&self) -> Vec<u8> {
        serde_json::to_vec_pretty(self).unwrap()
    }
}

impl DebugOutput for pb::Definition {
    const KEY: DebugKind = DebugKind::LLB;
    const PATH: &'static str = "llb.pb";

    fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.encode(&mut bytes).unwrap();

        bytes
    }
}
