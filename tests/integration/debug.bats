load helpers/images
load helpers/registry

function setup() {
    install_registry
    maybe_build_container_tools_image
    maybe_build_frontend_image
}

function teardown() {
    remove_registry
}

@test "debug output" {
    rm -f $BATS_TMPDIR/*.json
    rm -f $BATS_TMPDIR/*.pb

    docker buildx build -f examples/single-bin/Cargo.toml examples/single-bin \
        -o type=local,dest=$BATS_TMPDIR \
        --build-arg debug=all

    [ -r $BATS_TMPDIR/build-graph.json ]
    [ -r $BATS_TMPDIR/build-plan.json ]
    [ -r $BATS_TMPDIR/config.json ]
    [ -r $BATS_TMPDIR/llb.pb ]
}

@test "specific debug output" {
    rm -f $BATS_TMPDIR/*.json
    rm -f $BATS_TMPDIR/*.pb

    docker buildx build -f examples/single-bin/Cargo.toml examples/single-bin \
        -o type=local,dest=$BATS_TMPDIR \
        --build-arg debug=build-plan,build-graph

    [ -r $BATS_TMPDIR/build-graph.json ]
    [ -r $BATS_TMPDIR/build-plan.json ]
    [ ! -r $BATS_TMPDIR/config.json ]
    [ ! -r $BATS_TMPDIR/llb.pb ]
}
