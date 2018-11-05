use std::collections::BTreeMap;

use crate::config::Config;
use crate::path::{translate_command_arg, translate_command_program, translate_env_value};
use crate::plan::Invocation;

#[derive(Clone, Debug, PartialEq)]
pub struct CommandDetails {
    pub env: BTreeMap<String, String>,
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Simple(CommandDetails),

    WithBuildscript {
        buildscript: CommandDetails,
        command: CommandDetails,
    },
}

impl Command {
    pub fn from_invocation(invocation: &Invocation, config: &Config) -> Self {
        Command::Simple(CommandDetails::from_invocation(invocation, config))
    }
}

impl CommandDetails {
    pub fn from_invocation(invocation: &Invocation, config: &Config) -> Self {
        CommandDetails {
            program: translate_command_program(config, &invocation.program),

            args: {
                invocation
                    .args
                    .iter()
                    .map(|arg| translate_command_arg(config, &invocation.cwd, arg))
                    .collect()
            },

            env: {
                invocation
                    .env
                    .iter()
                    .map(|(name, value)| (name.clone(), translate_env_value(config, name, value)))
                    .collect()
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::env::current_dir;
    use std::path::PathBuf;

    use cargo::util::CargoResult;
    use maplit::btreemap;
    use semver::Version;

    use super::*;

    #[test]
    fn command_details_should_be_transformed() -> CargoResult<()> {
        let config = Config::from_workspace_root("../examples/workspace")?;

        let invocation = Invocation {
            package_name: "semver".into(),
            package_version: Version::parse("0.9.0")?,

            outputs: vec![current_dir()?
                .join("target")
                .join("debug")
                .join("deps")
                .join("libsemver-f1499887dbdabbd3.rlib")],

            program: format!(
                "{}/debug/build/libnghttp2-sys/build-script-build",
                config.get_local_outdir().display()
            ),

            cwd: PathBuf::from("/registry/src/github.com-1ecc6299db9ec823/semver-0.9.0"),

            env: btreemap!{
                String::from("CARGO") => String::from("/host/path/to/cargo"),
                String::from("CARGO_MANIFEST_DIR") => String::from("/registry/src/github.com-1ecc6299db9ec823/semver-0.9.0"),
                String::from("CARGO_PKG_AUTHORS") => String::from("Steve Klabnik <steve@steveklabnik.com>:The Rust Project Developers"),
                String::from("CARGO_PKG_DESCRIPTION") => String::from("Semantic version parsing and comparison.\n"),
                String::from("CARGO_PKG_HOMEPAGE") => String::from("https://docs.rs/crate/semver/"),
                String::from("CARGO_PKG_NAME") => String::from("semver"),
                String::from("CARGO_PKG_VERSION") => String::from("0.9.0"),
                String::from("CARGO_PKG_VERSION_MAJOR") => String::from("0"),
                String::from("CARGO_PKG_VERSION_MINOR") => String::from("9"),
                String::from("CARGO_PKG_VERSION_PATCH") => String::from("0"),
                String::from("CARGO_PKG_VERSION_PRE") => String::from(""),
                String::from("LD_LIBRARY_PATH") => format!(
                    "/other/host/path:{}/debug/deps:/and/one/mode/path",
                    config.get_local_outdir().display()
                ),
            },

            args: vec![
                String::from("--crate-name"),
                String::from("semver"),
                String::from("/registry/src/github.com-1ecc6299db9ec823/semver-0.9.0/src/lib.rs"),
                String::from("--crate-type"),
                String::from("lib"),
                format!("{}/debug/deps", config.get_local_outdir().display()),
                format!(
                    "dependency={}/debug/deps",
                    config.get_local_outdir().display()
                ),
                format!(
                    "semver_parser={}/debug/deps/libsemver_parser.rlib",
                    config.get_local_outdir().display()
                ),
                format!(
                    "serde={}/debug/deps/libserde-0c8e31cadb66bdae.rlib",
                    config.get_local_outdir().display()
                ),
            ],

            ..Default::default()
        };

        let command = CommandDetails::from_invocation(&invocation, &config);

        assert_eq!(
            command.program,
            "/rust-out/debug/build/libnghttp2-sys/build-script-build"
        );

        assert_eq!(
            command.env,
            btreemap!{
                String::from("CARGO") => String::from("/usr/bin/cargo"),
                String::from("CARGO_MANIFEST_DIR") => String::from("/rust-src"),
                String::from("CARGO_PKG_AUTHORS") => String::from("Steve Klabnik <steve@steveklabnik.com>:The Rust Project Developers"),
                String::from("CARGO_PKG_DESCRIPTION") => String::from("Semantic version parsing and comparison.\n"),
                String::from("CARGO_PKG_HOMEPAGE") => String::from("https://docs.rs/crate/semver/"),
                String::from("CARGO_PKG_NAME") => String::from("semver"),
                String::from("CARGO_PKG_VERSION") => String::from("0.9.0"),
                String::from("CARGO_PKG_VERSION_MAJOR") => String::from("0"),
                String::from("CARGO_PKG_VERSION_MINOR") => String::from("9"),
                String::from("CARGO_PKG_VERSION_PATCH") => String::from("0"),
                String::from("CARGO_PKG_VERSION_PRE") => String::from(""),
                String::from("LD_LIBRARY_PATH") => String::from("/rust-out/debug/deps"),
            },
        );

        assert_eq!(
            command.args,
            vec![
                String::from("--crate-name"),
                String::from("semver"),
                String::from("/rust-src/src/lib.rs"),
                String::from("--crate-type"),
                String::from("lib"),
                String::from("/rust-out/debug/deps"),
                String::from("dependency=/rust-out/debug/deps"),
                String::from("semver_parser=/rust-out/debug/deps/libsemver_parser.rlib"),
                String::from("serde=/rust-out/debug/deps/libserde-0c8e31cadb66bdae.rlib"),
            ],
        );

        Ok(())
    }
}
