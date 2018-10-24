use std::env::current_dir;
use std::fmt;
use std::path::{Path, PathBuf};

use cargo::core::{PackageId, Resolve, Shell, Workspace};
use cargo::ops::resolve_ws;
use cargo::util::{homedir, CargoResult, Config as CargoConfig};

use failure::format_err;
use path_absolutize::Absolutize;
use semver::Version;

pub struct Config {
    local_root: PathBuf,
    local_outdir: PathBuf,

    remote_root: PathBuf,
    remote_outdir: PathBuf,

    resolved: Resolve,
}

impl Config {
    pub fn from_workspace_root<P: AsRef<Path>>(root: P) -> CargoResult<Self> {
        let crate_path: PathBuf = if root.as_ref().is_absolute() {
            PathBuf::from(root.as_ref()).absolutize()?
        } else {
            current_dir()?.join(root).absolutize()?
        };

        let homedir = homedir(&crate_path).ok_or_else(|| {
            format_err!(
                "Cargo couldn't find your home directory. \
                 This probably means that $HOME was not set."
            )
        })?;

        let config = CargoConfig::new(Shell::new(), crate_path, homedir);
        let workspace = Workspace::new(&config.cwd().join("Cargo.toml"), &config)?;

        Self::from_cargo_workspace(&workspace)
    }

    pub fn from_cargo_workspace(workspace: &Workspace) -> CargoResult<Self> {
        let (_, resolved) = resolve_ws(workspace)?;

        Ok(Config {
            local_root: workspace.root().into(),
            local_outdir: workspace.target_dir().into_path_unlocked(),

            remote_root: PathBuf::from("/rust-src"),
            remote_outdir: PathBuf::from("/rust-out"),

            resolved,
        })
    }

    pub fn resolve(&self, name: &str, version: &Version) -> CargoResult<&PackageId> {
        self.resolved.query(&format!("{}:{}", name, version))
    }

    pub fn get_local_outdir(&self) -> &Path {
        &self.local_outdir
    }

    pub fn get_local_root(&self) -> &Path {
        &self.local_root
    }

    pub fn get_container_outdir(&self) -> &Path {
        &self.remote_outdir
    }

    pub fn get_container_root(&self) -> &Path {
        &self.remote_root
    }
}

impl fmt::Debug for Config {
    /// For easier interpretation, we omit `resolved` field.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Config")
            .field("local_root", &self.local_root)
            .field("local_targetdir", &self.local_outdir)
            .field("remote_root", &self.remote_root)
            .field("remote_targetdir", &self.remote_outdir)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_translate_target_paths() -> CargoResult<()> {
        let config = Config::from_workspace_root("../examples/workspace")?;

        assert_eq!(
            config.get_local_outdir(),
            current_dir()?
                .join("../examples/workspace/target")
                .absolutize()?
        );

        assert_eq!(
            config.get_local_root(),
            current_dir()?.join("../examples/workspace").absolutize()?
        );

        assert_eq!(config.get_container_outdir(), PathBuf::from("/rust-out"));
        assert_eq!(config.get_container_root(), PathBuf::from("/rust-src"));

        Ok(())
    }
}
