# syntax = docker/dockerfile:1.1-experimental

FROM clux/muslrust:nightly-2019-09-28 as builder
USER root

ARG extra_build_args

WORKDIR /rust-src
COPY . /rust-src

RUN --mount=type=cache,target=/rust-src/target \
    --mount=type=cache,target=/root/.cargo/git \
    --mount=type=cache,target=/root/.cargo/registry \
    cargo build --manifest-path cargo-wharf-frontend/Cargo.toml --release --target x86_64-unknown-linux-musl --bin cargo-wharf-frontend $extra_build_args

RUN --mount=type=cache,target=/rust-src/target \
    ["cp", "/rust-src/target/x86_64-unknown-linux-musl/release/cargo-wharf-frontend", "/usr/local/bin/cargo-wharf-frontend"]

FROM alpine
COPY --from=builder /usr/local/bin/cargo-wharf-frontend /usr/local/bin/cargo-wharf-frontend
ENTRYPOINT ["/usr/local/bin/cargo-wharf-frontend"]
