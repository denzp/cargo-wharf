# cargo-wharf-frontend

## Usage
Simple as it is:
```
docker build -f Cargo.toml .
```

Although, extra one-time preparation has to be made before the build.

1. [Make sure BuildKit is enabled](#buildkit-setup)
2. [Add the frontend directive](#frontend-directive)
3. [Create a builder image config](#builder-image-config)
4. [Create an output image config](#output-image-config)
5. [Specify binaries](#binaries)

## BuildKit setup
The possibility to build crates without Dockerfile is only possible thanks to [BuildKit] external frontends feature.

As for **Docker v19.03.3**, BuildKit can be enabled just by setting `DOCKER_BUILDKIT=1` env variable when running `docker build`.

## Frontend directive
To instruct BuildKit to use the frontend, the first line of the `Cargo.toml` should be:
```
# syntax = denzp/cargo-wharf-frontend:v0.1.0-alpha.0
```

## Builder image config
The builder image is an image that contains Rust toolchain and any extra tools that might be needed to build the crate.

Configuration is made with a `[package.metadata.wharf.builder]` metadata in `Cargo.toml`.

The semantics of the metadata *loosely* tries to follow `Dockerfile` directives:

| Key | Data type | Description | Examples | `Dockerfile` counterpart |
|-----|-----------|-------------|----------|--------------------------|
| image | `String` | Builder image image. | `"rust"`<br>`"clux/muslrust:nightly-2019-09-28"` | [`FROM`] |
| user | `Option<String>` | User which runs `rustc` and build scripts. | `"root"` | [`USER`] |
| env | `Option<BTreeMap<String, String>>` | Environment to run the `rustc` and build scripts. | `{ "NAME 1" = "VALUE 1" }` | [`ENV`] |
| target | `Option<String>` | Output target: similar to<br>`cargo build --target ..` | `"x86_64-unknown-linux-musl"` |

### Examples
Building with a stable Rust:
``` toml
[package.metadata.wharf.builder]
image = "rust"
```

Building MUSL executables:
``` toml
[package.metadata.wharf.builder]
image = "clux/muslrust:nightly-2019-09-28"
target = "x86_64-unknown-linux-musl"
```

## Output image config
The output image is a base where compiled binaries will be put, and tests will run.
There are no restrictions on which image should be used.

Configuration is made with a `[package.metadata.wharf.output]` metadata in `Cargo.toml`.

The semantics of the metadata tries to follow `Dockerfile` directives:

| Key | Data type | Description | Examples | `Dockerfile` counterpart |
|-----|-----------|-------------|----------|--------------------------|
| `image` | `String` | Base for the output image. | `"debian:stable"`<br>`"scratch"`<br>`"alpine"` | [`FROM`] |
| `user` | `Option<String>` | User which runs the entrypoint. | `"root"` | [`USER`] |
| `workdir` | `Option<PathBuf>` | Working directory to run the entrypoint. | `"/tmp"` | [`WORKDIR`] |
| `entrypoint` | `Option<Vec<String>>` | Path and arguments for container entrypoint. | `["/bin/sh", "-c"]` | [`ENTRYPOINT`] |
| `args` | `Option<Vec<String>>` | Default extra arguments for the entrypoint. | `["echo", "hello world"]` | [`CMD`] |
| `env` | `Option<BTreeMap<String, String>>` | Environment to run the entrypoint with. | `{ "NAME 1" = "VALUE 1" }` | [`ENV`] |

### Examples
``` toml
[package.metadata.wharf.output]
image = "debian:stable-slim"
entrypoint = ["/bin/echo", "hello"]
args = ["world"]
```

"Scratch" can be used to have an empty base image:
``` toml
[package.metadata.wharf.output]
image = "scratch"
entrypoint = ["/path/to/executable"]
```

## Binaries
It's also important to specify which binaries should be built and where to put them.
Each crate can use own convention about where the binaries should go.

For example, for `scratch` output image, it might be usefull to put binaries directly into `/` (root).

The binaries should be specified in `[[package.metadata.wharf.binary]]` array in `Cargo.toml`:

| Key | Data type | Description |
|-----|-----------|-------------|
| `name` | `String` | Binary name inside the crate. |
| `destination` | `PathBuf` | Destination path inside the output image. |

### Examples
``` toml
[[package.metadata.wharf.binary]]
name = "cargo-metadata-collector"
destination = "/usr/local/bin/cargo-metadata-collector"

[[package.metadata.wharf.binary]]
name = "cargo-test-runner"
destination = "/cargo-test-runner"
```

## Frontend parameters
There is an additional way to control the frontend: build arguments.

When using Docker, the parameters can be passed into the frontend as:
```
docker build -f Cargo.toml --build-arg debug=build-graph,llb
```

There are several parameters supported:

| Key | Data type | Description | Possible values | Default |
|-----|-----------|-------------|-----------------|---------|
| `profile` | `String` | Output kind - what build and copy into the output image. | `release-binaries`<br>`debug-binaries`<br>`release-tests`<br>`debug-tests` | `release-binaries` |
| `features` | `Vec<String>` | Enable the crate's features | `feature-1,feature-2` |
| `no-default-features` | `bool` | Disable crate's default features | `true`<br>`false` |
| `manifest-path` | `PathBuf` | Override the path to a crate manifest. Please note, this will not affect configuration collecting behavior. | `binary-1/Cargo.toml` |
| `debug` | `bool` or `Vec<String>` | Special mode of the image - instead of building, dump various debug information. | `all`<br>`config`<br>`build-plan`<br>`build-graph`<br>`llb` |

**Note about debugging the frontend**

When `debug=all` is used, every possible debug info will be dumped.
Otherwise, when only a partial dump is needed, several values can be specified: `debug=config,build-plan`.

By default, Docker will compose an image with those debug artifacts, and it might be tedious to inspect them.
The behavior can be overridden: Docker can be instructed to put outputs into a folder:
```
docker build -f Cargo.toml . \
    --output type=local,dest=debug-out \
    --build-arg debug=true
```

[`FROM`]: https://docs.docker.com/engine/reference/builder/#from
[`USER`]: https://docs.docker.com/engine/reference/builder/#from
[`WORKDIR`]: https://docs.docker.com/engine/reference/builder/#workdir
[`ENTRYPOINT`]: https://docs.docker.com/engine/reference/builder/#entrypoint
[`CMD`]: https://docs.docker.com/engine/reference/builder/#cmd
[`ENV`]: https://docs.docker.com/engine/reference/builder/#env

[BuildKit]: https://github.com/moby/buildkit
["Note for Docker users" section]: https://github.com/moby/buildkit/blob/master/frontend/dockerfile/docs/experimental.md#note-for-docker-users
