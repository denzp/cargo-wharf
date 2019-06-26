use std::collections::BTreeMap;

use crate::plan::RawInvocation;

#[derive(Clone, Debug, PartialEq)]
pub struct CommandDetails {
    pub env: BTreeMap<String, String>,
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Simple(CommandDetails),

    WithBuildscript {
        buildscript: CommandDetails,
        command: CommandDetails,
    },
}

impl From<&RawInvocation> for Command {
    fn from(invocation: &RawInvocation) -> Self {
        Command::Simple(CommandDetails::from(invocation))
    }
}

impl From<&RawInvocation> for CommandDetails {
    fn from(invocation: &RawInvocation) -> Self {
        CommandDetails {
            program: invocation.program.clone(),
            args: invocation.args.clone(),
            env: invocation.env.clone(),
        }
    }
}
