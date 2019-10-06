load common

function setup() {
    maybe_build_container_tools_image
    maybe_build_frontend_image
}

@test "single-bin :: binaries" {
    docker build -f examples/single-bin/Cargo.toml -t cargo-wharf/example-single-bin examples/single-bin

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
    docker build -f examples/single-bin/Cargo.toml -t cargo-wharf/example-single-bin:test examples/single-bin --build-arg mode=test

    run docker run --rm cargo-wharf/example-single-bin:test
    [ "$status" -eq 0 ]
    [[ "$output" == *"running 0 tests"* ]]
    [[ "$output" == *"test result: ok"* ]]
}
