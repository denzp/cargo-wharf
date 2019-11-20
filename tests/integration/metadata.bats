load helpers/images
load helpers/registry

function setup() {
    install_registry
    maybe_build_container_tools_image
    maybe_build_frontend_image
}

function teardown() {
    remove_registry

    docker rmi -f cargo-wharf/example-single-bin || true
}

@test "labels" {
    docker buildx build --load -f examples/single-bin/Cargo.toml -t cargo-wharf/example-single-bin examples/single-bin

    run docker image inspect cargo-wharf/example-single-bin -f "{{.Config.Labels}}"
    [[ "$output" == "map[my.awesome.label:another value simple-label:simple value]" ]]
}

@test "volumes" {
    docker buildx build --load -f examples/single-bin/Cargo.toml -t cargo-wharf/example-single-bin examples/single-bin

    run docker image inspect cargo-wharf/example-single-bin -f "{{.Config.Volumes}}"
    [[ "$output" == "map[/data:{} /local:{}]" ]]
}

@test "exposed ports" {
    docker buildx build --load -f examples/single-bin/Cargo.toml -t cargo-wharf/example-single-bin examples/single-bin

    run docker image inspect cargo-wharf/example-single-bin -f "{{.Config.ExposedPorts}}"
    [[ "$output" == "map[3500/tcp:{} 3600/udp:{} 3700/tcp:{}]" ]]
}
