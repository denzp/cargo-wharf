use std::collections::BTreeMap;
use std::mem::replace;
use std::path::{Path, PathBuf};

use semver::Version;
use serde::Serialize;

use crate::plan::{RawInvocation, RawTargetKind};
use crate::shared::tools::{BUILDSCRIPT_APPLY, BUILDSCRIPT_CAPTURE};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Node {
    package_name: String,
    package_version: Version,

    command: NodeCommand,

    kind: NodeKind<PathBuf>,
    outputs: Vec<PathBuf>,
    output_dirs: Vec<PathBuf>,
    links: BTreeMap<PathBuf, PathBuf>,
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize)]
pub enum PrimitiveNodeKind {
    Test,
    Binary,
    Example,
    Other,
    BuildScript,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub enum NodeKind<P> {
    Primitive(PrimitiveNodeKind),
    MergedBuildScript(P),
    BuildScriptOutputConsumer(PrimitiveNodeKind, P),
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum NodeCommand {
    Simple(NodeCommandDetails),

    WithBuildscript {
        compile: NodeCommandDetails,
        run: NodeCommandDetails,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NodeCommandDetails {
    pub env: BTreeMap<String, String>,
    pub program: String,
    pub cwd: PathBuf,
    pub args: Vec<String>,
}

impl Node {
    pub fn outputs_iter(&self) -> impl Iterator<Item = &Path> {
        self.outputs.iter().map(|path| path.as_path())
    }

    pub fn output_dirs_iter(&self) -> impl Iterator<Item = &Path> {
        self.output_dirs.iter().map(|path| path.as_path())
    }

    pub fn links_iter(&self) -> impl Iterator<Item = (&Path, &Path)> {
        self.links
            .iter()
            .map(|(dest, src)| (dest.as_path(), src.as_path()))
    }

    pub fn kind(&self) -> NodeKind<&Path> {
        use NodeKind::*;

        match self.kind {
            Primitive(kind) => Primitive(kind),
            MergedBuildScript(ref buf) => MergedBuildScript(buf.as_path()),

            BuildScriptOutputConsumer(original, ref buf) => {
                BuildScriptOutputConsumer(original, buf.as_path())
            }
        }
    }

    pub fn package_name(&self) -> &str {
        &self.package_name
    }

    pub fn binary_name(&self) -> Option<&str> {
        match self.kind {
            NodeKind::Primitive(PrimitiveNodeKind::Binary) => {}
            NodeKind::BuildScriptOutputConsumer(PrimitiveNodeKind::Binary, _) => {}

            _ => return None,
        };

        self.links_iter()
            .next()
            .and_then(|(to, _)| to.file_name().and_then(|name| name.to_str()))
            .or_else(|| Some(self.package_name()))
    }

    pub fn command(&self) -> &NodeCommand {
        &self.command
    }

    pub fn into_command_details(self) -> NodeCommandDetails {
        match self.command {
            NodeCommand::Simple(details) => details,
            NodeCommand::WithBuildscript { compile, .. } => compile,
        }
    }

    pub fn add_buildscript_run_command(&mut self, mut run_command: NodeCommandDetails) {
        let out_dir: PathBuf = run_command.env.get("OUT_DIR").unwrap().into();

        run_command.use_wrapper(BUILDSCRIPT_CAPTURE);

        take_mut::take(&mut self.command, |command| {
            command.add_buildscript_run(run_command)
        });

        self.kind = NodeKind::MergedBuildScript(out_dir.clone());
        self.output_dirs.push(out_dir.clone());
        self.outputs = vec![out_dir];
        self.links.clear();
    }

    pub fn transform_into_buildscript_consumer(&mut self, out_dir: &Path) {
        if let NodeCommand::Simple(ref mut details) = self.command {
            details.use_wrapper(BUILDSCRIPT_APPLY);
        }

        self.kind = match self.kind {
            NodeKind::Primitive(kind) => NodeKind::BuildScriptOutputConsumer(kind, out_dir.into()),
            _ => NodeKind::BuildScriptOutputConsumer(PrimitiveNodeKind::Other, out_dir.into()),
        };
    }
}

impl NodeCommand {
    pub fn add_buildscript_run(self, run: NodeCommandDetails) -> Self {
        match self {
            NodeCommand::Simple(compile) => NodeCommand::WithBuildscript { compile, run },

            other => other,
        }
    }

    pub fn is_simple(&self) -> bool {
        if let NodeCommand::Simple(_) = self {
            true
        } else {
            false
        }
    }
}

impl NodeCommandDetails {
    pub fn use_wrapper(&mut self, wrapper: &str) {
        let original = replace(&mut self.program, wrapper.into());
        let mut args = replace(&mut self.args, vec!["--".into(), original]);

        self.args.append(&mut args);
    }
}

impl From<&RawInvocation> for Node {
    fn from(invocation: &RawInvocation) -> Self {
        Self {
            kind: invocation.into(),

            package_name: invocation.package_name.clone(),
            package_version: invocation.package_version.clone(),

            command: invocation.into(),

            outputs: invocation.outputs.clone(),
            links: invocation.links.clone(),

            output_dirs: {
                invocation
                    .outputs
                    .iter()
                    .map(|path| path.parent().unwrap().into())
                    .collect()
            },
        }
    }
}

impl From<&RawInvocation> for NodeKind<PathBuf> {
    fn from(invocation: &RawInvocation) -> Self {
        if invocation.args.contains(&String::from("--test")) {
            return NodeKind::Primitive(PrimitiveNodeKind::Test);
        }

        if invocation.target_kind.contains(&RawTargetKind::Bin) {
            return NodeKind::Primitive(PrimitiveNodeKind::Binary);
        }

        if invocation.target_kind.contains(&RawTargetKind::Example) {
            return NodeKind::Primitive(PrimitiveNodeKind::Example);
        }

        if invocation.target_kind.contains(&RawTargetKind::CustomBuild)
            && invocation.program != "rustc"
        {
            return NodeKind::Primitive(PrimitiveNodeKind::BuildScript);
        }

        NodeKind::Primitive(PrimitiveNodeKind::Other)
    }
}

impl From<&RawInvocation> for NodeCommand {
    fn from(invocation: &RawInvocation) -> Self {
        NodeCommand::Simple(NodeCommandDetails::from(invocation))
    }
}

impl From<&RawInvocation> for NodeCommandDetails {
    fn from(invocation: &RawInvocation) -> Self {
        NodeCommandDetails {
            program: invocation.program.clone(),
            args: invocation.args.clone(),
            env: invocation.env.clone(),
            cwd: invocation.cwd.clone(),
        }
    }
}
