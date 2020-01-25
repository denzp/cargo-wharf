# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Changed
- Use host SSH Agent when available (exactly, when `--ssh=default` flag is passed for `docker build`).

## [0.1.0-alpha.1] - 2019-12-01
### Added
- Advanced output image metadata (Dockerfile's `VOLUME`, `EXPOSE`, `LABEL`, `STOPSIGNAL`).
- Custom image setup commands (Dockerfile's `RUN`).
- Support `staticlib` dependencies.

### Changed
- README and usage guide.
- Switched to stable Rust channel.

## [0.1.0-alpha.0] - 2019-11-07
### Added
- The frontend itself: first public release.

[Unreleased]: https://github.com/denzp/cargo-wharf/compare/cargo-wharf-frontend-v0.1.0-alpha.1...HEAD
[0.1.0-alpha.1]: https://github.com/denzp/cargo-wharf/compare/cargo-wharf-frontend-v0.1.0-alpha.0...cargo-wharf-frontend-v0.1.0-alpha.1
[0.1.0-alpha.0]: https://github.com/denzp/cargo-wharf/releases/tag/cargo-wharf-frontend-v0.1.0-alpha.0
