# cargo-wharf
> Seamless and cacheable Docker container building toolkit for Rust.

## Features
* **Small and efficient output images.**<br>
*Only binaries (and eventually mandatory static assets) in the output image. No `target` directory or other useless build artifacts.*
* **Share and reuse cached dependencies between the builds.**<br>
*Yes, it's safe. Every dependency is built in its isolated environment.*
* **Ability to produce test images.**<br>
*The container created from that image will do the same as `cargo test` but in a safe isolated environment.*

**Disclaimer #1!** The approach relies on bleeding edge features of Docker, namely [BuildKit]. The `cargo-wharf` was tested on `Docker v19.03.3` with BuildKit being enabled (please follow the ["Note for Docker users" section] about `DOCKER_BUILDKIT` environment variable).

**Disclaimer #2!** The `cargo-wharf` **should not be used in production**, for the reason, it's being under heavy development.

## Common usage

Taking risks, huh?
Well, get ready for a mind-blowing experience.
There are several things needed to go on:
1. Add `# syntax = denzp/cargo-wharf-frontend:v0.1.0-alpha.0` as the first line of your `Cargo.toml`.
2. Define important metadata: builder and output images, list of binaries and their final locations. Examples can be found [here](cargo-container-tools/Cargo.toml) and [there](cargo-wharf-frontend/Cargo.toml).
3. Run `docker build -f path/to/Cargo.toml .`
4. Change the code or dependency crates and repeat *Step 3*.

Effectively, *Step 3* will use the frontend image (specified at *Step 1*) to gather build plan and image metadata (defined at *Step 2*) and kickstart image building.

Every dependency is going be built and (which is more importantly) **cached** in a similar to a stage in extremely multi-staged Dockerfile.
Early experiments [might be useful](https://github.com/denzp/cargo-wharf/blob/experiment-dockerfile/tests/simple.binaries.dockerfile) to understand the operation concept.

## Components

### cargo-wharf-frontend
[Docker Hub](https://hub.docker.com/r/denzp/cargo-wharf-frontend)
[README](cargo-wharf-frontend/README.md)
[CHANGELOG](cargo-wharf-frontend/CHANGELOG.md)

The custom frontend for BuildKit that produces LLB graph out of Cargo's build plan.

### cargo-container-tools
[Docker Hub](https://hub.docker.com/r/denzp/cargo-container-tools)
[README](cargo-container-tools/README.md)
[CHANGELOG](cargo-container-tools/CHANGELOG.md)

Auxiliary tools that are useful for building Docker images of Rust crates and for `cargo-wharf-frontend` in particular.

[BuildKit]: https://github.com/moby/buildkit
["Note for Docker users" section]: https://github.com/moby/buildkit/blob/master/frontend/dockerfile/docs/experimental.md#note-for-docker-users
