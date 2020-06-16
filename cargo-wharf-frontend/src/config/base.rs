use std::borrow::Cow;
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::path::PathBuf;

use failure::{bail, format_err, Error};
use serde::{Deserialize, Serialize};

use buildkit_frontend::oci::{ExposedPort, Signal};
use buildkit_llb::ops::source::ImageSource;
use buildkit_llb::prelude::*;

#[derive(Debug, PartialEq, Deserialize)]
#[serde(try_from = "Vec<schema::MetadataWrapper>")]
pub struct BaseConfig {
    pub builder: BaseBuilderConfig,
    pub output: BaseOutputConfig,
    pub binaries: Vec<BinaryDefinition>,
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct BaseBuilderConfig {
    pub image: String,
    pub user: Option<String>,
    pub env: Option<BTreeMap<String, String>>,
    pub target: Option<String>,
    pub setup_commands: Option<Vec<CustomCommand>>
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct BaseOutputConfig {
    pub image: String,
    pub user: Option<String>,
    pub workdir: Option<PathBuf>,
    pub entrypoint: Option<Vec<String>>,
    pub args: Option<Vec<String>>,
    pub env: Option<BTreeMap<String, String>>,
    pub expose: Option<Vec<ExposedPort>>,
    pub volumes: Option<Vec<PathBuf>>,
    pub labels: Option<BTreeMap<String, String>>,
    pub stop_signal: Option<Signal>,
    pub pre_install_commands: Option<Vec<CustomCommand>>,
    pub post_install_commands: Option<Vec<CustomCommand>>,
    pub copy: Option<Vec<StaticAssetDefinition>>
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct BinaryDefinition {
    pub name: String,
    pub destination: PathBuf,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct StaticAssetDefinition {
    pub src: PathBuf,
    pub dst: PathBuf,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct CustomCommand {
    pub display: Option<String>,

    #[serde(flatten)]
    pub kind: CustomCommandKind,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum CustomCommandKind {
    Shell(String),
    Command(Vec<String>),
}

impl TryFrom<Vec<schema::MetadataWrapper>> for BaseConfig {
    type Error = Error;

    fn try_from(raw: Vec<schema::MetadataWrapper>) -> Result<Self, Self::Error> {
        let (builder, output, binaries) = {
            raw.into_iter()
                .filter_map(|item| item.metadata)
                .filter_map(|item| item.wharf)
                .try_fold((None, None, vec![]), extract_config)?
        };

        Ok(Self {
            builder: builder.ok_or_else(|| format_err!("Missing 'wharf.builder' section"))?,
            output: output.ok_or_else(|| format_err!("Missing 'wharf.output' section"))?,
            binaries,
        })
    }
}

impl BaseBuilderConfig {
    pub fn source(&self) -> ImageSource {
        Source::image(&self.image).with_resolve_mode(ResolveMode::PreferLocal)
    }
}

impl BaseOutputConfig {
    pub fn source(&self) -> ImageSource {
        Source::image(&self.image).with_resolve_mode(ResolveMode::PreferLocal)
    }
}

impl<'a> From<&'a CustomCommand> for (&'a str, Vec<&'a str>, Cow<'a, str>) {
    fn from(command: &'a CustomCommand) -> Self {
        match command.kind {
            CustomCommandKind::Command(ref name_and_args) => (
                &name_and_args[0],
                name_and_args[1..]
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
                command
                    .display
                    .as_ref()
                    .map(|display| Cow::Borrowed(display.as_str()))
                    .unwrap_or_else(|| Cow::Owned(name_and_args.join(" "))),
            ),

            CustomCommandKind::Shell(ref shell) => (
                "/bin/sh",
                vec!["-c", shell.as_str()],
                command
                    .display
                    .as_ref()
                    .map(|display| Cow::Borrowed(display.as_str()))
                    .unwrap_or_else(|| Cow::Borrowed(shell.as_str())),
            ),
        }
    }
}

type ConfigCtx = (
    Option<BaseBuilderConfig>,
    Option<BaseOutputConfig>,
    Vec<BinaryDefinition>,
);

fn extract_config(cx: ConfigCtx, metadata: schema::WharfMetadata) -> Result<ConfigCtx, Error> {
    let (mut builder, mut output, mut binaries) = cx;

    if let Some(mut incoming) = metadata.binary {
        binaries.append(&mut incoming);
    }

    builder = match (builder.take(), metadata.builder) {
        (builder, None) => builder,

        (None, Some(incoming)) => Some(incoming),

        (Some(_), Some(_)) => {
            bail!("Found duplicated 'wharf.builder' section");
        }
    };

    output = match (output.take(), metadata.output) {
        (output, None) => output,

        (None, Some(incoming)) => Some(incoming),

        (Some(_), Some(_)) => {
            bail!("Found duplicated 'wharf.output' section");
        }
    };

    Ok((builder, output, binaries))
}

#[test]
fn transformation() {
    use schema::*;

    let raw = vec![
        MetadataWrapper {
            metadata: Some(PackageMetadata {
                wharf: Some(WharfMetadata {
                    output: Some(BaseOutputConfig {
                        image: "alpine:latest".into(),
                        user: Some("root".into()),
                        workdir: Some("/root".into()),
                        entrypoint: None,
                        args: None,
                        env: None,
                        expose: None,
                        volumes: None,
                        labels: None,
                        stop_signal: None,
                        pre_install_commands: None,
                        post_install_commands: None,
                        copy: None,
                    }),

                    builder: None,
                    binary: None,
                }),
            }),
        },
        MetadataWrapper {
            metadata: Some(PackageMetadata {
                wharf: Some(WharfMetadata {
                    builder: Some(BaseBuilderConfig {
                        image: "rust:latest".into(),
                        env: None,
                        user: None,
                        target: None,
                        setup_commands: None
                    }),

                    output: None,
                    binary: None,
                }),
            }),
        },
        MetadataWrapper {
            metadata: Some(PackageMetadata {
                wharf: Some(WharfMetadata {
                    binary: Some(vec![BinaryDefinition {
                        name: "binary-1".into(),
                        destination: "/bin/binary-1".into(),
                    }]),

                    output: None,
                    builder: None,
                }),
            }),
        },
        MetadataWrapper { metadata: None },
        MetadataWrapper {
            metadata: Some(PackageMetadata {
                wharf: Some(WharfMetadata {
                    binary: Some(vec![BinaryDefinition {
                        name: "binary-2".into(),
                        destination: "/usr/local/bin/binary-2".into(),
                    }]),

                    output: None,
                    builder: None,
                }),
            }),
        },
    ];

    assert_eq!(
        BaseConfig::try_from(raw).unwrap(),
        BaseConfig {
            builder: BaseBuilderConfig {
                image: "rust:latest".into(),
                env: None,
                user: None,
                target: None,
                setup_commands: None,
            },
            output: BaseOutputConfig {
                image: "alpine:latest".into(),
                user: Some("root".into()),
                workdir: Some("/root".into()),
                entrypoint: None,
                args: None,
                env: None,
                expose: None,
                volumes: None,
                labels: None,
                stop_signal: None,
                pre_install_commands: None,
                post_install_commands: None,
                copy: None,
            },
            binaries: vec![
                BinaryDefinition {
                    name: "binary-1".into(),
                    destination: "/bin/binary-1".into(),
                },
                BinaryDefinition {
                    name: "binary-2".into(),
                    destination: "/usr/local/bin/binary-2".into(),
                }
            ],
        }
    );
}

#[test]
fn duplicated_config() {
    use schema::*;

    let raw = vec![
        MetadataWrapper {
            metadata: Some(PackageMetadata {
                wharf: Some(WharfMetadata {
                    builder: Some(BaseBuilderConfig {
                        image: "rust:latest".into(),
                        env: None,
                        user: None,
                        target: None,
                        setup_commands: None,
                    }),
                    output: Some(BaseOutputConfig {
                        image: "alpine:latest".into(),
                        user: Some("root".into()),
                        workdir: Some("/root".into()),
                        entrypoint: None,
                        args: None,
                        env: None,
                        expose: None,
                        volumes: None,
                        labels: None,
                        stop_signal: None,
                        pre_install_commands: None,
                        post_install_commands: None,
                        copy: None,
                    }),

                    binary: None,
                }),
            }),
        },
        MetadataWrapper {
            metadata: Some(PackageMetadata {
                wharf: Some(WharfMetadata {
                    builder: Some(BaseBuilderConfig {
                        image: "rust:latest".into(),
                        env: None,
                        user: None,
                        target: None,
                        setup_commands: None,
                    }),

                    output: None,
                    binary: None,
                }),
            }),
        },
    ];

    assert!(BaseConfig::try_from(raw).is_err());

    let raw = vec![
        MetadataWrapper {
            metadata: Some(PackageMetadata {
                wharf: Some(WharfMetadata {
                    builder: Some(BaseBuilderConfig {
                        image: "rust:latest".into(),
                        env: None,
                        user: None,
                        target: None,
                        setup_commands: None,
                    }),
                    output: Some(BaseOutputConfig {
                        image: "alpine:latest".into(),
                        user: Some("root".into()),
                        workdir: Some("/root".into()),
                        entrypoint: None,
                        args: None,
                        env: None,
                        expose: None,
                        volumes: None,
                        labels: None,
                        stop_signal: None,
                        pre_install_commands: None,
                        post_install_commands: None,
                        copy: None,
                    }),

                    binary: None,
                }),
            }),
        },
        MetadataWrapper {
            metadata: Some(PackageMetadata {
                wharf: Some(WharfMetadata {
                    output: Some(BaseOutputConfig {
                        image: "rust:latest".into(),
                        user: None,
                        workdir: None,
                        entrypoint: None,
                        args: None,
                        env: None,
                        expose: None,
                        volumes: None,
                        labels: None,
                        stop_signal: None,
                        pre_install_commands: None,
                        post_install_commands: None,
                        copy: None,
                    }),

                    builder: None,
                    binary: None,
                }),
            }),
        },
    ];

    assert!(BaseConfig::try_from(raw).is_err());
}

#[test]
fn missing_config() {
    use schema::*;

    let raw = vec![MetadataWrapper {
        metadata: Some(PackageMetadata { wharf: None }),
    }];

    assert!(BaseConfig::try_from(raw).is_err());

    let raw = vec![MetadataWrapper {
        metadata: Some(PackageMetadata {
            wharf: Some(WharfMetadata {
                builder: Some(BaseBuilderConfig {
                    image: "another".into(),
                    env: None,
                    user: None,
                    target: None,
                    setup_commands: None,
                }),

                output: None,
                binary: None,
            }),
        }),
    }];

    assert!(BaseConfig::try_from(raw).is_err());

    let raw = vec![MetadataWrapper {
        metadata: Some(PackageMetadata {
            wharf: Some(WharfMetadata {
                output: Some(BaseOutputConfig {
                    image: "another".into(),
                    user: Some("root".into()),
                    workdir: Some("/root".into()),
                    entrypoint: None,
                    args: None,
                    env: None,
                    expose: None,
                    volumes: None,
                    labels: None,
                    stop_signal: None,
                    pre_install_commands: None,
                    post_install_commands: None,
                    copy: None,
                }),

                builder: None,
                binary: None,
            }),
        }),
    }];

    assert!(BaseConfig::try_from(raw).is_err());
}

mod schema {
    use super::*;

    #[derive(Debug, Deserialize)]
    pub(super) struct MetadataWrapper {
        pub(super) metadata: Option<PackageMetadata>,
    }

    #[derive(Debug, Deserialize)]
    pub(super) struct PackageMetadata {
        pub(super) wharf: Option<WharfMetadata>,
    }

    #[derive(Debug, Deserialize)]
    pub(super) struct WharfMetadata {
        pub(super) builder: Option<BaseBuilderConfig>,
        pub(super) output: Option<BaseOutputConfig>,
        pub(super) binary: Option<Vec<BinaryDefinition>>,
    }
}
