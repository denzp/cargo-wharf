# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0-alpha.1] - 2019-12-01
### Changed
- Switched to stable Rust channel.
- Cargo updated to `v0.40.0`.

## [0.2.0-alpha.0] - 2019-11-07
### Added
- Build plan generation tool.
- Test runner tool.
- Metadata collector tool.

### Changed
- The crate was shaped and redesigned almost from scratch to faciliate needs of `cargo-wharf-frontend`.
- Buildscript helper was rewritten from scratch as 2 components: capturing and applying.

### Removed
- Temporarily removed `cargo-ldd` helper to check dynamic dependencies.

## [0.1.0] - 2018-11-07
### Added
- Basic tools needed to support legacy approach with dynamically-generated Dockerfile.

[Unreleased]: https://github.com/denzp/cargo-wharf/compare/cargo-container-tools-v0.2.0-alpha.1...HEAD
[0.2.0-alpha.1]: https://github.com/denzp/cargo-wharf/compare/cargo-container-tools-v0.2.0-alpha.0...cargo-container-tools-v0.2.0-alpha.1
[0.2.0-alpha.0]: https://github.com/denzp/cargo-wharf/compare/legacy-dockerfile...cargo-container-tools-v0.2.0-alpha.0
[0.1.0]: https://github.com/denzp/cargo-wharf/releases/tag/legacy-dockerfile
