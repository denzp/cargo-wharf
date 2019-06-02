# syntax=docker/dockerfile-upstream:experimental

FROM ekidd/rust-musl-builder:nightly as builder

USER root
ENV RUSTUP_HOME "/home/rust/.rustup"

# Crate sources and workspace lockfile
WORKDIR /rust-src
COPY Cargo.lock /rust-src/Cargo.lock
COPY cargo-container-tools /rust-src

# Alternative caching approach
RUN --mount=type=cache,target=/rust-src/target --mount=type=cache,target=/root/.cargo \
    ["cargo", "build", "--release", "--target", "x86_64-unknown-linux-musl"]

RUN --mount=type=cache,target=/rust-src/target \
    ["cp", "/rust-src/target/x86_64-unknown-linux-musl/release/cargo-buildscript", "/usr/local/bin/cargo-buildscript"]

RUN --mount=type=cache,target=/rust-src/target \
    ["cp", "/rust-src/target/x86_64-unknown-linux-musl/release/cargo-test-runner", "/usr/local/bin/cargo-test-runner"]

RUN --mount=type=cache,target=/rust-src/target \
    ["cp", "/rust-src/target/x86_64-unknown-linux-musl/release/cargo-ldd", "/usr/local/bin/cargo-ldd"]

# Copy the binaries from build stage
FROM alpine
COPY --from=builder /usr/local/bin/cargo-buildscript /usr/local/bin/cargo-buildscript
COPY --from=builder /usr/local/bin/cargo-test-runner /usr/local/bin/cargo-test-runner
COPY --from=builder /usr/local/bin/cargo-ldd /usr/local/bin/cargo-ldd
