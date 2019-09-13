# TOOLS_VERSION    = v$(shell cargo pkgid --manifest-path=cargo-container-tools/Cargo.toml | cut -d\# -f2 | cut -d: -f2)
# FRONTEND_VERSION = v$(shell cargo pkgid --manifest-path=cargo-buildkit-frontend/Cargo.toml | cut -d\# -f2 | cut -d: -f2)

container-tools:
	docker build -f cargo-container-tools/Dockerfile -t denzp/cargo-container-tools:local .

cargo-frontend:
	docker build -f cargo-buildkit-frontend/Dockerfile -t denzp/cargo-buildkit-frontend:local .

example-workspace: container-tools cargo-frontend
	docker build -f examples/workspace/Cargo.toml -t cargo-wharf/example-workspace examples/workspace

example-workspace-debug: container-tools cargo-frontend
	docker build -f examples/workspace/Cargo.toml -t cargo-wharf/example-workspace examples/workspace -o type=local,dest=debug-output --build-arg debug=config,build-plan,build-graph,llb

example-single-bin: container-tools cargo-frontend
	docker build -f examples/single-bin/Cargo.toml -t cargo-wharf/example-single-bin examples/single-bin

example-single-bin-debug: container-tools cargo-frontend
	docker build -f examples/single-bin/Cargo.toml -t cargo-wharf/example-single-bin examples/single-bin -o type=local,dest=debug-output --build-arg debug=config,build-plan,build-graph,llb
