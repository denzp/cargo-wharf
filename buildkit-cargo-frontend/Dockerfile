# syntax=docker/dockerfile-upstream:experimental

FROM rustlang/rust:nightly as builder

# Crate whole workspace
WORKDIR /rust-src
COPY . /rust-src

# Build the binary with a cache mount
RUN --mount=type=cache,target=/rust-src/target \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/usr/local/cargo/registry \
    ["cargo", "build", "--release", "--bin", "buildkit-cargo-frontend"]

# Copy the resulting binary into a normal layer
RUN --mount=type=cache,target=/rust-src/target \
    ["cp", "/rust-src/target/release/buildkit-cargo-frontend", "/usr/local/bin/buildkit-cargo-frontend"]

# Copy the binaries into the final stage
FROM debian:stable-slim
COPY --from=builder /usr/local/bin/buildkit-cargo-frontend /usr/local/bin/buildkit-cargo-frontend
ENTRYPOINT ["/usr/local/bin/buildkit-cargo-frontend"]
