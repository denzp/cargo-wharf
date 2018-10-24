use std::path::PathBuf;

use cargo::core::GitReference;
use cargo::util::errors::{internal, CargoResult};

use crate::config::Config;
use crate::plan::Invocation;

#[derive(Debug, PartialEq)]
pub enum SourceKind {
    RegistryUrl(String),
    ContextPath,
    GitCheckout {
        repo: String,
        reference: GitReference,
    },
}

impl SourceKind {
    pub fn from_invocation(invocation: &Invocation, config: &Config) -> CargoResult<Self> {
        let package_id = config.resolve(&invocation.package_name, &invocation.package_version)?;
        let source_id = package_id.source_id();

        if source_id.is_registry() {
            return Ok(SourceKind::RegistryUrl(format!(
                "https://crates.io/api/v1/crates/{}/{}/download",
                package_id.name(),
                package_id.version()
            )));
        }

        if source_id.is_git() {
            return Ok(match source_id.precise() {
                Some(rev) => SourceKind::GitCheckout {
                    repo: source_id.url().to_string(),
                    reference: GitReference::Rev(rev.into()),
                },

                None => SourceKind::GitCheckout {
                    repo: source_id.url().to_string(),
                    reference: source_id.git_reference().cloned().unwrap(),
                },
            });
        }

        if source_id.is_path() {
            let path = PathBuf::from(&source_id.url().to_string()[7..]);

            // TODO(denzp): return human readable error when path is on in workspace
            path.strip_prefix(config.get_local_root())?;

            return Ok(SourceKind::ContextPath);
        }

        Err(internal(format!("Unknown source: {:?}", source_id)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use semver::Version;

    #[test]
    fn registry_source() -> CargoResult<()> {
        let config = Config::from_workspace_root("../examples/workspace")?;
        let invocation = Invocation {
            package_name: "clap".into(),
            package_version: Version::parse("2.32.0")?,
            ..Default::default()
        };

        let source = SourceKind::from_invocation(&invocation, &config)?;

        assert_eq!(
            source,
            SourceKind::RegistryUrl(String::from(
                "https://crates.io/api/v1/crates/clap/2.32.0/download"
            ))
        );

        Ok(())
    }

    #[test]
    fn path_source() -> CargoResult<()> {
        let config = Config::from_workspace_root("../examples/workspace")?;
        let invocation = Invocation {
            package_name: "binary-1".into(),
            package_version: Version::parse("0.1.0")?,
            ..Default::default()
        };

        let source = SourceKind::from_invocation(&invocation, &config)?;

        assert_eq!(source, SourceKind::ContextPath);

        Ok(())
    }

    #[test]
    fn git_source() -> CargoResult<()> {
        let config = Config::from_workspace_root("../examples/workspace")?;
        let invocation = Invocation {
            package_name: "log".into(),
            package_version: Version::parse("0.4.5")?,
            ..Default::default()
        };

        let source = SourceKind::from_invocation(&invocation, &config)?;

        assert_eq!(
            source,
            SourceKind::GitCheckout {
                repo: String::from("https://github.com/rust-lang-nursery/log.git"),
                reference: GitReference::Rev(String::from(
                    "5f3cb9e144d8fd41362b6a1c9e1c6192e232a1eb"
                )),
            }
        );

        Ok(())
    }
}
