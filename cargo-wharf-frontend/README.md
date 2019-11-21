# `cargo-wharf-frontend` - BuildKit frontend for Rust

## Usage
Almost simple as it is:
```
docker build -f Cargo.toml .
```

Although, extra one-time setup has to be made before the build.

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
The semantics of the metadata *loosely* tries to follow `Dockerfile` directives.

*Real life examples can be found [here](../cargo-container-tools/Cargo.toml) and [there](Cargo.toml).*

| Base image | |
|--:|:--|
| Key | `package.metadata.wharf.builder.image` |
| Data type| `String` |
| Description | Builder base image that contains Rust. |
| `Dockerfile` counterpart | [`FROM`] |

``` toml
[package.metadata.wharf.builder]
image = "rust"
```

| Setup commands | |
|--:|:--|
| Key | `package.metadata.wharf.builder.setup-commands` |
| Data type| `Option<Vec<CustomCommand>>` |
| Description | Execute commands to setup the builder image. |
| `Dockerfile` counterpart | [`RUN`] |

``` toml
[package.metadata.wharf.builder]
image = "rust"
setup-commands = [
  { shell = "apt-get update && apt-get install -y adb" },
  { command = ["apt-get", "install", "-y", "ffmpeg"], display = "Install ffmpeg" },
]
```

| User | |
|--:|:--|
| Key | `package.metadata.wharf.builder.user` |
| Data type| `Option<String>` |
| Description | User which runs `rustc` and build scripts. |
| `Dockerfile` counterpart | [`USER`]  |

``` toml
[package.metadata.wharf.builder]
image = "rust"
user = "root"
```

| Environment variables | |
|--:|:--|
| Key | `package.metadata.wharf.builder.env` |
| Data type| `Option<BTreeMap<String, String>>` |
| Description | Environment to run the `rustc` and build scripts. |
| `Dockerfile` counterpart | [`ENV`] |

``` toml
[package.metadata.wharf.builder]
image = "rust"
env = { NAME_1 = "VALUE_1" }
```

``` toml
[package.metadata.wharf.builder]
image = "rust"

[package.metadata.wharf.builder.env]
"NAME 1" = "VALUE 1"
```

| Build target | |
|--:|:--|
| Key | `package.metadata.wharf.builder.target` |
| Data type| `Option<String>` |
| Description | Output target: similar to `cargo build --target <TARGET_NAME>` |
| `Dockerfile` counterpart | - |

``` toml
[package.metadata.wharf.builder]
image = "clux/muslrust:nightly-2019-09-28"
target = "x86_64-unknown-linux-musl"
```

## Output image config
The output image is a base where compiled binaries will be put, and tests will run.
There are no restrictions on which image should be used.

Configuration is made with a `[package.metadata.wharf.output]` metadata in `Cargo.toml`.
The semantics of the metadata tries to follow `Dockerfile` directives.

*Real life examples can be found [here](cargo-container-tools/Cargo.toml) and [there](cargo-wharf-frontend/Cargo.toml).*

| Base image | |
|--:|:--|
| Key | `package.metadata.wharf.output.image` |
| Data type| `String` |
| Description | Base for the output image. |
| `Dockerfile` counterpart | [`FROM`] |

``` toml
[package.metadata.wharf.output]
image = "debian:stable-slim"
```

``` toml
[package.metadata.wharf.output]
image = "scratch"
```

| Pre-install commands | |
|--:|:--|
| Key | `package.metadata.wharf.output.pre-install-commands` |
| Data type| `Option<Vec<CustomCommand>>` |
| Description | Execute commands in the output image before the binaries are copied. |
| `Dockerfile` counterpart | [`RUN`] |

``` toml
[package.metadata.wharf.output]
image = "debian"
pre-install-commands = [
  { shell = "apt-get update && apt-get install -y adb", display = "My custom shell command" },
  { command = ["apt-get", "install", "-y", "ffmpeg"], display = "My custom command" },
]
```

