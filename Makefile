# TOOLS_VERSION    = v$(shell cargo pkgid --manifest-path=cargo-container-tools/Cargo.toml | cut -d\# -f2 | cut -d: -f2)
# FRONTEND_VERSION = v$(shell cargo pkgid --manifest-path=cargo-wharf-frontend/Cargo.toml | cut -d\# -f2 | cut -d: -f2)

container-tools:
	docker build -f cargo-container-tools/Dockerfile -t denzp/cargo-container-tools:local .

cargo-frontend:
	docker build -f cargo-wharf-frontend/Dockerfile -t denzp/cargo-wharf-frontend:local .

example-workspace: container-tools cargo-frontend
	docker build -f examples/workspace/Cargo.toml -t cargo-wharf/example-workspace examples/workspace
	docker run --rm cargo-wharf/example-workspace

example-workspace-debug: container-tools cargo-frontend
	docker build -f examples/workspace/Cargo.toml -t cargo-wharf/example-workspace examples/workspace -o type=local,dest=debug-output --build-arg debug=true

example-workspace-test: container-tools cargo-frontend
	docker build -f examples/workspace/Cargo.toml -t cargo-wharf/example-workspace:test examples/workspace --build-arg mode=test
	docker run --rm cargo-wharf/example-workspace:test

example-single-bin: container-tools cargo-frontend
	docker build -f examples/single-bin/Cargo.toml -t cargo-wharf/example-single-bin examples/single-bin
	docker run --rm cargo-wharf/example-single-bin

example-single-bin-debug: container-tools cargo-frontend
	docker build -f examples/single-bin/Cargo.toml -t cargo-wharf/example-single-bin examples/single-bin -o type=local,dest=debug-output --build-arg debug=true

example-single-bin-test: container-tools cargo-frontend
	docker build -f examples/single-bin/Cargo.toml -t cargo-wharf/example-single-bin:test examples/single-bin --build-arg mode=test
	docker run --rm cargo-wharf/example-single-bin:test
