## Cargo Wharf - cacheable and efficient Docker images builder for Rust.

# Features
* **Efficiently cache dependencies between the builds.**<br>
*Every dependency is built in its isolated environment and cached independently from others.*
* **Small and efficient output images.**<br>
*Only binaries (and eventually mandatory static assets) in the output image. No `target` directory or other useless build artifacts.*
* **No tools has to be installed on the host.**<br>
*Only Docker!*
* **Ability to produce test images.**<br>
*The container created from that image will do the same as `cargo test` but in a safe isolated environment.*

<p align="center">
    <a href="https://asciinema.org/a/280049" target="_blank"><img src="https://asciinema.org/a/280049.svg" /></a>
</p>

# Usage
**Disclaimer #1** Modern Docker with enabled [BuildKit] is needed!

**Disclaimer #2** Due to active development and expected breaking changes `cargo-wharf` **should not be used in production yet**.

**Usage guide can be found in the BuildKit frontend [README](cargo-wharf-frontend/README.md).**

## `cargo-wharf-frontend`
[[Docker Hub](https://hub.docker.com/r/denzp/cargo-wharf-frontend)]
[[README](cargo-wharf-frontend/README.md)]
[[CHANGELOG](cargo-wharf-frontend/CHANGELOG.md)]

The custom frontend for BuildKit that produces LLB graph out of Cargo's build plan.

## `cargo-container-tools`
[[Docker Hub](https://hub.docker.com/r/denzp/cargo-container-tools)]
[[README](cargo-container-tools/README.md)]
[[CHANGELOG](cargo-container-tools/CHANGELOG.md)]

Auxiliary tools that are useful for building Docker images of Rust crates and for `cargo-wharf-frontend` in particular.

# License
`cargo-wharf` is primarily distributed under the terms of both the MIT license and
the Apache License (Version 2.0), with portions covered by various BSD-like
licenses.

See LICENSE-APACHE, and LICENSE-MIT for details.

# Contribution
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in `cargo-wharf` by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.

[BuildKit]: https://github.com/moby/buildkit
