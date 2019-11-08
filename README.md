# cargo-wharf
> Seamless and cacheable Docker container building for Rust crates.

[![asciicast](https://asciinema.org/a/280049.svg)](https://asciinema.org/a/280049?cols=95)

## Features
* **Small and efficient output images.**<br>
*Only binaries (and eventually mandatory static assets) in the output image. No `target` directory or other useless build artifacts.*
* **Share and reuse cached dependencies between the builds.**<br>
*Yes, it's safe. Every dependency is built in its isolated environment.*
* **Ability to produce test images.**<br>
*The container created from that image will do the same as `cargo test` but in a safe isolated environment.*

**Disclaimer #1!** The approach relies on bleeding edge features of Docker, namely [BuildKit]. `cargo-wharf` was tested on `Docker v19.03.3` with BuildKit being enabled (please follow the ["Note for Docker users" section] about `DOCKER_BUILDKIT` environment variable).

**Disclaimer #2!** `cargo-wharf` **should not be used in production**, for the reason, it's being under heavy development.

## Common usage

Taking risks, huh?
Well, then get ready for a mind-blowing experience!
There are several things needed to go on:

### Step 1: Specify a frontend directive.
Add the following line at the beginning of your `Cargo.toml`:
```
# syntax = denzp/cargo-wharf-frontend:v0.1.0-alpha.0
```

*This directive will instruct Docker (or to be precise, BuildKit) to use the image as a frontend.*

### Step 2: Define a build metadata.
`cargo-wharf` needs to know how to build you image.
Inside of `Cargo.toml` specify `[package.metadata.wharf.builder]`, `[package.metadata.wharf.output]` and at least one `[[package.metadata.wharf.binary]]` section.

*Examples can be found [here](cargo-container-tools/Cargo.toml) and [there](cargo-wharf-frontend/Cargo.toml).*

### Step 3: Run the image build process.
Instead of a path to `Dockerfile`, simply provide Docker with a path to `Cargo.toml`:
```
docker build -f path/to/Cargo.toml -t NAME:TAG .
```

*This step will use defined earlier frontend and metadata to create Cargo build plan, and kickstart the image building process.*

### Step 4. Enjoy!
You can change your crate code or even dependencies, and the image will be incrementally rebuilt.

More detailed guide of a frontend usage can be found in its [README](cargo-wharf-frontend/README.md).

## Components

### cargo-wharf-frontend
[[Docker Hub](https://hub.docker.com/r/denzp/cargo-wharf-frontend)]
[[README](cargo-wharf-frontend/README.md)]
[[CHANGELOG](cargo-wharf-frontend/CHANGELOG.md)]

The custom frontend for BuildKit that produces LLB graph out of Cargo's build plan.

### cargo-container-tools
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
["Note for Docker users" section]: https://github.com/moby/buildkit/blob/master/frontend/dockerfile/docs/experimental.md#note-for-docker-users
