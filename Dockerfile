FROM rustlang/rust:nightly as builder
COPY cargo-container-tools /rust-src

USER root
WORKDIR /rust-src
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo build --release --target x86_64-unknown-linux-musl

# Ensure the binaries can be run on normal container
RUN ldd /rust-src/target/x86_64-unknown-linux-musl/release/cargo-buildscript | grep -qzv "not found"
RUN ldd /rust-src/target/x86_64-unknown-linux-musl/release/cargo-test-runner | grep -qzv "not found"
RUN ldd /rust-src/target/x86_64-unknown-linux-musl/release/cargo-ldd | grep -qzv "not found"

# Copy the binaries from build stage
FROM alpine
COPY --from=builder /rust-src/target/x86_64-unknown-linux-musl/release/cargo-buildscript /usr/local/bin/cargo-buildscript
COPY --from=builder /rust-src/target/x86_64-unknown-linux-musl/release/cargo-test-runner /usr/local/bin/cargo-test-runner
COPY --from=builder /rust-src/target/x86_64-unknown-linux-musl/release/cargo-ldd /usr/local/bin/cargo-ldd
