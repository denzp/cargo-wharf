use std::collections::HashMap;
use std::iter::{empty, once};
use std::path::{Path, PathBuf};

use buildkit_proto::pb::{
    self, op::Op, ExecOp, Input, MountType, NetMode, OpMetadata, SecurityMode,
};
use either::Either;
use unzip3::Unzip3;

use super::context::Context;
use super::mount::Mount;

use crate::ops::OperationBuilder;
use crate::serialization::{Operation, Output, SerializedNode};
use crate::utils::{OperationOutput, OutputIdx};

/// Command execution operation. This is what a Dockerfile's `RUN` directive being translated to.
#[derive(Debug)]
pub struct Command<'a> {
    context: Context,
    mounts: Vec<Mount<'a, PathBuf>>,

    description: HashMap<String, String>,
    caps: HashMap<String, bool>,
    ignore_cache: bool,
}

impl<'a> Command<'a> {
    pub fn run<S>(name: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            context: Context::new(name),
            mounts: vec![],

            description: Default::default(),
            caps: Default::default(),
            ignore_cache: false,
        }
    }

    pub fn args<A, S>(mut self, args: A) -> Self
    where
        A: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.context.args = args.into_iter().map(|item| item.as_ref().into()).collect();
        self
    }

    pub fn env<S, Q>(mut self, name: S, value: Q) -> Self
    where
        S: AsRef<str>,
        Q: AsRef<str>,
    {
        let env = format!("{}={}", name.as_ref(), value.as_ref());

        self.context.env.push(env);
        self
    }

    pub fn cwd<P>(mut self, path: P) -> Self
    where
        P: Into<PathBuf>,
    {
        self.context.cwd = path.into();
        self
    }

    pub fn mount<P>(mut self, mount: Mount<'a, P>) -> Self
    where
        P: AsRef<Path>,
    {
        match mount {
            Mount::Layer(..) | Mount::ReadOnlyLayer(..) | Mount::Scratch(..) => {
                self.caps.insert("exec.mount.bind".into(), true);
            }

            Mount::ReadOnlySelector(..) => {
                self.caps.insert("exec.mount.bind".into(), true);
                self.caps.insert("exec.mount.selector".into(), true);
            }

            Mount::SharedCache(..) => {
                self.caps.insert("exec.mount.cache".into(), true);
                self.caps.insert("exec.mount.cache.sharing".into(), true);
            }
        }

        self.mounts.push(mount.into_owned());
        self
    }

    pub fn output(&self, index: u32) -> OperationOutput {
        // TODO: check if the requested index available.

        OperationOutput(self, OutputIdx(index))
    }
}

impl<'a> OperationBuilder for Command<'a> {
    fn custom_name<S>(mut self, name: S) -> Self
    where
        S: Into<String>,
    {
        self.description
            .insert("llb.customname".into(), name.into());

        self
    }

    fn ignore_cache(mut self, ignore: bool) -> Self {
        self.ignore_cache = ignore;
        self
    }
}

impl<'a> Operation for Command<'a> {
    fn serialize(&self) -> Result<Output, ()> {
        let (inputs, mounts, tails): (Vec<_>, Vec<_>, Vec<_>) = {
            let mut last_input_index = 0;

            self.mounts
                .iter()
                .map(|mount| {
                    let inner_mount = match mount {
                        Mount::ReadOnlyLayer(_, destination) => pb::Mount {
                            input: last_input_index,
                            dest: destination.to_string_lossy().into(),
                            output: -1,
                            readonly: true,
                            mount_type: MountType::Bind as i32,

                            ..Default::default()
                        },

                        Mount::ReadOnlySelector(_, destination, source) => pb::Mount {
                            input: last_input_index,
                            dest: destination.to_string_lossy().into(),
                            output: -1,
                            readonly: true,
                            selector: source.to_string_lossy().into(),
                            mount_type: MountType::Bind as i32,

                            ..Default::default()
                        },

                        Mount::Layer(output, _, path) => pb::Mount {
                            input: last_input_index,
                            dest: path.to_string_lossy().into(),
                            output: output.into(),
                            mount_type: MountType::Bind as i32,

                            ..Default::default()
                        },

                        Mount::Scratch(output, path) => {
                            let mount = pb::Mount {
                                input: -1,
                                dest: path.to_string_lossy().into(),
                                output: output.into(),
                                mount_type: MountType::Bind as i32,

                                ..Default::default()
                            };

                            return (Either::Right(empty()), mount, Either::Right(empty()));
                        }

                        Mount::SharedCache(path) => {
                            use buildkit_proto::pb::{CacheOpt, CacheSharingOpt};

                            let mount = pb::Mount {
                                input: -1,
                                dest: path.to_string_lossy().into(),
                                output: -1,
                                mount_type: MountType::Cache as i32,

                                cache_opt: Some(CacheOpt {
                                    id: path.display().to_string(),
                                    sharing: CacheSharingOpt::Shared as i32,
                                }),

                                ..Default::default()
                            };

                            return (Either::Right(empty()), mount, Either::Right(empty()));
                        }
                    };

                    let input = match mount {
                        Mount::ReadOnlyLayer(input, ..) => input,
                        Mount::ReadOnlySelector(input, ..) => input,
                        Mount::Layer(_, input, ..) => input,

                        Mount::SharedCache(..) => {
                            unreachable!();
                        }

                        Mount::Scratch(..) => {
                            unreachable!();
                        }
                    };

                    let serialized = input.0.serialize().unwrap();
                    let input = Input {
                        digest: serialized.head.digest.clone(),
                        index: input.1.into(),
                    };

                    last_input_index += 1;

                    (
                        Either::Left(once(input)),
                        inner_mount,
                        Either::Left(serialized.into_iter()),
                    )
                })
                .unzip3()
        };

        let head = pb::Op {
            op: Some(Op::Exec(ExecOp {
                mounts,
                network: NetMode::Unset.into(),
                security: SecurityMode::Sandbox.into(),
                meta: Some(self.context.clone().into()),
            })),

            inputs: inputs.into_iter().flatten().collect(),

            ..Default::default()
        };

        let metadata = OpMetadata {
            description: self.description.clone(),
            caps: self.caps.clone(),
            ignore_cache: self.ignore_cache,

            ..Default::default()
        };

        Ok(Output {
            head: SerializedNode::new(head, metadata),
            tail: tails.into_iter().flatten().collect(),
        })
    }
}
