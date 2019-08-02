# TOOLS_VERSION    = v$(shell cargo pkgid --manifest-path=cargo-container-tools/Cargo.toml | cut -d\# -f2 | cut -d: -f2)
# FRONTEND_VERSION = v$(shell cargo pkgid --manifest-path=cargo-buildkit-frontend/Cargo.toml | cut -d\# -f2 | cut -d: -f2)

buildctl-container-tools:
	buildctl build --frontend gateway.v0 --opt source=docker/dockerfile:1.1-experimental --local context=. --local dockerfile=cargo-container-tools --output type=image,name=docker.io/denzp/cargo-container-tools:local

buildctl-cargo-frontend:
	buildctl build --frontend gateway.v0 --opt source=docker/dockerfile:1.1-experimental --local context=. --local dockerfile=cargo-buildkit-frontend --output type=image,name=docker.io/denzp/cargo-buildkit-frontend:local

buildctl-cargo-frontend-example: buildctl-container-tools buildctl-cargo-frontend
	buildctl build --frontend gateway.v0 --opt source=denzp/cargo-buildkit-frontend:local --local context=examples/workspace --output type=local,dest=.debug-output

buildctl-cargo-frontend-example-debug: buildctl-container-tools buildctl-cargo-frontend
	buildctl build --frontend gateway.v0 --opt source=denzp/cargo-buildkit-frontend:local --local context=examples/workspace  --opt debug=build-plan,build-graph,llb --output type=local,dest=.debug-output
