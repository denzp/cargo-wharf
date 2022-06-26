use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

use anyhow::Context;
use cargo::core::compiler::BuildOutput;
use cargo::util::CargoResult;
use serde_derive::{Deserialize, Serialize};

use crate::env::RuntimeEnv;

const DEFAULT_FILE_NAME: &str = "buildscript-output.json";

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildScriptOutput {
    /// Paths to pass to rustc with the `-L` flag.
    pub library_paths: Vec<PathBuf>,

    /// Names and link kinds of libraries, suitable for the `-l` flag.
    pub library_links: Vec<String>,

    /// Linker arguments suitable to be passed to `-C link-arg=<args>`
    pub linker_args: Vec<String>,

    /// Various `--cfg` flags to pass to the compiler.
    pub cfgs: Vec<String>,

    /// Additional environment variables to run the compiler with.
    pub env: Vec<(String, String)>,

    /// Additional metadata for the build scripts of dependent crates.
    pub metadata: Vec<(String, String)>,

    /// The manifest links value.
    pub link_name: Option<String>,
}

impl BuildScriptOutput {
    pub fn serialize(&self) -> CargoResult<()> {
        let writer = BufWriter::new(
            File::create(RuntimeEnv::output_dir()?.join(DEFAULT_FILE_NAME))
                .context("Unable to open output JSON file for writing")?,
        );

        serde_json::to_writer(writer, self).context("Unable to serialize the build output")?;
        Ok(())
    }

    pub fn deserialize() -> CargoResult<Self> {
        Self::deserialize_from_dir(RuntimeEnv::output_dir()?)
    }

    pub fn deserialize_from_dir(dir: &Path) -> CargoResult<Self> {
        let reader = BufReader::new(
            File::open(dir.join(DEFAULT_FILE_NAME))
                .context("Unable to open output JSON file for reading")?,
        );

        Ok(serde_json::from_reader(reader).context("Unable to deserialize the build output")?)
    }
}

impl From<BuildOutput> for BuildScriptOutput {
    fn from(output: BuildOutput) -> Self {
        Self {
            library_paths: output.library_paths,
            library_links: output.library_links,
            // TODO: Can we really drop all link types?
            linker_args: output
                .linker_args
                .into_iter()
                .map(|(_link_type, arg)| arg)
                .collect(),
            cfgs: output.cfgs,
            env: output.env,
            metadata: output.metadata,
            link_name: RuntimeEnv::manifest_link_name().map(String::from),
        }
    }
}
