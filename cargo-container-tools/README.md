# cargo-container-tools

Container helpers collection, primarily to be used by `cargo-wharf-frontend`.
All of the executables are statically linked.

## Build plan collector
Path: `/usr/local/bin/cargo-build-plan`

Collects build plan of the crate or workspace as a JSON.

## Crate metadata collector
Path: `/usr/local/bin/cargo-metadata-collector`

Collects `[package.metadata]` and *(unofficial)* `[workspace.metadata]` of the crate or workspace as a JSON.

## Test runner
Path: `/usr/local/bin/cargo-test-runner`

Runs tests just like `cargo test`.

## Build script output capture
Path: `/usr/local/bin/cargo-buildscript-capture`

Runs and captures build script output for further usage.

## Build script output apply
Path: `/usr/local/bin/cargo-buildscript-apply`

Applies a captured build script output into subsequent `rustc` calls.
