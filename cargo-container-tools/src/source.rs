use cargo::core::{GitReference, Resolve};
use cargo::util::errors::{internal, CargoResult};

use serde_derive::Serialize;

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceKind {
    RegistryUrl(String),
    Local,
    GitCheckout {
        repo: String,
        reference: Option<String>,
    },
}

impl SourceKind {
    pub fn find(resolve: &Resolve, name_and_version: &str) -> CargoResult<Self> {
        let package_id = resolve.query(&name_and_version)?;
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
                    reference: Some(rev.into()),
                },

                None => SourceKind::GitCheckout {
                    repo: source_id.url().to_string(),
                    reference: source_id.git_reference().map(|reference| match reference {
                        GitReference::Branch(ref b) => b.into(),
                        GitReference::Tag(ref s) => s.into(),
                        GitReference::Rev(ref s) => s.into(),
                    }),
                },
            });
        }

        if source_id.is_path() {
            return Ok(SourceKind::Local);
        }

        Err(internal(format!("Unknown source: {:?}", source_id)))
    }
}

#[cfg(test)]
mod tests {
    // use std::path::PathBuf;

    use super::*;
    use cargo::core::Workspace;
    use cargo::ops::resolve_ws;
    use cargo::Config;

    #[test]
    fn registry_source() -> CargoResult<()> {
        let resolve = resolve_workspace_example()?;

        assert_eq!(
            SourceKind::find(&resolve, "bitflags:1.2.1")?,
            SourceKind::RegistryUrl(String::from(
                "https://crates.io/api/v1/crates/bitflags/1.2.1/download"
            )),
        );

        Ok(())
    }

    #[test]
    fn context_source() -> CargoResult<()> {
        let resolve = resolve_workspace_example()?;

        assert_eq!(
            SourceKind::find(&resolve, "binary-1:0.1.0")?,
            SourceKind::Local,
        );

        Ok(())
    }

    #[test]
    fn git_source() -> CargoResult<()> {
        let resolve = resolve_workspace_example()?;

        assert_eq!(
            SourceKind::find(&resolve, "log:0.4.0")?,
            SourceKind::GitCheckout {
                repo: String::from("https://github.com/rust-lang-nursery/log.git"),
                reference: Some(String::from("bf40d1f563cf3eef63233d935ce56f2198b381d3")),
            },
        );

        Ok(())
    }

    fn resolve_workspace_example() -> CargoResult<Resolve> {
        let mut config = Config::default()?;
        config.configure(0, None, &None, false, true, false, &None, &[])?;

        let manifest_path = {
            std::env::current_dir()
                .unwrap()
                .parent()
                .unwrap()
                .join("examples/workspace/Cargo.toml")
        };

        let ws = Workspace::new(&manifest_path, &config)?;

        resolve_ws(&ws).map(|(_, resolved)| resolved)
    }
}
