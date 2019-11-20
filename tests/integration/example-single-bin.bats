load helpers/images
load helpers/registry

function setup() {
    install_registry
    maybe_build_container_tools_image
    maybe_build_frontend_image
}

function teardown() {
    remove_registry

    docker rmi -f cargo-wharf/example-single-bin:test || true
    docker rmi -f cargo-wharf/example-single-bin || true
}

@test "single-bin :: binaries" {
    docker buildx build --load -f examples/single-bin/Cargo.toml -t cargo-wharf/example-single-bin examples/single-bin

    run docker run --rm cargo-wharf/example-single-bin
    [ "$status" -eq 0 ]
    [ "${lines[0]}" = "Hello from the container!" ]
    [ "${lines[1]}" = "Args: [\"/bin/wharf-output\", \"predefined arg\"]" ]
    [[ "$output" == *"\"NAME 1\": \"VALUE 1\""* ]]

    run docker run --rm cargo-wharf/example-single-bin extra-arg
    [ "$status" -eq 0 ]
    [ "${lines[1]}" = "Args: [\"/bin/wharf-output\", \"extra-arg\"]" ]
}

@test "single-bin :: tests" {
    docker buildx build --load -f examples/single-bin/Cargo.toml -t cargo-wharf/example-single-bin:test examples/single-bin --build-arg profile=test

    run docker run --rm cargo-wharf/example-single-bin:test
    [ "$status" -eq 0 ]
    [[ "$output" == *"running 0 tests"* ]]
    [[ "$output" == *"test result: ok"* ]]
}
