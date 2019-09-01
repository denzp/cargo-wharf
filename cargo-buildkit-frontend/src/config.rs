use std::convert::TryFrom;
use std::path::PathBuf;

use failure::{bail, format_err, Error, ResultExt};
use log::*;
use serde::{Deserialize, Serialize};

use buildkit_frontend::Bridge;
use buildkit_llb::ops::source::ImageSource;
use buildkit_llb::prelude::*;

use crate::image::TOOLS_IMAGE;
use crate::CONTEXT_PATH;

const METADATA_COLLECTOR_EXEC: &str = "/usr/local/bin/cargo-metadata-collector";
const OUTPUT_LAYER_PATH: &str = "/output";
const OUTPUT_NAME: &str = "build-plan.json";

#[derive(Debug, Serialize, PartialEq)]
pub struct Config {
    config: DefaultConfig,
    binaries: Vec<BinaryDefinition>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct DefaultConfig {
    builder_image: String,
    release_image: String,

    default_user: Option<String>,
    default_workdir: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct BinaryDefinition {
    name: String,
    destination: PathBuf,

    user: Option<String>,
    workdir: Option<PathBuf>,
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

        Self::try_from(raw_metadata)
    }

    pub fn builder_image(&self) -> ImageSource {
        Source::image(&self.config.builder_image).with_resolve_mode(ResolveMode::PreferLocal)
    }
}

impl TryFrom<Vec<schema::MetadataWrapper>> for Config {
    type Error = Error;

    fn try_from(raw: Vec<schema::MetadataWrapper>) -> Result<Self, Error> {
        let (config, binaries) = {
            raw.into_iter()
                .filter_map(|item| item.metadata)
                .filter_map(|item| item.wharf)
                .try_fold((None, vec![]), |(config, mut binaries), metadata| {
                    if let Some(mut incoming) = metadata.binary {
                        binaries.append(&mut incoming);
                    }

                    Ok(match (config, metadata.config) {
                        (config, None) => (config, binaries),
                        (None, Some(incoming)) => (Some(incoming), binaries),

                        (Some(_), Some(_)) => {
                            bail!("Found duplicated 'wharf.config' section");
                        }
                    })
                })?
        };

        Ok(Self {
            config: config.ok_or_else(|| format_err!("Missing 'wharf.config' section"))?,
            binaries,
        })
    }
}

#[test]
fn transformation() {
    use schema::*;

    let raw = vec![
        MetadataWrapper {
            metadata: Some(PackageMetadata {
                wharf: Some(WharfMetadata {
                    config: Some(DefaultConfig {
                        builder_image: "rust:latest".into(),
                        release_image: "alpine:latest".into(),
                        default_user: Some("root".into()),
                        default_workdir: Some("/root".into()),
                    }),
                    binary: None,
                }),
            }),
        },
        MetadataWrapper {
            metadata: Some(PackageMetadata {
                wharf: Some(WharfMetadata {
                    config: None,
                    binary: Some(vec![BinaryDefinition {
                        name: "binary-1".into(),
                        destination: "/bin/binary-1".into(),
                        user: Some("binary-1-user".into()),
                        workdir: None,
                    }]),
                }),
            }),
        },
        MetadataWrapper { metadata: None },
        MetadataWrapper {
            metadata: Some(PackageMetadata {
                wharf: Some(WharfMetadata {
                    config: None,
                    binary: Some(vec![BinaryDefinition {
                        name: "binary-2".into(),
                        destination: "/usr/local/bin/binary-2".into(),
                        user: None,
                        workdir: Some("/".into()),
                    }]),
                }),
            }),
        },
    ];

    assert_eq!(
        Config::try_from(raw).unwrap(),
        Config {
            config: DefaultConfig {
                builder_image: "rust:latest".into(),
                release_image: "alpine:latest".into(),
                default_user: Some("root".into()),
                default_workdir: Some("/root".into()),
            },
            binaries: vec![
                BinaryDefinition {
                    name: "binary-1".into(),
                    destination: "/bin/binary-1".into(),
                    user: Some("binary-1-user".into()),
                    workdir: None,
                },
                BinaryDefinition {
                    name: "binary-2".into(),
                    destination: "/usr/local/bin/binary-2".into(),
                    user: None,
                    workdir: Some("/".into()),
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
                    config: Some(DefaultConfig {
                        builder_image: "rust:latest".into(),
                        release_image: "alpine:latest".into(),
                        default_user: Some("root".into()),
                        default_workdir: Some("/root".into()),
                    }),
                    binary: None,
                }),
            }),
        },
        MetadataWrapper {
            metadata: Some(PackageMetadata {
                wharf: Some(WharfMetadata {
                    config: Some(DefaultConfig {
                        builder_image: "another".into(),
                        release_image: "another".into(),
                        default_user: None,
                        default_workdir: None,
                    }),
                    binary: None,
                }),
            }),
        },
    ];

    assert!(Config::try_from(raw).is_err());
}

#[test]
fn missing_config() {
    use schema::*;

    let raw = vec![MetadataWrapper {
        metadata: Some(PackageMetadata { wharf: None }),
    }];

    assert!(Config::try_from(raw).is_err());
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
        pub(super) config: Option<DefaultConfig>,
        pub(super) binary: Option<Vec<BinaryDefinition>>,
    }
}
