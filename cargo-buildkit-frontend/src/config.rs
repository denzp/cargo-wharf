use std::convert::TryFrom;
use std::path::PathBuf;

use failure::{bail, format_err, Error, ResultExt};
use log::*;
use serde::{Deserialize, Serialize};

use buildkit_frontend::Bridge;
use buildkit_llb::prelude::*;

use crate::image::{RustDockerImage, TOOLS_IMAGE};
use crate::CONTEXT_PATH;

const METADATA_COLLECTOR_EXEC: &str = "/usr/local/bin/cargo-metadata-collector";
const OUTPUT_LAYER_PATH: &str = "/output";
const OUTPUT_NAME: &str = "build-plan.json";

#[derive(Debug, Serialize)]
pub struct Config {
    builder: BuilderConfig,
    builder_image: RustDockerImage,

    output: OutputConfig,

    binaries: Vec<BinaryDefinition>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct BuilderConfig {
    image: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct OutputConfig {
    image: String,
    user: Option<String>,
    workdir: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct BinaryDefinition {
    name: String,
    destination: PathBuf,
}

impl Config {
    pub async fn analyse(bridge: &mut Bridge) -> Result<Self, Error> {
        let context = {
            Source::local("context")
                .custom_name("Using context")
                .add_exclude_pattern("**/target")
        };

        let command = {
            Command::run(METADATA_COLLECTOR_EXEC)
                .args(&[
                    "--manifest-path",
                    &PathBuf::from(CONTEXT_PATH)
                        .join("Cargo.toml")
                        .to_string_lossy(),
                ])
                .args(&[
                    "--output",
                    &PathBuf::from(OUTPUT_LAYER_PATH)
                        .join(OUTPUT_NAME)
                        .to_string_lossy(),
                ])
                .cwd(CONTEXT_PATH)
                .mount(Mount::Layer(OutputIdx(0), TOOLS_IMAGE.output(), "/"))
                .mount(Mount::ReadOnlyLayer(context.output(), CONTEXT_PATH))
                .mount(Mount::Scratch(OutputIdx(1), OUTPUT_LAYER_PATH))
                .custom_name("Collecting configuration metadata")
        };

        let metadata_layer = {
            bridge
                .solve(Terminal::with(command.output(1)))
                .await
                .context("Unable to collect metadata")?
        };

        let metadata = {
            bridge
                .read_file(&metadata_layer, OUTPUT_NAME, None)
                .await
                .context("Unable to read metadata output")?
        };

        let raw_metadata: Vec<schema::MetadataWrapper> = {
            serde_json::from_slice(&metadata).context("Unable to parse configuration metadata")?
        };

        debug!("raw metadata: {:?}", raw_metadata);

        let base = ConfigBase::try_from(raw_metadata)?;

        let builder_source =
            Source::image(&base.builder.image).with_resolve_mode(ResolveMode::PreferLocal);

        let builder_image = {
            RustDockerImage::analyse(bridge, builder_source)
                .await
                .context("Unable to analyse Rust builder image")?
        };

        Ok(Self {
            builder: base.builder,
            builder_image,

            output: base.output,
            binaries: base.binaries,
        })
    }

    pub fn builder_image(&self) -> &RustDockerImage {
        &self.builder_image
    }
}

type ConfigCtx = (
    Option<BuilderConfig>,
    Option<OutputConfig>,
    Vec<BinaryDefinition>,
);

#[derive(Debug, PartialEq)]
struct ConfigBase {
    builder: BuilderConfig,
    output: OutputConfig,
    binaries: Vec<BinaryDefinition>,
}

impl TryFrom<Vec<schema::MetadataWrapper>> for ConfigBase {
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
            },
            output: OutputConfig {
                image: "alpine:latest".into(),
                user: Some("root".into()),
                workdir: Some("/root".into()),
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
                    }),
                    output: Some(OutputConfig {
                        image: "alpine:latest".into(),
                        user: Some("root".into()),
                        workdir: Some("/root".into()),
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
                    }),
                    output: Some(OutputConfig {
                        image: "alpine:latest".into(),
                        user: Some("root".into()),
                        workdir: Some("/root".into()),
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
