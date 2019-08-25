FROM clux/muslrust:nightly-2019-08-24 as builder
USER root

WORKDIR /rust-src
COPY . /rust-src

RUN --mount=type=cache,target=/rust-src/target \
    --mount=type=cache,target=/root/.cargo/git \
    --mount=type=cache,target=/root/.cargo/registry \
    ["cargo", "build", "--release", "--target", "x86_64-unknown-linux-musl"]

RUN --mount=type=cache,target=/rust-src/target \
    ["cp", "/rust-src/target/x86_64-unknown-linux-musl/release/cargo-buildscript-capture", "/usr/local/bin/cargo-buildscript-capture"]

RUN --mount=type=cache,target=/rust-src/target \
    ["cp", "/rust-src/target/x86_64-unknown-linux-musl/release/cargo-buildscript-apply", "/usr/local/bin/cargo-buildscript-apply"]

FROM alpine
COPY --from=builder /usr/local/bin/cargo-buildscript-capture /usr/local/bin/cargo-buildscript-capture
COPY --from=builder /usr/local/bin/cargo-buildscript-apply /usr/local/bin/cargo-buildscript-apply
