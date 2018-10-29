use cargo::util::CargoResult;
use semver::Version;
use serde_derive::Deserialize;

use crate::path::TargetPath;

pub fn container_tools_version() -> CargoResult<Version> {
    let tools_manifest_str = include_str!("../../../cargo-container-tools/Cargo.toml");
    let tools_manifest: TomlManifest = toml::from_str(tools_manifest_str)?;

    Ok(tools_manifest.package.version)
}

pub fn find_unique_base_paths<'b>(
    paths: impl Iterator<Item = &'b TargetPath>,
) -> impl Iterator<Item = TargetPath> {
    let mut input_paths = paths.map(|item| item.parent().unwrap()).collect::<Vec<_>>();
    let mut output_paths = vec![];

    input_paths.sort();

    let remaining = input_paths.iter().fold(None, |last, current| match last {
        Some(last) => {
            if current.starts_with(&last) {
                Some(current)
            } else {
                output_paths.push(unsafe { TargetPath::from_path(last) });
                Some(current)
            }
        }

        None => Some(current),
    });

    if let Some(remaining) = remaining {
        output_paths.push(unsafe { TargetPath::from_path(remaining) });
    }

    output_paths.into_iter()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct TomlManifest {
    package: TomlProject,
}

#[derive(Debug, Deserialize)]
struct TomlProject {
    version: semver::Version,
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn it_should_provide_container_tools_version() {
        assert_eq!(container_tools_version().unwrap().to_string(), "0.1.0");
    }

    #[test]
    fn it_should_group_target_paths() {
        let paths = unsafe {
            vec![
                TargetPath::from_path("/rust-out/debug/lib.rlib"),
                TargetPath::from_path("/rust-out/root.rlib"),
                TargetPath::from_path("/rust-out/debug/nested/path.log"),
                TargetPath::from_path("/rust-out/release/another/lib.rlib"),
                TargetPath::from_path("/rust-out/debug/super/nested/path.log"),
                TargetPath::from_path("/rust-out/release/nested/lib.rlib"),
            ]
        };

        assert_eq!(
            find_unique_base_paths(paths.iter()).collect::<Vec<_>>(),
            unsafe {
                vec![
                    TargetPath::from_path("/rust-out/debug/nested"),
                    TargetPath::from_path("/rust-out/debug/super/nested"),
                    TargetPath::from_path("/rust-out/release/another"),
                    TargetPath::from_path("/rust-out/release/nested"),
                ]
            }
        );
    }
}
