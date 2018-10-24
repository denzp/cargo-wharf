use std::collections::BTreeMap;

use crate::config::Config;
use crate::path::{translate_command_arg, translate_command_program, translate_env_value};
use crate::plan::Invocation;

#[derive(Debug, PartialEq)]
pub struct CommandDetails {
    pub env: BTreeMap<String, String>,
    pub program: String,
    pub args: Vec<String>,
}

// pub enum Command {
//     Simple(CommandDetails),
//     Complex(Vec<CommandDetails>),
// }

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

    use cargo::core::Workspace;
    use cargo::util::{CargoResult, Config as CargoConfig};
    use semver::Version;

    use super::*;
    use crate::config::Config;
    use crate::plan::Invocation;

    #[test]
    fn command_details_should_be_transformed() -> CargoResult<()> {
        let cargo_config = CargoConfig::default()?;
        let cargo_ws = Workspace::new(&current_dir()?.join("Cargo.toml"), &cargo_config)?;

        let config = Config::from_cargo_workspace(&cargo_ws)?;

        let invocation_env = vec![
            ("CARGO", String::from("/host/path/to/cargo")),
            (
                "CARGO_MANIFEST_DIR",
                String::from("/registry/src/github.com-1ecc6299db9ec823/semver-0.9.0"),
            ),
            (
                "CARGO_PKG_AUTHORS",
                String::from("Steve Klabnik <steve@steveklabnik.com>:The Rust Project Developers"),
            ),
            (
                "CARGO_PKG_DESCRIPTION",
                String::from("Semantic version parsing and comparison.\n"),
            ),
            (
                "CARGO_PKG_HOMEPAGE",
                String::from("https://docs.rs/crate/semver/"),
            ),
            ("CARGO_PKG_NAME", String::from("semver")),
            ("CARGO_PKG_VERSION", String::from("0.9.0")),
            ("CARGO_PKG_VERSION_MAJOR", String::from("0")),
            ("CARGO_PKG_VERSION_MINOR", String::from("9")),
            ("CARGO_PKG_VERSION_PATCH", String::from("0")),
            ("CARGO_PKG_VERSION_PRE", String::from("")),
            (
                "LD_LIBRARY_PATH",
                format!(
                    "{}/debug/deps:/other/host/path:/and/one/mode/path",
                    config.get_local_outdir().display()
                ),
            ),
        ];

        let invocation = Invocation {
            package_name: "semver".into(),
            package_version: Version::parse("0.9.0")?,

            outputs: vec![
                current_dir()?
                    .join("target")
                    .join("debug")
                    .join("deps")
                    .join("libsemver-f1499887dbdabbd3.rlib"),
            ],

            program: format!(
                "{}/debug/build/libnghttp2-sys/build-script-build",
                config.get_local_outdir().display()
            ),

            cwd: PathBuf::from("/registry/src/github.com-1ecc6299db9ec823/semver-0.9.0"),

            env: {
                invocation_env
                    .into_iter()
                    .map(|(name, value)| (String::from(name), value))
                    .collect()
            },

            args: vec![
                String::from("--crate-name"),
                String::from("semver"),
                String::from("/registry/src/github.com-1ecc6299db9ec823/semver-0.9.0/src/lib.rs"),
                String::from("--color"),
                String::from("always"),
                String::from("--crate-type"),
                String::from("lib"),
                String::from("--emit=dep-info,link"),
                String::from("-C"),
                String::from("debuginfo=2"),
                String::from("--cfg"),
                String::from("feature=\"default\""),
                String::from("--cfg"),
                String::from("feature=\"serde\""),
                String::from("-C"),
                String::from("metadata=f1499887dbdabbd3"),
                String::from("-C"),
                String::from("extra-filename=-f1499887dbdabbd3"),
                String::from("--out-dir"),
                format!("{}/debug/deps", config.get_local_outdir().display()),
                String::from("-L"),
                format!(
                    "dependency={}/debug/deps",
                    config.get_local_outdir().display()
                ),
                String::from("--extern"),
                format!(
                    "semver_parser={}/debug/deps/libsemver_parser-5cf6454af153e09f.rlib",
                    config.get_local_outdir().display()
                ),
                String::from("--extern"),
                format!(
                    "serde={}/debug/deps/libserde-0c8e31cadb66bdae.rlib",
                    config.get_local_outdir().display()
                ),
                String::from("--cap-lints"),
                String::from("allow"),
            ],

            ..Default::default()
        };

        let command = CommandDetails::from_invocation(&invocation, &config);

        let expected_env = vec![
            ("CARGO", "/usr/bin/cargo"),
            ("CARGO_MANIFEST_DIR", "/rust-src"),
            (
                "CARGO_PKG_AUTHORS",
                "Steve Klabnik <steve@steveklabnik.com>:The Rust Project Developers",
            ),
            (
                "CARGO_PKG_DESCRIPTION",
                "Semantic version parsing and comparison.\n",
            ),
            ("CARGO_PKG_HOMEPAGE", "https://docs.rs/crate/semver/"),
            ("CARGO_PKG_NAME", "semver"),
            ("CARGO_PKG_VERSION", "0.9.0"),
            ("CARGO_PKG_VERSION_MAJOR", "0"),
            ("CARGO_PKG_VERSION_MINOR", "9"),
            ("CARGO_PKG_VERSION_PATCH", "0"),
            ("CARGO_PKG_VERSION_PRE", ""),
            ("LD_LIBRARY_PATH", "/rust-out/debug/deps"),
        ];

        assert_eq!(
            command,
            CommandDetails {
                program: String::from("/rust-out/debug/build/libnghttp2-sys/build-script-build"),

                env: {
                    expected_env
                        .into_iter()
                        .map(|(name, value)| (String::from(name), String::from(value)))
                        .collect()
                },

                args: vec![
                    String::from("--crate-name"),
                    String::from("semver"),
                    String::from("/rust-src/src/lib.rs"),
                    String::from("--color"),
                    String::from("always"),
                    String::from("--crate-type"),
                    String::from("lib"),
                    String::from("--emit=dep-info,link"),
                    String::from("-C"),
                    String::from("debuginfo=2"),
                    String::from("--cfg"),
                    String::from("feature=\"default\""),
                    String::from("--cfg"),
                    String::from("feature=\"serde\""),
                    String::from("-C"),
                    String::from("metadata=f1499887dbdabbd3"),
                    String::from("-C"),
                    String::from("extra-filename=-f1499887dbdabbd3"),
                    String::from("--out-dir"),
                    String::from("/rust-out/debug/deps"),
                    String::from("-L"),
                    String::from("dependency=/rust-out/debug/deps",),
                    String::from("--extern"),
                    String::from(
                        "semver_parser=/rust-out/debug/deps/libsemver_parser-5cf6454af153e09f.rlib",
                    ),
                    String::from("--extern"),
                    String::from("serde=/rust-out/debug/deps/libserde-0c8e31cadb66bdae.rlib",),
                    String::from("--cap-lints"),
                    String::from("allow"),
                ]
            }
        );

        Ok(())
    }
}
