use semver::Version;
use serde::Deserialize;

fn main() {
    println!(
        "cargo:rustc-env=CONTAINER_TOOLS_VERSION={}",
        get_container_tools_version()
    );
}

#[cfg(feature = "local-container-tools")]
fn get_container_tools_version() -> &'static str {
    "local"
}

#[cfg(not(feature = "local-container-tools"))]
fn get_container_tools_version() -> String {
    let container_tools_manifest: Manifest =
        toml::from_str(include_str!("../cargo-container-tools/Cargo.toml"))
            .expect("Unable to parse container-tools crate manifest");

    format!("v{}", container_tools_manifest.package.version)
}

#[derive(Debug, Deserialize)]
struct Manifest {
    package: Package,
}

#[derive(Debug, Deserialize)]
struct Package {
    version: Version,
}
