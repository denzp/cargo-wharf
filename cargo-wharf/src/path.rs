use std::ops::Deref;
use std::path::{Path, PathBuf};

use cargo::util::{CargoError, CargoResult};
use pathdiff::diff_paths;

use crate::config::Config;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TargetPath(PathBuf);

impl TargetPath {
    #[cfg(test)]
    pub unsafe fn from_path<P: AsRef<Path>>(path: P) -> Self {
        TargetPath(path.as_ref().into())
    }

    pub fn with_config(config: &Config, path: &Path) -> CargoResult<TargetPath> {
        let relative_path = {
            path.strip_prefix(config.get_local_outdir())
                .map_err(CargoError::from)
                .map_err(|error| error.context("The given path is not in `target` dir..."))?
        };

        let target_path = config.get_container_outdir().join(relative_path);

        Ok(TargetPath(target_path))
    }

    pub fn as_relative_for(&self, other: &Self) -> PathBuf {
        diff_paths(&self.0, &other.0.parent().unwrap()).unwrap()
    }
}

impl Deref for TargetPath {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub fn translate_command_program(config: &Config, program: &str) -> String {
    translate_pathes(config, None, program)
}

pub fn translate_command_arg(config: &Config, cwd: &Path, arg: &str) -> String {
    translate_pathes(config, Some(cwd), arg)
}

pub fn translate_env_value(config: &Config, name: &str, value: &str) -> String {
    match name {
        "CARGO" => String::from("/usr/bin/cargo"),
        "CARGO_MANIFEST_DIR" => config.get_container_root().display().to_string(),

        "OUT_DIR" => translate_pathes(config, None, value),

        "LD_LIBRARY_PATH" => value
            .split(':')
            .filter_map(|part| {
                let transformed = translate_pathes(config, None, part);

                if transformed == part {
                    None
                } else {
                    Some(transformed)
                }
            })
            .collect::<Vec<_>>()
            .join(":"),

        _ => value.into(),
    }
}

fn translate_pathes(config: &Config, cwd: Option<&Path>, input: &str) -> String {
    let mut result = input.replace(
        config.get_local_outdir().display().to_string().as_str(),
        config.get_container_outdir().display().to_string().as_str(),
    );

    if let Some(cwd) = cwd {
        result = result.replace(
            cwd.display().to_string().as_str(),
            config.get_container_root().display().to_string().as_str(),
        );
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_translate_target_paths() -> CargoResult<()> {
        let config = Config::from_workspace_root("../examples/workspace")?;

        unsafe {
            assert_eq!(
                TargetPath::with_config(
                    &config,
                    &config
                        .get_local_outdir()
                        .join("debug")
                        .join("library-1.rlib")
                )?,
                TargetPath::from_path(
                    &PathBuf::from("/rust-out")
                        .join("debug")
                        .join("library-1.rlib")
                ),
            );
        }

        unsafe {
            assert_eq!(
                TargetPath::with_config(
                    &config,
                    &config
                        .get_local_outdir()
                        .join("debug")
                        .join("deps")
                        .join("binary-2")
                )?,
                TargetPath::from_path(
                    &PathBuf::from("/rust-out")
                        .join("debug")
                        .join("deps")
                        .join("binary-2")
                ),
            );
        }

        Ok(())
    }

    #[test]
    fn it_should_panic_when_path_is_not_target() -> CargoResult<()> {
        let config = Config::from_workspace_root("../examples/workspace")?;
        let path = config.get_local_root().join("src/lib.rs");

        TargetPath::with_config(&config, &path).expect_err("should panic!");

        Ok(())
    }

    #[test]
    fn it_should_provide_relative_paths() -> CargoResult<()> {
        let path1 = unsafe { TargetPath::from_path("/rust-out/debug/binary") };
        let path2 = unsafe { TargetPath::from_path("/rust-out/debug/deps/binary-2") };

        assert_eq!(path1.as_relative_for(&path2), PathBuf::from("../binary"));
        assert_eq!(
            path2.as_relative_for(&path1),
            PathBuf::from("deps/binary-2")
        );

        Ok(())
    }

    #[test]
    fn it_should_translate_arguments() -> CargoResult<()> {
        let config = Config::from_workspace_root("../examples/workspace")?;
        let cwd = PathBuf::from("/registry/src/github.com-1ecc6299db9ec823/semver-0.9.0");

        assert_eq!(
            translate_command_arg(&config, &cwd, "--crate-name"),
            String::from("--crate-name")
        );

        assert_eq!(
            translate_command_arg(
                &config,
                &cwd,
                "--src=/registry/src/github.com-1ecc6299db9ec823/semver-0.9.0/src/lib.rs"
            ),
            String::from("--src=/rust-src/src/lib.rs")
        );

        assert_eq!(
            translate_command_arg(
                &config,
                &cwd,
                &format!(
                    "--extern={}",
                    config.get_local_outdir().join("debug/lib.rlib").display()
                )
            ),
            String::from("--extern=/rust-out/debug/lib.rlib")
        );

        Ok(())
    }

    #[test]
    fn it_should_translate_env() -> CargoResult<()> {
        let config = Config::from_workspace_root("../examples/workspace")?;

        assert_eq!(
            translate_env_value(&config, "CARGO", "/any/cargo/path"),
            String::from("/usr/bin/cargo")
        );

        assert_eq!(
            translate_env_value(&config, "CARGO_MANIFEST_DIR", "/any/manifest/path"),
            String::from("/rust-src")
        );

        assert_eq!(
            translate_env_value(
                &config,
                "LD_LIBRARY_PATH",
                &format!(
                    "{}/debug/deps:/other/host/path:/and/one/mode/path",
                    config.get_local_outdir().display()
                )
            ),
            String::from("/rust-out/debug/deps")
        );

        assert_eq!(
            translate_env_value(
                &config,
                "OUT_DIR",
                &format!(
                    "{}/debug/build/libnghttp2-sys-d7e2e844533c088a/out",
                    config.get_local_outdir().display()
                )
            ),
            String::from("/rust-out/debug/build/libnghttp2-sys-d7e2e844533c088a/out")
        );

        Ok(())
    }
}