| Post-install commands | |
|--:|:--|
| Key | `package.metadata.wharf.output.post-install-commands` |
| Data type| `Option<Vec<CustomCommand>>` |
| Description | Execute commands in the output image after the binaries were copied. |
| `Dockerfile` counterpart | [`RUN`] |

``` toml
[package.metadata.wharf.output]
image = "debian"
post-install-commands = [
  { shell = "ldd my-binary-1 | grep -qzv 'not found'", display = "Check shared deps" },
]
```

| User | |
|--:|:--|
| Key | `package.metadata.wharf.output.user` |
| Data type| `Option<String>` |
| Description | User which runs the entrypoint. |
| `Dockerfile` counterpart | [`USER`] |

``` toml
[package.metadata.wharf.output]
image = "scratch"
user = "root"
```

| Working directory | |
|--:|:--|
| Key | `package.metadata.wharf.output.workdir` |
| Data type| `Option<PathBuf>` |
| Description | Working directory to run the entrypoint. |
| `Dockerfile` counterpart | [`WORKDIR`] |

``` toml
[package.metadata.wharf.output]
image = "debian:stable-slim"
workdir = "/tmp"
```

| Entrypoint | |
|--:|:--|
| Key | `package.metadata.wharf.output.entrypoint` |
| Data type| `Option<Vec<String>>` |
| Description | Path and arguments for the container entrypoint. |
| `Dockerfile` counterpart | [`ENTRYPOINT`] |

``` toml
[package.metadata.wharf.output]
image = "debian:stable-slim"
entrypoint = ["/bin/sh", "-c"]
```

| Additional arguments | |
|--:|:--|
| Key | `package.metadata.wharf.output.args` |
| Data type| `Option<Vec<String>>` |
| Description | Default extra arguments for the entrypoint. |
| `Dockerfile` counterpart | [`CMD`] |


``` toml
[package.metadata.wharf.output]
image = "debian:stable-slim"
entrypoint = ["/bin/echo", "hello"]
args = ["world"]
```

| Environment variables | |
|--:|:--|
| Key | `package.metadata.wharf.output.env` |
| Data type| `Option<BTreeMap<String, String>>` |
| Description | Environment variables to run the entrypoint with. |
| `Dockerfile` counterpart | [`ENV`] |

``` toml
[package.metadata.wharf.output]
image = "scratch"
env = { NAME_1 = "VALUE_1" }
```

``` toml
[package.metadata.wharf.output]
image = "scratch"

[package.metadata.wharf.output.env]
"NAME 1" = "VALUE 1"
```

| Volumes | |
|--:|:--|
| Key | `package.metadata.wharf.output.volumes` |
| Data type| `Option<Vec<PathBuf>>` |
| Description | Pathes to the mount points of container volumes. |
| `Dockerfile` counterpart | [`VOLUME`] |

``` toml
[package.metadata.wharf.output]
image = "scratch"
volumes = ["/local", "/data"]
```

| Exposed ports | |
|--:|:--|
| Key | `package.metadata.wharf.output.expose` |
| Data type| `Option<Vec<ExposedPort>>` |
| Description | Announce which ports will be listened. |
| `Dockerfile` counterpart | [`EXPOSE`] |

``` toml
[package.metadata.wharf.output]
image = "scratch"
expose = ["3500/tcp", "3600/udp", "3700"]
```

| Labels | |
|--:|:--|
| Key | `package.metadata.wharf.output.labels` |
| Data type| `Option<BTreeMap<String, String>>` |
| Description | Labels the output images should be annotated with. |
| `Dockerfile` counterpart | [`LABEL`] |

``` toml
[package.metadata.wharf.output]
image = "scratch"

[package.metadata.wharf.output.labels]
"simple-label" = "simple value"
"my.awesome.label" = "another value"
```

