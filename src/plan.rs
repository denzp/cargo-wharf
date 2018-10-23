use std::collections::BTreeMap;
use std::io::stdin;
use std::path::PathBuf;

use cargo::util::CargoResult;

use semver::Version;
use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Invocation {
    pub package_name: String,
    pub package_version: Version,
    pub target_kind: Vec<TargetKind>,
    pub deps: Vec<usize>,
    pub outputs: Vec<PathBuf>,
    pub links: BTreeMap<PathBuf, PathBuf>,
    pub program: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub cwd: PathBuf, // TODO(denzp): should this really be an "Option<PathBuf>" like in Cargo?
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum TargetKind {
    Lib,
    Bin,
    Test,
    CustomBuild,
    ProcMacro,
}

#[derive(Debug, Deserialize)]
struct SerializedBuildPlan {
    invocations: Vec<Invocation>,
}

pub fn invocations_from_stdio() -> CargoResult<Vec<Invocation>> {
    Ok(serde_json::from_reader::<_, SerializedBuildPlan>(stdin())?.invocations)
}

impl Default for Invocation {
    fn default() -> Self {
        Self {
            package_name: Default::default(),
            package_version: Version::parse("0.0.0").unwrap(),
            target_kind: Default::default(),
            deps: Default::default(),
            outputs: Default::default(),
            links: Default::default(),
            program: Default::default(),
            args: Default::default(),
            env: Default::default(),
            cwd: Default::default(),
        }
    }
}
