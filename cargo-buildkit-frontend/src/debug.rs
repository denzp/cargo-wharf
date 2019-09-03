use std::mem::replace;
use std::path::Path;

use prost::Message;

use buildkit_frontend::Options;
use buildkit_llb::ops::fs::SequenceOperation;
use buildkit_llb::prelude::*;
use buildkit_proto::pb;

pub struct DebugOperation {
    inner: SequenceOperation<'static>,
}

impl DebugOperation {
    pub fn new() -> Self {
        Self {
            inner: FileSystem::sequence().custom_name("Writing the debug output"),
        }
    }

    pub fn maybe<O>(&mut self, options: &Options, output: &O)
    where
        O: DebugOutput,
    {
        if options.has_value("debug", O::KEY) {
            self.append_debug_output(O::PATH, output);
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
    const KEY: &'static str;
    const PATH: &'static str;

    fn as_bytes(&self) -> Vec<u8>;
}

impl DebugOutput for crate::config::Config {
    const KEY: &'static str = "config";
    const PATH: &'static str = "config.json";

    fn as_bytes(&self) -> Vec<u8> {
        serde_json::to_vec_pretty(self).unwrap()
    }
}

impl DebugOutput for crate::plan::RawBuildPlan {
    const KEY: &'static str = "build-plan";
    const PATH: &'static str = "build-plan.json";

    fn as_bytes(&self) -> Vec<u8> {
        serde_json::to_vec_pretty(self).unwrap()
    }
}

impl DebugOutput for crate::graph::BuildGraph {
    const KEY: &'static str = "build-graph";
    const PATH: &'static str = "build-graph.json";

    fn as_bytes(&self) -> Vec<u8> {
        serde_json::to_vec_pretty(self).unwrap()
    }
}

impl DebugOutput for pb::Definition {
    const KEY: &'static str = "llb";
    const PATH: &'static str = "llb.pb";

    fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.encode(&mut bytes).unwrap();

        bytes
    }
}
