use lazy_static::*;

use buildkit_llb::ops::source::{ImageSource, LocalSource};
use buildkit_llb::prelude::*;

lazy_static! {
    pub static ref CONTEXT: LocalSource = {
        Source::local("context")
            .custom_name("Using build context")
            .add_exclude_pattern("**/target")
    };
    pub static ref DOCKERFILE: LocalSource = {
        Source::local("dockerfile")
            .custom_name("Using build context")
            .add_exclude_pattern("**/target")
    };
}

pub const CONTEXT_PATH: &str = "/context";
pub const DOCKERFILE_PATH: &str = "/dockerfile";
pub const TARGET_PATH: &str = "/target";

pub mod tools {
    use super::*;

    lazy_static! {
        pub static ref IMAGE: ImageSource = Source::image(env!("CONTAINER_TOOLS_REF"));
    }

    pub const METADATA_COLLECTOR: &str = "/usr/local/bin/cargo-metadata-collector";
    pub const BUILDSCRIPT_CAPTURE: &str = "/usr/local/bin/cargo-buildscript-capture";
    pub const BUILDSCRIPT_APPLY: &str = "/usr/local/bin/cargo-buildscript-apply";
    pub const BUILD_PLAN: &str = "/usr/local/bin/cargo-build-plan";
    pub const TEST_RUNNER: &str = "/usr/local/bin/cargo-test-runner";
}
