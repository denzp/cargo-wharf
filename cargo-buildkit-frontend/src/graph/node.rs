use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use semver::Version;
use serde::Serialize;

use crate::plan::{RawInvocation, RawTargetKind};

#[derive(Debug, Clone, Serialize)]
pub struct Node {
    package_name: String,
    package_version: Version,

    command: NodeCommand,

    kind: NodeKind,
    outputs: Vec<PathBuf>,
    links: BTreeMap<PathBuf, PathBuf>,
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize)]
pub enum NodeKind {
    Test,
    Binary,
    Example,
    Other,
    BuildScript,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum NodeCommand {
    Simple(NodeCommandDetails),

    WithBuildscript {
        buildscript: NodeCommandDetails,
        command: NodeCommandDetails,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NodeCommandDetails {
    pub env: BTreeMap<String, String>,
    pub program: String,
    pub args: Vec<String>,
}

impl Node {
    pub fn get_outputs_iter(&self) -> impl Iterator<Item = &Path> {
        self.outputs.iter().map(|path| path.as_path())
    }

    pub fn get_links_iter(&self) -> impl Iterator<Item = (&Path, &Path)> {
        self.links
            .iter()
            .map(|(dest, src)| (dest.as_path(), src.as_path()))
    }

    pub fn get_exports_iter(&self) -> impl Iterator<Item = &Path> {
        self.get_outputs_iter()
            .chain(self.get_links_iter().map(|pair| pair.0))
    }

    pub fn kind(&self) -> NodeKind {
        self.kind
    }

    pub fn command(&self) -> &NodeCommand {
        &self.command
    }

    pub fn into_command_details(self) -> NodeCommandDetails {
        match self.command {
            NodeCommand::Simple(details) => details,
            NodeCommand::WithBuildscript { command, .. } => command,
        }
    }

    pub fn add_buildscript_command(&mut self, buildscript: NodeCommandDetails) {
        take_mut::take(&mut self.command, |command| {
            command.add_buildscript(buildscript)
        });
    }
}

impl NodeCommand {
    pub fn add_buildscript(self, buildscript: NodeCommandDetails) -> Self {
        match self {
            NodeCommand::Simple(command) => NodeCommand::WithBuildscript {
                buildscript,
                command,
            },

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

impl From<&RawInvocation> for Node {
    fn from(invocation: &RawInvocation) -> Self {
        Self {
            kind: invocation.into(),

            package_name: invocation.package_name.clone(),
            package_version: invocation.package_version.clone(),

            command: invocation.into(),

            outputs: invocation.outputs.clone(),
            links: invocation.links.clone(),
        }
    }
}

impl From<&RawInvocation> for NodeKind {
    fn from(invocation: &RawInvocation) -> Self {
        if invocation.args.contains(&String::from("--test")) {
            return NodeKind::Test;
        }

        if invocation.target_kind.contains(&RawTargetKind::Bin) {
            return NodeKind::Binary;
        }

        if invocation.target_kind.contains(&RawTargetKind::Example) {
            return NodeKind::Example;
        }

        if invocation.target_kind.contains(&RawTargetKind::CustomBuild)
            && invocation.program != "rustc"
        {
            return NodeKind::BuildScript;
        }

        NodeKind::Other
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
        }
    }
}
