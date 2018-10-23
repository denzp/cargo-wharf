use std::fmt;
use std::path::{Path, PathBuf};

use cargo::core::{PackageId, Resolve, Workspace};
use cargo::ops::resolve_ws;
use cargo::util::CargoResult;

use semver::Version;

pub struct Config {
    local_root: PathBuf,
    local_outdir: PathBuf,

    remote_root: PathBuf,
    remote_outdir: PathBuf,

    resolved: Resolve,
}

impl Config {
    pub fn from_cargo_workspace(ws: &Workspace) -> CargoResult<Self> {
        let (_, resolved) = resolve_ws(ws)?;

        Ok(Config {
            local_root: ws.root().into(),
            local_outdir: ws.target_dir().into_path_unlocked(),

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
            .field("local_targetdir", &self.local_outdir)
            .field("remote_root", &self.remote_root)
            .field("remote_targetdir", &self.remote_outdir)
            .finish()
    }
}
