use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use semver::Version;

use super::command::{Command, CommandDetails};
use crate::plan::{RawInvocation, RawTargetKind};

#[derive(Debug)]
pub struct Node {
    package_name: String,
    package_version: Version,

    command: Command,

    kind: NodeKind,
    outputs: Vec<PathBuf>,
    links: BTreeMap<PathBuf, PathBuf>,
}

#[derive(Debug, PartialEq)]
pub enum NodeKind {
    Test,
    Binary,
    Example,
    Other,
    BuildScript,
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

    pub fn kind(&self) -> &NodeKind {
        &self.kind
    }

    pub fn command(&self) -> &Command {
        &self.command
    }

    pub fn add_buildscript_command(&mut self, buildscript: CommandDetails) {
        if let Command::Simple(command) = self.command.clone() {
            self.command = Command::WithBuildscript {
                buildscript,
                command,
            };
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
