load common

function setup() {
    maybe_build_container_tools_image
    maybe_build_frontend_image
}

@test "multi-bin :: binaries" {
    docker build -f examples/multi-bin/Cargo.toml -t cargo-wharf/example-multi-bin examples/multi-bin

    run docker run --rm cargo-wharf/example-multi-bin /bin/bin-1
    [ "$status" -eq 0 ]
    [ "$output" = "Hello from the bin-1!" ]

    run docker run --rm cargo-wharf/example-multi-bin /bin/bin-2
    [ "$status" -eq 0 ]
    [ "$output" = "Hello from the bin-2!" ]
}

@test "multi-bin :: tests" {
    docker build -f examples/multi-bin/Cargo.toml -t cargo-wharf/example-multi-bin:test examples/multi-bin --build-arg profile=test

    run docker run --rm cargo-wharf/example-multi-bin:test
    [ "$status" -eq 0 ]
    [[ "$output" == *"test bin_1_test_case ... ok"* ]]
    [[ "$output" == *"test bin_2_test_case ... ok"* ]]
}
