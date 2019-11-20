load helpers/images
load helpers/registry

function setup() {
    install_registry
    maybe_build_container_tools_image
    maybe_build_frontend_image
}

function teardown() {
    remove_registry

    docker rmi -f cargo-wharf/example-multi-bin || true
}

@test "pretty print binaries" {
    run docker buildx build --load -f examples/multi-bin/Cargo.toml -t cargo-wharf/example-multi-bin examples/multi-bin

    [ "$status" -eq 0 ]
    [[ "$output" == *"Running   \`Update apt-get cache\`"* ]]
    [[ "$output" == *"Running   \`apt-get install -y protobuf-compiler\`"* ]]
    [[ "$output" == *"Running   \`echo '' > /custom-output\`"* ]]
    [[ "$output" == *"Compiling pkg-config"* ]]
    [[ "$output" == *"Compiling cc"* ]]
    [[ "$output" == *"Compiling openssl-sys [build script]"* ]]
    [[ "$output" == *"Running   openssl-sys [build script]"* ]]
    [[ "$output" == *"Compiling openssl-sys"* ]]
    [[ "$output" == *"Compiling binary bin-1"* ]]
    [[ "$output" == *"Compiling binary bin-2"* ]]
}

@test "pretty print tests" {
    run docker buildx build --load -f examples/multi-bin/Cargo.toml -t cargo-wharf/example-multi-bin examples/multi-bin --build-arg profile=test

    [ "$status" -eq 0 ]
    [[ "$output" == *"Running   \`Update apt-get cache\`"* ]]
    [[ "$output" == *"Running   \`apt-get install -y protobuf-compiler\`"* ]]
    [[ "$output" == *"Running   \`echo '' > /custom-output\`"* ]]
    [[ "$output" == *"Compiling pkg-config"* ]]
    [[ "$output" == *"Compiling cc"* ]]
    [[ "$output" == *"Compiling openssl-sys [build script]"* ]]
    [[ "$output" == *"Running   openssl-sys [build script]"* ]]
    [[ "$output" == *"Compiling openssl-sys"* ]]
    [[ "$output" == *"Compiling test bin_1"* ]]
    [[ "$output" == *"Compiling test bin_2"* ]]
}
