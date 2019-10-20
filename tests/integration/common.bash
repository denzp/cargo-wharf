function maybe_build_container_tools_image {
    if_changed          cargo-container-tools               build_container_tools_image
    if_image_missing    denzp/cargo-container-tools:local   build_container_tools_image
}

function maybe_build_frontend_image() {
    if_changed          cargo-wharf-frontend                build_frontend_image
    if_image_missing    denzp/cargo-wharf-frontend:local    build_frontend_image
}

function build_container_tools_image {
    echo -e '# \rbuilding the container-tools docker image...' >&3
    docker build -f cargo-container-tools/Dockerfile -t denzp/cargo-container-tools:local .
    echo -e '# \rbuilding the container-tools docker image... done' >&3
}

function build_frontend_image() {
    echo -e '# \rbuilding the frontend docker image...' >&3
    docker build -f cargo-wharf-frontend/Dockerfile -t denzp/cargo-wharf-frontend:local . --build-arg extra_build_args="--features=local-container-tools"
    echo -e '# \rbuilding the frontend docker image... done' >&3
}

function if_changed() {
    CACHE_SOURCE=$1
    CACHE_FILE=tests/integration/.cache/$(echo "$CACHE_SOURCE" | sha1sum | awk '{print $1}')

    if [ ! -r $CACHE_FILE ] || [ ! $(cat $CACHE_FILE) == $(get_hash $1) ]; then
        echo "if_changed: '$1' changed since last run..." >&3

        shift
        $@

        mkdir -p $(dirname $CACHE_FILE)
        echo -n $(get_hash $CACHE_SOURCE) > $CACHE_FILE
    fi;
}

function if_image_missing() {
    if ! docker image inspect $1 >&2 > /dev/null; then
        echo "if_image_missing: '$1' is missing..." >&3

        shift
        $@
    fi;
}

function get_hash() {
    find $1 -type f -print0 | sort -z | xargs -0 sha1sum | sha1sum | awk '{print $1}'
}
