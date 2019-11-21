use std::convert::TryFrom;

use failure::{bail, Error};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(try_from = "String")]
pub enum Profile {
    ReleaseBinaries,
    DebugBinaries,

    ReleaseTests,
    DebugTests,
}

impl TryFrom<String> for Profile {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "release" | "release-binaries" => Ok(Profile::ReleaseBinaries),
            "debug" | "debug-binaries" => Ok(Profile::DebugBinaries),
            "test" | "release-test" => Ok(Profile::ReleaseTests),
            "debug-test" => Ok(Profile::DebugTests),

            other => bail!("Unknown mode: {}", other),
        }
    }
}

impl Default for Profile {
    fn default() -> Self {
        Profile::ReleaseBinaries
    }
}
