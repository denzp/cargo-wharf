load common

function setup() {
    maybe_build_container_tools_image
    maybe_build_frontend_image
}

@test "default behavior" {
    docker build -f examples/single-bin/Cargo.toml -t cargo-wharf/example-single-bin examples/single-bin

    run docker run --rm cargo-wharf/example-single-bin
    [[ "$output" == *"feature-1 is on"* ]]
    [[ "$output" != *"feature-2 is on"* ]]
}

@test "no-default-features" {
    docker build -f examples/single-bin/Cargo.toml -t cargo-wharf/example-single-bin examples/single-bin --build-arg no-default-features=true

    run docker run --rm cargo-wharf/example-single-bin
    [[ "$output" != *"feature-1 is on"* ]]
    [[ "$output" != *"feature-2 is on"* ]]
}

@test "no-default-features + features" {
    docker build -f examples/single-bin/Cargo.toml -t cargo-wharf/example-single-bin examples/single-bin --build-arg no-default-features=true --build-arg features=feature-2

    run docker run --rm cargo-wharf/example-single-bin
    [[ "$output" != *"feature-1 is on"* ]]
    [[ "$output" == *"feature-2 is on"* ]]
}

@test "features" {
    docker build -f examples/single-bin/Cargo.toml -t cargo-wharf/example-single-bin examples/single-bin --build-arg features=feature-2

    run docker run --rm cargo-wharf/example-single-bin
    [[ "$output" == *"feature-1 is on"* ]]
    [[ "$output" == *"feature-2 is on"* ]]
}

@test "workspace features" {
    docker build -f examples/workspace/Cargo.toml -t cargo-wharf/example-workspace examples/workspace --build-arg features=the-special-feature --build-arg manifest-path=binary-1/Cargo.toml

    run docker run --rm cargo-wharf/example-workspace binary-1
    [[ "$output" == *"the-special-feature is on"* ]]
}
