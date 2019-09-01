use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use buildkit_proto::pb::{self, op::Op, OpMetadata, SourceOp};

use crate::ops::{OperationBuilder, SingleBorrowedOutput, SingleOwnedOutput};
use crate::serialization::{Context, Node, Operation, OperationId, Result};
use crate::utils::{OperationOutput, OutputIdx};

#[derive(Debug)]
pub struct ImageSource {
    id: OperationId,
    name: String,
    description: HashMap<String, String>,
    ignore_cache: bool,
    resolve_mode: Option<ResolveMode>,
    digest: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum ResolveMode {
    Default,
    ForcePull,
    PreferLocal,
}

impl fmt::Display for ResolveMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ResolveMode::Default => write!(f, "default"),
            ResolveMode::ForcePull => write!(f, "pull"),
            ResolveMode::PreferLocal => write!(f, "local"),
        }
    }
}

impl Default for ResolveMode {
    fn default() -> Self {
        ResolveMode::Default
    }
}

impl ImageSource {
    pub(crate) fn new<S>(name: S) -> Self
    where
        S: Into<String>,
    {
        let mut name = name.into();

        let tag_separator = match name.find(':') {
            Some(len) => len,
            None => {
                let original_len = name.len();

                name += ":latest";
                original_len
            }
        };

        if name[..tag_separator].find('/').is_none() {
            name = String::from("library/") + &name;
        }

        Self {
            id: OperationId::default(),
            name: format!("docker.io/{}", name),
            description: Default::default(),
            ignore_cache: false,
            resolve_mode: None,
            digest: None,
        }
    }

    pub fn with_resolve_mode(mut self, mode: ResolveMode) -> Self {
        self.resolve_mode = Some(mode);
        self
    }

    pub fn resolve_mode(&self) -> Option<ResolveMode> {
        self.resolve_mode
    }

    pub fn with_digest<S>(mut self, digest: S) -> Self
    where
        S: Into<String>,
    {
        self.digest = Some(digest.into());
        self
    }

    pub fn canonical_name(&self) -> &str {
        &self.name
    }
}

impl<'a> SingleBorrowedOutput<'a> for ImageSource {
    fn output(&'a self) -> OperationOutput<'a> {
        OperationOutput::borrowed(self, OutputIdx(0))
    }
}

impl<'a> SingleOwnedOutput<'static> for Arc<ImageSource> {
    fn output(&self) -> OperationOutput<'static> {
        OperationOutput::owned(self.clone(), OutputIdx(0))
    }
}

impl OperationBuilder<'static> for ImageSource {
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

impl Operation for ImageSource {
    fn id(&self) -> &OperationId {
        &self.id
    }

    fn serialize(&self, _: &mut Context) -> Result<Node> {
        let mut attrs = HashMap::default();

        if let Some(ref mode) = self.resolve_mode {
            attrs.insert("image.resolvemode".into(), mode.to_string());
        }

        let head = pb::Op {
            op: Some(Op::Source(SourceOp {
                identifier: match self.digest {
                    None => format!("docker-image://{}", self.canonical_name()),
                    Some(ref digest) => {
                        format!("docker-image://{}@{}", self.canonical_name(), digest)
                    }
                },
                attrs,
            })),

            ..Default::default()
        };

        let metadata = OpMetadata {
            description: self.description.clone(),
            ignore_cache: self.ignore_cache,

            ..Default::default()
        };

        Ok(Node::new(head, metadata))
    }
}

