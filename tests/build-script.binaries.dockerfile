# syntax=docker/dockerfile-upstream:experimental
FROM denzp/cargo-container-tools:0.1.0 as container-tools

FROM rustlang/rust:nightly as my-custom-builder
RUN apt-get update
RUN echo "Some build-time system dependencies can be installed here."

FROM my-custom-builder as builder-node-0
WORKDIR /rust-src
RUN curl -L https://crates.io/api/v1/crates/lazy_static/1.1.0/download | tar -xvzC /rust-src --strip-components=1
RUN ["mkdir", "-p", "/rust-out/debug/deps"]
RUN ["rustc", "--crate-name", "build_script_build"]
RUN ["ln", "-sf", "deps/build_script_build-6b781e25beddd7bd", "/rust-out/debug/build-script-build"]

FROM my-custom-builder as builder-node-2
WORKDIR /rust-src
RUN curl -L https://crates.io/api/v1/crates/lazy_static/1.1.0/download | tar -xvzC /rust-src --strip-components=1
COPY --from=builder-node-0 /rust-out/debug/build-script-build /rust-out/debug/build-script-build
COPY --from=builder-node-0 /rust-out/debug/deps/build_script_build-6b781e25beddd7bd /rust-out/debug/deps/build_script_build-6b781e25beddd7bd
RUN ["mkdir", "-p", "/rust-out/debug/deps"]
RUN ["mkdir", "-p", "/rust-out/somewhere/out"]
RUN ["sh", "-c", "echo '{\"ANY_ENV_VAR\":\"value\",\"CARGO_MANIFEST_DIR\":\"/rust-src\",\"OUT_DIR\":\"/rust-out/somewhere/out\"}' > /tmp/.buildscript-env"]
RUN ["sh", "-c", "echo '[\"--crate-name\",\"lazy_static\",\"--feature=\\\"any\\\"\"]' > /tmp/.rustc-args"]
RUN ["sh", "-c", "echo '{\"ANY_OTHER_ENV_VAR\":\"\'quotes\\\" and multiple \\nlines\",\"CARGO_MANIFEST_DIR\":\"/rust-src\",\"OUT_DIR\":\"/rust-out/somewhere/out\"}' > /tmp/.rustc-env"]
RUN --mount=from=container-tools,target=/usr/local/bin/cargo-buildscript ["/usr/local/bin/cargo-buildscript", "debug/build-script-build", "--buildscript-env", "/tmp/.buildscript-env", "--rustc-args", "/tmp/.rustc-args", "--rustc-env", "/tmp/.rustc-env"]
RUN ["ln", "-sf", "deps/lazy_static-hash.rlib", "/rust-out/debug/lazy_static.rlib"]

FROM my-custom-builder as builder-node-3
WORKDIR /rust-src
COPY . /rust-src
COPY --from=builder-node-0 /rust-out/debug/build-script-build /rust-out/debug/build-script-build
COPY --from=builder-node-0 /rust-out/debug/deps/build_script_build-6b781e25beddd7bd /rust-out/debug/deps/build_script_build-6b781e25beddd7bd
COPY --from=builder-node-2 /rust-out/debug/deps/lazy_static-hash.rlib /rust-out/debug/deps/lazy_static-hash.rlib
COPY --from=builder-node-2 /rust-out/debug/lazy_static.rlib /rust-out/debug/lazy_static.rlib
RUN ["mkdir", "-p", "/rust-out/debug/deps"]
RUN ["rustc", "--crate-name", "binary-1"]
RUN ["ln", "-sf", "deps/binary-1-hash", "/rust-out/debug/binary-1"]


FROM debian:stable-slim as my-awesome-binaries
RUN echo "Can setup binaries image here."
COPY --from=builder-node-3 /rust-out/debug/deps/binary-1-hash /usr/local/bin/binary-1
RUN --mount=from=container-tools,target=/usr/local/bin/cargo-ldd ["/usr/local/bin/cargo-ldd", "/usr/local/bin/binary-1"]

