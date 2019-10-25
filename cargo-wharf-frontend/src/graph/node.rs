use std::collections::BTreeMap;
use std::mem::replace;
use std::path::{Path, PathBuf};

use semver::Version;
use serde::{Deserialize, Serialize};

use crate::plan::{RawInvocation, RawTargetKind};
use crate::shared::tools::{BUILDSCRIPT_APPLY, BUILDSCRIPT_CAPTURE};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Node {
    package_name: String,
    package_version: Version,

    command: NodeCommand,

    kind: NodeKind<PathBuf>,
    outputs: Vec<PathBuf>,
    output_dirs: Vec<PathBuf>,
    links: BTreeMap<PathBuf, PathBuf>,
}

#[derive(Debug, PartialEq, Clone, Copy, Deserialize, Serialize)]
pub enum PrimitiveNodeKind {
    Test,
    Binary,
    Example,
    Other,
    BuildScriptCompile,
    BuildScriptRun,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub enum NodeKind<P> {
    Primitive(PrimitiveNodeKind),
    MergedBuildScript(P),
    BuildScriptOutputConsumer(PrimitiveNodeKind, P),
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum NodeCommand {
    Simple(NodeCommandDetails),

    WithBuildscript {
        compile: NodeCommandDetails,
        run: NodeCommandDetails,
    },
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct NodeCommandDetails {
    pub env: BTreeMap<String, String>,
    pub program: String,
    pub cwd: PathBuf,
    pub args: Vec<String>,
}

pub enum BuildScriptMergeResult {
    Ok,
    DependencyBuildScript,
    AlreadyMerged,
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

    pub fn package_version(&self) -> &Version {
        &self.package_version
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

    pub fn test_name(&self) -> Option<&str> {
        match self.kind {
            NodeKind::Primitive(PrimitiveNodeKind::Test) => {}
            NodeKind::BuildScriptOutputConsumer(PrimitiveNodeKind::Test, _) => {}

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

    pub fn add_buildscript_compile_node(
        &mut self,
        mut compile_node: Node,
    ) -> BuildScriptMergeResult {
        let out_dir: PathBuf = match self.command {
            NodeCommand::Simple(ref mut details) => {
                let real_buildscript_path = {
                    compile_node
                        .links_iter()
                        .filter(|(to, _)| *to == Path::new(&details.program))
                        .map(|(_, from)| from)
                        .next()
                };

                if let Some(path) = real_buildscript_path {
                    details.program = path.to_string_lossy().into();
                } else {
                    return BuildScriptMergeResult::DependencyBuildScript;
                }

                details.use_wrapper(BUILDSCRIPT_CAPTURE);
                details.env.get("OUT_DIR").unwrap().into()
            }

            NodeCommand::WithBuildscript { .. } => {
                return BuildScriptMergeResult::AlreadyMerged;
            }
        };

        self.kind = NodeKind::MergedBuildScript(out_dir.clone());
        self.output_dirs.append(&mut compile_node.output_dirs);
        self.output_dirs.push(out_dir.clone());
        self.outputs.push(out_dir);

        take_mut::take(&mut self.command, |command| {
            command.add_buildscript_compile(compile_node.into_command_details())
        });

        BuildScriptMergeResult::Ok
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

    pub fn add_dependency_buildscript(&mut self, dependency: Node) {
        let out_dir = match dependency.command {
            NodeCommand::Simple(ref details) => details.env.get("OUT_DIR"),
            NodeCommand::WithBuildscript { ref run, .. } => run.env.get("OUT_DIR"),
        };

        let out_dir = match out_dir {
            Some(dir) => dir,
            None => {
                return;
            }
        };

        if let NodeCommand::WithBuildscript { ref mut run, .. } = self.command {
            run.args
                .insert(0, format!("--with-metadata-from={}", out_dir));
        }
    }

    pub fn sources_path(&self) -> &Path {
        match self.command {
            NodeCommand::Simple(ref details) => &details.cwd,
            NodeCommand::WithBuildscript { ref compile, .. } => &compile.cwd,
        }
    }
}

impl NodeCommand {
    pub fn add_buildscript_compile(self, compile: NodeCommandDetails) -> Self {
        match self {
            NodeCommand::Simple(run) => NodeCommand::WithBuildscript { compile, run },
            other => other,
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
            return NodeKind::Primitive(PrimitiveNodeKind::BuildScriptRun);
        }

        if invocation.target_kind.contains(&RawTargetKind::CustomBuild)
            && invocation.program == "rustc"
        {
            return NodeKind::Primitive(PrimitiveNodeKind::BuildScriptCompile);
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
