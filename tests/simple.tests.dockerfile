# syntax=docker/dockerfile-upstream:experimental
FROM denzp/cargo-container-tools:0.1.0 as container-tools

FROM rustlang/rust:nightly as my-custom-builder
RUN apt-get update
RUN echo "Some build-time system dependencies can be installed here."

FROM my-custom-builder as builder-node-0
WORKDIR /rust-src
RUN curl -L https://crates.io/api/v1/crates/bitflags/1.0.4/download | tar -xvzC /rust-src --strip-components=1
ENV CARGO_MANIFEST_DIR "/rust-src"
RUN ["mkdir", "-p", "/rust-out/debug/deps"]
RUN ["rustc", "--crate-name", "bitflags"]

FROM my-custom-builder as builder-node-1
WORKDIR /rust-src
RUN git clone https://github.com/rust-lang-nursery/log.git /rust-src && git checkout 5f3cb9e144d8fd41362b6a1c9e1c6192e232a1eb
COPY --from=builder-node-0 /rust-out/debug/deps/bitflags.rlib /rust-out/debug/deps/bitflags.rlib
ENV ANY_ENV "\'quotes\" and multiple \nlines"
RUN ["mkdir", "-p", "/rust-out/debug/deps"]
RUN ["rustc", "--crate-name", "log"]
RUN ["ln", "-sf", "debug/deps/log.rlib", "/rust-out/log.rlib"]

FROM my-custom-builder as builder-node-2
WORKDIR /rust-src
COPY . /rust-src
COPY --from=builder-node-0 /rust-out/debug/deps/bitflags.rlib /rust-out/debug/deps/bitflags.rlib
COPY --from=builder-node-1 /rust-out/debug/deps/log.rlib /rust-out/debug/deps/log.rlib
COPY --from=builder-node-1 /rust-out/log.rlib /rust-out/log.rlib
RUN ["mkdir", "-p", "/rust-out/debug/deps"]
RUN ["rustc", "--crate-name", "binary-1"]
RUN ["ln", "-sf", "deps/binary-1-hash", "/rust-out/debug/binary-1"]

FROM my-custom-builder as builder-node-3
WORKDIR /rust-src
COPY . /rust-src
COPY --from=builder-node-0 /rust-out/debug/deps/bitflags.rlib /rust-out/debug/deps/bitflags.rlib
RUN ["mkdir", "-p", "/rust-out/debug/deps"]
RUN ["rustc", "--crate-name", "binary-2"]
RUN ["ln", "-sf", "deps/binary-2-hash", "/rust-out/debug/binary-2"]

FROM my-custom-builder as builder-node-4
WORKDIR /rust-src
COPY . /rust-src
COPY --from=builder-node-0 /rust-out/debug/deps/bitflags.rlib /rust-out/debug/deps/bitflags.rlib
COPY --from=builder-node-1 /rust-out/debug/deps/log.rlib /rust-out/debug/deps/log.rlib
COPY --from=builder-node-1 /rust-out/log.rlib /rust-out/log.rlib
RUN ["mkdir", "-p", "/rust-out/debug/deps"]
RUN ["rustc", "--crate-name", "binary-1", "--test"]
RUN ["ln", "-sf", "deps/binary-1-test-hash", "/rust-out/debug/binary-1-test-hash"]

FROM my-custom-builder as builder-node-5
WORKDIR /rust-src
COPY . /rust-src
COPY --from=builder-node-0 /rust-out/debug/deps/bitflags.rlib /rust-out/debug/deps/bitflags.rlib
RUN ["mkdir", "-p", "/rust-out/debug/deps"]
RUN ["rustc", "--crate-name", "binary-2", "--test"]
RUN ["ln", "-sf", "deps/binary-2-test-hash", "/rust-out/debug/binary-2-test-hash"]




FROM debian:stable-slim as my-awesome-tests
RUN echo "Can setup tests image here: copy static fixtures or install dependencies."
COPY --from=builder-node-4 /rust-out/debug/deps/binary-1-test-hash /rust-tests/binary-1-test-hash
COPY --from=builder-node-5 /rust-out/debug/deps/binary-2-test-hash /rust-tests/binary-2-test-hash
COPY --from=container-tools /usr/local/bin/cargo-test-runner /usr/bin/cargo-test-runner
ENTRYPOINT ["cargo-test-runner", "/rust-tests/binary-1-test-hash", "/rust-tests/binary-2-test-hash"]
