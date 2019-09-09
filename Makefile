# TOOLS_VERSION    = v$(shell cargo pkgid --manifest-path=cargo-container-tools/Cargo.toml | cut -d\# -f2 | cut -d: -f2)
# FRONTEND_VERSION = v$(shell cargo pkgid --manifest-path=cargo-buildkit-frontend/Cargo.toml | cut -d\# -f2 | cut -d: -f2)

container-tools:
	buildctl build \
		--frontend gateway.v0 \
		--opt source=docker/dockerfile:1.1-experimental \
		--local context=. \
		--local dockerfile=cargo-container-tools \
		--output type=image,name=docker.io/denzp/cargo-container-tools:local

cargo-frontend:
	buildctl build \
		--frontend gateway.v0 \
		--opt source=docker/dockerfile:1.1-experimental \
		--local context=. \
		--local dockerfile=cargo-buildkit-frontend \
		--output type=image,name=docker.io/denzp/cargo-buildkit-frontend:local

docker:
	buildctl build \
		--frontend gateway.v0 \
		--opt source=docker/dockerfile:1.1-experimental \
		--local context=. \
		--local dockerfile=cargo-container-tools \
		--output type=docker,name=denzp/cargo-container-tools:local | docker load
	buildctl build \
		--frontend gateway.v0 \
		--opt source=docker/dockerfile:1.1-experimental \
		--local context=. \
		--local dockerfile=cargo-buildkit-frontend \
		--output type=docker,name=denzp/cargo-buildkit-frontend:local | docker load

example-workspace: container-tools cargo-frontend
	buildctl build \
		--frontend gateway.v0 \
		--opt source=denzp/cargo-buildkit-frontend:local \
		--local context=examples/workspace \
		--output type=docker,name=cargo-wharf/example-workspace | docker load

example-workspace-debug: container-tools cargo-frontend
	buildctl build \
		--frontend gateway.v0 \
		--opt source=denzp/cargo-buildkit-frontend:local \
		--local context=examples/workspace \
		--opt debug=config,build-plan,build-graph,llb \
		--output type=local,dest=debug-output

example-single-bin: container-tools cargo-frontend
	buildctl build \
		--frontend gateway.v0 \
		--opt source=denzp/cargo-buildkit-frontend:local \
		--local context=examples/single-bin \
		--output type=docker,name=cargo-wharf/example-single-bin | docker load

example-single-bin-debug: container-tools cargo-frontend
	buildctl build \
		--frontend gateway.v0 \
		--opt source=denzp/cargo-buildkit-frontend:local \
		--local context=examples/single-bin \
		--opt debug=config,build-plan,build-graph,llb \
		--output type=local,dest=debug-output
