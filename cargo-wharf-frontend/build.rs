use semver::Version;
use serde::Deserialize;

fn main() {
    println!(
        "cargo:rustc-env=CONTAINER_TOOLS_REF={}",
        get_container_tools_ref()
    );
}

cfg_if::cfg_if! {
    if #[cfg(feature = "integration-testing")] {
        fn get_container_tools_ref() -> &'static str {
            "localhost:10395/denzp/cargo-container-tools:local"
        }
    } else if #[cfg(feature = "local-container-tools")] {
        fn get_container_tools_ref() -> &'static str {
            "denzp/cargo-container-tools:local"
        }
    } else {
        fn get_container_tools_ref() -> String {
            let container_tools_manifest: Manifest =
                toml::from_str(include_str!("../cargo-container-tools/Cargo.toml"))
                    .expect("Unable to parse container-tools crate manifest");

            format!(
                "denzp/cargo-container-tools:v{}",
                container_tools_manifest.package.version
            )
        }
    }
}

#[derive(Debug, Deserialize)]
struct Manifest {
    package: Package,
}

#[derive(Debug, Deserialize)]
struct Package {
    version: Version,
}
