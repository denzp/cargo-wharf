load common

function setup() {
    maybe_build_container_tools_image
    maybe_build_frontend_image
}

@test "labels" {
    docker build -f examples/single-bin/Cargo.toml -t cargo-wharf/example-single-bin examples/single-bin

    run docker image inspect cargo-wharf/example-single-bin -f "{{.Config.Labels}}"
    [[ "$output" == "map[my.awesome.label:another value simple-label:simple value]" ]]
}

@test "volumes" {
    docker build -f examples/single-bin/Cargo.toml -t cargo-wharf/example-single-bin examples/single-bin

    run docker image inspect cargo-wharf/example-single-bin -f "{{.Config.Volumes}}"
    [[ "$output" == "map[/data:{} /local:{}]" ]]
}

@test "exposed ports" {
    docker build -f examples/single-bin/Cargo.toml -t cargo-wharf/example-single-bin examples/single-bin

    run docker image inspect cargo-wharf/example-single-bin -f "{{.Config.ExposedPorts}}"
    [[ "$output" == "map[3500/tcp:{} 3600/udp:{} 3700/tcp:{}]" ]]
}
