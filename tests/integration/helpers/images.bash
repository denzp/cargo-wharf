function maybe_build_container_tools_image {
    if_changed          cargo-container-tools                               build_container_tools_image
    if_image_missing    localhost:10395/denzp/cargo-container-tools:local   build_container_tools_image

    docker push localhost:10395/denzp/cargo-container-tools:local
}

function maybe_build_frontend_image() {
    if_changed          cargo-wharf-frontend                                build_frontend_image
    if_image_missing    localhost:10395/denzp/cargo-wharf-frontend:local    build_frontend_image

    docker push localhost:10395/denzp/cargo-wharf-frontend:local
}

function build_container_tools_image {
    echo -e '# \rbuilding the container-tools docker image...' >&3

    extra_buildx_args=()
    if [[ ! -z "${EXPORT_DOCKER_CACHE}" ]]; then
        extra_buildx_args+=(--cache-to type=registry,ref=denzp/cargo-container-tools:cache,mode=max)
    fi

    docker buildx build --load \
        -f cargo-container-tools/Cargo.toml . \
        --tag localhost:10395/denzp/cargo-container-tools:local \
        --cache-from type=registry,ref=denzp/cargo-container-tools:cache \
        "${extra_buildx_args[@]}" 2>&3

    echo -e '# \rbuilding the container-tools docker image... done' >&3
}

function build_frontend_image() {
    echo -e '# \rbuilding the frontend docker image...' >&3

    extra_buildx_args=()
    if [[ ! -z "${EXPORT_DOCKER_CACHE}" ]]; then
        extra_buildx_args+=(--cache-to type=registry,ref=denzp/cargo-wharf-frontend:cache,mode=max)
    fi

    docker buildx build --load \
        -f cargo-wharf-frontend/Cargo.toml . \
        --tag localhost:10395/denzp/cargo-wharf-frontend:local \
        --build-arg manifest-path=cargo-wharf-frontend/Cargo.toml \
        --build-arg features=container-tools-testing \
        "${extra_buildx_args[@]}" 2>&3

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
