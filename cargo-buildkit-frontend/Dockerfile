FROM clux/muslrust:nightly as builder

USER root

WORKDIR /rust-src
COPY . /rust-src

RUN --mount=type=cache,target=/rust-src/target \
    --mount=type=cache,target=/root/.cargo/git \
    --mount=type=cache,target=/root/.cargo/registry \
    ["cargo", "build", "--release", "--target", "x86_64-unknown-linux-musl", "--bin", "cargo-buildkit-frontend"]

RUN --mount=type=cache,target=/rust-src/target \
    ["cp", "/rust-src/target/x86_64-unknown-linux-musl/release/cargo-buildkit-frontend", "/usr/local/bin/cargo-buildkit-frontend"]

FROM alpine
COPY --from=builder /usr/local/bin/cargo-buildkit-frontend /usr/local/bin/cargo-buildkit-frontend
ENTRYPOINT ["/usr/local/bin/cargo-buildkit-frontend"]
