FROM ekidd/rust-musl-builder:nightly as builder
WORKDIR /rust-src

# Fix permissions
USER root
RUN ["sudo", "chown", "-R", "rust:rust", "/rust-src"]

# Old good caching approach
USER rust
ENV USER "rust"
RUN ["cargo", "init", "--bin"]
COPY cargo-container-tools/Cargo.toml /rust-src/Cargo.toml
RUN ["cargo", "build", "--release", "--target", "x86_64-unknown-linux-musl"]

# Real build
COPY cargo-container-tools/src /rust-src/src
RUN ["cargo", "build", "--release", "--target", "x86_64-unknown-linux-musl"]

# Ensure the binaries can be run on normal container
RUN ldd /rust-src/target/x86_64-unknown-linux-musl/release/cargo-buildscript | grep -qzv "not found"
RUN ldd /rust-src/target/x86_64-unknown-linux-musl/release/cargo-test-runner | grep -qzv "not found"
RUN ldd /rust-src/target/x86_64-unknown-linux-musl/release/cargo-ldd | grep -qzv "not found"

# Copy the binaries from build stage
FROM alpine
COPY --from=builder /rust-src/target/x86_64-unknown-linux-musl/release/cargo-buildscript /usr/local/bin/cargo-buildscript
COPY --from=builder /rust-src/target/x86_64-unknown-linux-musl/release/cargo-test-runner /usr/local/bin/cargo-test-runner
COPY --from=builder /rust-src/target/x86_64-unknown-linux-musl/release/cargo-ldd /usr/local/bin/cargo-ldd
