use semver::Version;
use serde::{Deserialize, Serialize};
use toml_edit::easy::Value;

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
#[serde(tag = "type")]
pub enum Origin {
    WorkspaceRoot,
    Package { name: String, version: Version },
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Metadata {
    pub origin: Origin,
    pub metadata: Option<Value>,
}

pub mod manifest {
    use super::*;

    #[derive(Deserialize, Clone, Debug)]
    pub struct Root {
        pub workspace: Option<Workspace>,
    }

    #[derive(Deserialize, Clone, Debug)]
    pub struct Workspace {
        pub metadata: Option<toml_edit::easy::Value>,
    }
}
