function install_registry {
    docker run -d -p 10395:5000 --restart=always --name cargo-wharf-integration-tests-registry registry:2
    sleep 2
}

function remove_registry {
    docker rm -f cargo-wharf-integration-tests-registry
}
