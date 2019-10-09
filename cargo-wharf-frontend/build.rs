use semver::Version;
use serde::Deserialize;

fn main() {
    let container_tools_manifest: Manifest =
        toml::from_str(include_str!("../cargo-container-tools/Cargo.toml"))
            .expect("Unable to parse container-tools crate manifest");

    println!(
        "cargo:rustc-env=CONTAINER_TOOLS_VERSION={}",
        container_tools_manifest.package.version
    );
}

#[derive(Debug, Deserialize)]
struct Manifest {
    package: Package,
}

#[derive(Debug, Deserialize)]
struct Package {
    version: Version,
}