| Stop signal | |
|--:|:--|
| Key | `package.metadata.wharf.output.stop-signal` |
| Data type| `Option<Signal>` |
| Description | System call signal that will be sent to the container to exit. |
| `Dockerfile` counterpart | [`STOPSIGNAL`] |

``` toml
[package.metadata.wharf.output]
image = "scratch"
stop-signal = "SIGINT"
```

## Binaries
It's also important to specify which binaries should be built and where to put them.
Each crate can use own convention about where the binaries should go.

For example, with `scratch` output image, it might be usefull to put binaries directly into `/` (root).

The binaries should be specified in `[[package.metadata.wharf.binary]]` array in `Cargo.toml`:

| Key | Data type | Description |
|-----|-----------|-------------|
| `name` | `String` | Binary name inside the crate. |
| `destination` | `PathBuf` | Destination path inside the output image. |

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

| Profile | |
|--:|:--|
| Name | `profile` |
| Data type| `Option<Profile>` |
| Description | Defines what will be built and copied into the output image. |
| *Possible values* | `release-binaries`, `release-tests`,<br>`debug-binaries`, `debug-tests` |
| **Default** | `release-binaries` |

```
docker build -f Cargo.toml --build-arg profile=release-tests
```

| Features | |
|--:|:--|
| Name | `features` |
| Data type| `Option<Vec<String>>` |
| Description | Enable the crate's features. |

```
docker build -f Cargo.toml --build-arg features=feature-1,feature-2
```

| Default features | |
|--:|:--|
| Name | `no-default-features` |
| Data type| `Option<bool>` |
| Description | Disable crate's default features. |
| *Possible values* | `true`, `false` |

```
docker build -f Cargo.toml --build-arg no-default-features=true
```

| Manifest path | |
|--:|:--|
| Name | `manifest-path` |
| Data type| `Option<PathBuf>` |
| Description | Override the path to a crate manifest. Please note, this will not affect configuration collecting behavior. |

```
docker build -f Cargo.toml --build-arg manifest-path=binary-1/Cargo.toml
```

| Debug mode | |
|--:|:--|
| Name | `debug` |
| Data type| `Vec<DebugKind>` |
| Description | Special mode of the image - instead of building, dump various debug information. |
| *Possible values* | `all`, `config`, `build-plan`, `build-graph`, `llb` |

```
docker build -f Cargo.toml --build-arg debug=build-graph,llb
```

```
docker build -f Cargo.toml --build-arg debug=all
```

**Note about debugging the frontend**

When `debug=all` is used, every possible debug information will be dumped.
Otherwise, when only a partial dump is needed, several values can be specified: `debug=config,build-plan`.

By default, Docker will compose an image with those debug artifacts, and it might be tedious to inspect them.
The behavior can be overridden: Docker can be instructed to put outputs into a folder:
```
docker build -f Cargo.toml . \
    --output type=local,dest=debug-out \
    --build-arg debug=all
```

[`FROM`]: https://docs.docker.com/engine/reference/builder/#from
[`USER`]: https://docs.docker.com/engine/reference/builder/#from
[`WORKDIR`]: https://docs.docker.com/engine/reference/builder/#workdir
[`ENTRYPOINT`]: https://docs.docker.com/engine/reference/builder/#entrypoint
[`CMD`]: https://docs.docker.com/engine/reference/builder/#cmd
[`ENV`]: https://docs.docker.com/engine/reference/builder/#env
[`LABEL`]: https://docs.docker.com/engine/reference/builder/#label
[`EXPOSE`]: https://docs.docker.com/engine/reference/builder/#expose
[`VOLUME`]: https://docs.docker.com/engine/reference/builder/#volume
[`STOPSIGNAL`]: https://docs.docker.com/engine/reference/builder/#stopsignal
[`RUN`]: https://docs.docker.com/engine/reference/builder/#run

[BuildKit]: https://github.com/moby/buildkit
["Note for Docker users" section]: https://github.com/moby/buildkit/blob/master/frontend/dockerfile/docs/experimental.md#note-for-docker-users
