use std::env;
use std::path::Path;

use anyhow::Error;
use cargo::util::CargoResult;
use lazy_static::*;

lazy_static! {
    static ref OUT_DIR: Option<String> = env::var("OUT_DIR").ok();
    static ref CARGO_PKG_NAME: Option<String> = env::var("CARGO_PKG_NAME").ok();
    static ref CARGO_MANIFEST_LINKS: Option<String> = env::var("CARGO_MANIFEST_LINKS").ok();
}

pub struct RuntimeEnv;

impl RuntimeEnv {
    pub fn output_dir() -> CargoResult<&'static Path> {
        OUT_DIR
            .as_ref()
            .map(|value| Path::new(value))
            .ok_or_else(|| Error::msg("unable to find OUT_DIR env variable"))
    }

    pub fn package_name() -> CargoResult<&'static str> {
        CARGO_PKG_NAME
            .as_ref()
            .map(|value| value.as_str())
            .ok_or_else(|| Error::msg("unable to find CARGO_PKG_NAME env variable"))
    }

    pub fn manifest_link_name() -> Option<&'static str> {
        CARGO_MANIFEST_LINKS.as_ref().map(|value| value.as_str())
    }
}
