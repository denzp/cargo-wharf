# syntax=docker/dockerfile-upstream:experimental

FROM rustlang/rust:nightly as builder

# Crate whole workspace
WORKDIR /rust-src
COPY . /rust-src

# Build the binary with a cache mount
RUN --mount=type=cache,target=/rust-src/target \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/usr/local/cargo/registry \
    ["cargo", "build", "--bin", "cargo-buildkit-frontend"]

# Copy the resulting binary into a normal layer
RUN --mount=type=cache,target=/rust-src/target \
    ["cp", "/rust-src/target/debug/cargo-buildkit-frontend", "/usr/local/bin/cargo-buildkit-frontend"]

# Copy the binaries into the final stage
FROM debian:stable-slim
COPY --from=builder /usr/local/bin/cargo-buildkit-frontend /usr/local/bin/cargo-buildkit-frontend
ENTRYPOINT ["/usr/local/bin/cargo-buildkit-frontend"]
