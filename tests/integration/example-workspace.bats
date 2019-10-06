load common

function setup() {
    maybe_build_container_tools_image
    maybe_build_frontend_image
}

@test "workspace example :: binaries" {
    docker build -f examples/workspace/Cargo.toml -t cargo-wharf/example-workspace examples/workspace

    run docker run --rm cargo-wharf/example-workspace
    [ "$status" -eq 0 ]
    [ "$output" = "" ]

    run docker run --rm cargo-wharf/example-workspace binary-1
    [ "$status" -eq 0 ]
    [ "${lines[0]}" = "Hello from build script" ]
    [ "${lines[1]}" = "Hello from binary 1" ]

    run docker run --rm cargo-wharf/example-workspace binary-2
    [ "$status" -eq 0 ]
    [ "${lines[0]}" = "Hello from build script" ]
    [ "${lines[1]}" = "Hello from binary 2" ]
}

@test "workspace example :: tests" {
    docker build -f examples/workspace/Cargo.toml -t cargo-wharf/example-workspace:test examples/workspace --build-arg mode=test

    run docker run --rm cargo-wharf/example-workspace:test
    [ "$status" -eq 1 ]
    [[ "$output" == *"running 1 test"* ]]
    [[ "$output" == *"test faulty_test ... FAILED"* ]]
    [[ "$output" == *"'faulty_test' panicked at 'this should fail'"* ]]
    [[ "$output" == *"test result: FAILED"* ]]
}
