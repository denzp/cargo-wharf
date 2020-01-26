fn main() {
    println!(
        "cargo:rustc-env=CONTAINER_TOOLS_REF={}",
        get_container_tools_ref()
    );
}

cfg_if::cfg_if! {
    if #[cfg(feature = "container-tools-testing")] {
        fn get_container_tools_ref() -> &'static str {
            "localhost:10395/denzp/cargo-container-tools:local"
        }
    } else if #[cfg(feature = "container-tools-local")] {
        fn get_container_tools_ref() -> &'static str {
            "denzp/cargo-container-tools:local"
        }
    } else if #[cfg(feature = "container-tools-master")] {
        fn get_container_tools_ref() -> &'static str {
            "denzp/cargo-container-tools:master"
        }
    } else {
        fn get_container_tools_ref() -> &'static str {
            "denzp/cargo-container-tools:v0.2.0-alpha.1"
        }
    }
}