#[test]
fn serialization() {
    crate::check_op!(
        ImageSource::new("rustlang/rust:nightly"),
        |digest| { "sha256:dee2a3d7dd482dd8098ba543ff1dcb01efd29fcd16fdb0979ef556f38564543a" },
        |description| { vec![] },
        |caps| { vec![] },
        |cached_tail| { vec![] },
        |inputs| { vec![] },
        |op| {
            Op::Source(SourceOp {
                identifier: "docker-image://docker.io/rustlang/rust:nightly".into(),
                attrs: Default::default(),
            })
        },
    );

    crate::check_op!(
        ImageSource::new("library/alpine:latest"),
        |digest| { "sha256:0e6b31ceed3e6dc542018f35a53a0e857e6a188453d32a2a5bbe7aa2971c1220" },
        |description| { vec![] },
        |caps| { vec![] },
        |cached_tail| { vec![] },
        |inputs| { vec![] },
        |op| {
            Op::Source(SourceOp {
                identifier: "docker-image://docker.io/library/alpine:latest".into(),
                attrs: Default::default(),
            })
        },
    );

    crate::check_op!(
        ImageSource::new("rustlang/rust:nightly").custom_name("image custom name"),
        |digest| { "sha256:dee2a3d7dd482dd8098ba543ff1dcb01efd29fcd16fdb0979ef556f38564543a" },
        |description| { vec![("llb.customname", "image custom name")] },
        |caps| { vec![] },
        |cached_tail| { vec![] },
        |inputs| { vec![] },
        |op| {
            Op::Source(SourceOp {
                identifier: "docker-image://docker.io/rustlang/rust:nightly".into(),
                attrs: Default::default(),
            })
        },
    );

    crate::check_op!(
        ImageSource::new("rustlang/rust:nightly").with_digest("sha256:123456"),
        |digest| { "sha256:a9837e26998d165e7b6433f8d40b36d259905295860fcbbc62bbce75a6c991c6" },
        |description| { vec![] },
        |caps| { vec![] },
        |cached_tail| { vec![] },
        |inputs| { vec![] },
        |op| {
            Op::Source(SourceOp {
                identifier: "docker-image://docker.io/rustlang/rust:nightly@sha256:123456".into(),
                attrs: Default::default(),
            })
        },
    );
}

#[test]
fn resolve_mode() {
    crate::check_op!(
        ImageSource::new("rustlang/rust:nightly").with_resolve_mode(ResolveMode::Default),
        |digest| { "sha256:792e246751e84b9a5e40c28900d70771a07e8cc920c1039cdddfc6bf69256dfe" },
        |description| { vec![] },
        |caps| { vec![] },
        |cached_tail| { vec![] },
        |inputs| { vec![] },
        |op| {
            Op::Source(SourceOp {
                identifier: "docker-image://docker.io/rustlang/rust:nightly".into(),
                attrs: crate::utils::test::to_map(vec![("image.resolvemode", "default")]),
            })
        },
    );

    crate::check_op!(
        ImageSource::new("rustlang/rust:nightly").with_resolve_mode(ResolveMode::ForcePull),
        |digest| { "sha256:0bd920010eab701bdce44c61d220e6943d56d3fb9a9fa4e773fc060c0d746122" },
        |description| { vec![] },
        |caps| { vec![] },
        |cached_tail| { vec![] },
        |inputs| { vec![] },
        |op| {
            Op::Source(SourceOp {
                identifier: "docker-image://docker.io/rustlang/rust:nightly".into(),
                attrs: crate::utils::test::to_map(vec![("image.resolvemode", "pull")]),
            })
        },
    );

    crate::check_op!(
        ImageSource::new("rustlang/rust:nightly").with_resolve_mode(ResolveMode::PreferLocal),
        |digest| { "sha256:bd6797c8644d2663b29c36a8b3b63931e539be44ede5e56aca2da4f35f241f18" },
        |description| { vec![] },
        |caps| { vec![] },
        |cached_tail| { vec![] },
        |inputs| { vec![] },
        |op| {
            Op::Source(SourceOp {
                identifier: "docker-image://docker.io/rustlang/rust:nightly".into(),
                attrs: crate::utils::test::to_map(vec![("image.resolvemode", "local")]),
            })
        },
    );
}

#[test]
fn image_name() {
    crate::check_op!(ImageSource::new("rustlang/rust"), |op| {
        Op::Source(SourceOp {
            identifier: "docker-image://docker.io/rustlang/rust:latest".into(),
            attrs: Default::default(),
        })
    });

    crate::check_op!(ImageSource::new("rust:nightly"), |op| {
        Op::Source(SourceOp {
            identifier: "docker-image://docker.io/library/rust:nightly".into(),
            attrs: Default::default(),
        })
    });

    crate::check_op!(ImageSource::new("rust"), |op| {
        Op::Source(SourceOp {
            identifier: "docker-image://docker.io/library/rust:latest".into(),
            attrs: Default::default(),
        })
    });

    crate::check_op!(ImageSource::new("rust:complex/tag"), |op| {
        Op::Source(SourceOp {
            identifier: "docker-image://docker.io/library/rust:complex/tag".into(),
            attrs: Default::default(),
        })
    });
}
