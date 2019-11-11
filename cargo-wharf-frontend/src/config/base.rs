use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::path::PathBuf;

use failure::{bail, format_err, Error};
use log::*;
use serde::{Deserialize, Serialize};

use buildkit_frontend::oci::{ExposedPort, Signal};
use buildkit_llb::ops::source::ImageSource;
use buildkit_llb::prelude::*;

#[derive(Debug, PartialEq, Deserialize)]
#[serde(try_from = "Vec<schema::MetadataWrapper>")]
pub struct ConfigBase {
    pub builder: BuilderConfig,
    pub output: OutputConfig,
    pub binaries: Vec<BinaryDefinition>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct BuilderConfig {
    pub image: String,
    pub user: Option<String>,
    pub env: Option<BTreeMap<String, String>>,
    pub target: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct OutputConfig {
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
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct BinaryDefinition {
    pub name: String,
    pub destination: PathBuf,
}

impl TryFrom<Vec<schema::MetadataWrapper>> for ConfigBase {
    type Error = Error;

    fn try_from(raw: Vec<schema::MetadataWrapper>) -> Result<Self, Self::Error> {
        debug!("raw metadata: {:?}", raw);

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

impl BuilderConfig {
    pub fn source(&self) -> ImageSource {
        Source::image(&self.image).with_resolve_mode(ResolveMode::PreferLocal)
    }
}

impl OutputConfig {
    pub fn source(&self) -> ImageSource {
        Source::image(&self.image).with_resolve_mode(ResolveMode::PreferLocal)
    }
}

type ConfigCtx = (
    Option<BuilderConfig>,
    Option<OutputConfig>,
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
                    output: Some(OutputConfig {
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
                    }),

                    builder: None,
                    binary: None,
                }),
            }),
        },
        MetadataWrapper {
            metadata: Some(PackageMetadata {
                wharf: Some(WharfMetadata {
                    builder: Some(BuilderConfig {
                        image: "rust:latest".into(),
                        env: None,
                        user: None,
                        target: None,
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
        ConfigBase::try_from(raw).unwrap(),
        ConfigBase {
            builder: BuilderConfig {
                image: "rust:latest".into(),
                env: None,
                user: None,
                target: None,
            },
            output: OutputConfig {
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
            ]
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
                    builder: Some(BuilderConfig {
                        image: "rust:latest".into(),
                        env: None,
                        user: None,
                        target: None,
                    }),
                    output: Some(OutputConfig {
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
                    }),

                    binary: None,
                }),
            }),
        },
        MetadataWrapper {
            metadata: Some(PackageMetadata {
                wharf: Some(WharfMetadata {
                    builder: Some(BuilderConfig {
                        image: "rust:latest".into(),
                        env: None,
                        user: None,
                        target: None,
                    }),

                    output: None,
                    binary: None,
                }),
            }),
        },
    ];

    assert!(ConfigBase::try_from(raw).is_err());

    let raw = vec![
        MetadataWrapper {
            metadata: Some(PackageMetadata {
                wharf: Some(WharfMetadata {
                    builder: Some(BuilderConfig {
                        image: "rust:latest".into(),
                        env: None,
                        user: None,
                        target: None,
                    }),
                    output: Some(OutputConfig {
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
                    }),

                    binary: None,
                }),
            }),
        },
        MetadataWrapper {
            metadata: Some(PackageMetadata {
                wharf: Some(WharfMetadata {
                    output: Some(OutputConfig {
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
                    }),

                    builder: None,
                    binary: None,
                }),
            }),
        },
    ];

    assert!(ConfigBase::try_from(raw).is_err());
}

#[test]
fn missing_config() {
    use schema::*;

    let raw = vec![MetadataWrapper {
        metadata: Some(PackageMetadata { wharf: None }),
    }];

    assert!(ConfigBase::try_from(raw).is_err());

    let raw = vec![MetadataWrapper {
        metadata: Some(PackageMetadata {
            wharf: Some(WharfMetadata {
                builder: Some(BuilderConfig {
                    image: "another".into(),
                    env: None,
                    user: None,
                    target: None,
                }),

                output: None,
                binary: None,
            }),
        }),
    }];

    assert!(ConfigBase::try_from(raw).is_err());

    let raw = vec![MetadataWrapper {
        metadata: Some(PackageMetadata {
            wharf: Some(WharfMetadata {
                output: Some(OutputConfig {
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
                }),

                builder: None,
                binary: None,
            }),
        }),
    }];

    assert!(ConfigBase::try_from(raw).is_err());
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
        pub(super) builder: Option<BuilderConfig>,
        pub(super) output: Option<OutputConfig>,
        pub(super) binary: Option<Vec<BinaryDefinition>>,
    }
}
