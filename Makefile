VERSION = $(shell cargo pkgid --manifest-path=cargo-container-tools/Cargo.toml | cut -d\# -f2 | cut -d: -f2)

docker: docker-container-tools docker-buildkit-frontend
buildkit: buildkit-container-tools buildkit-frontend

docker-container-tools:
	docker build -t denzp/cargo-container-tools:$(VERSION) -f cargo-container-tools/Dockerfile .

docker-buildkit-frontend:
	docker build -t denzp/buildkit-cargo-frontend:$(VERSION) -f buildkit-cargo-frontend/Dockerfile .

buildkit-container-tools:
	buildctl build --frontend=dockerfile.v0 --local context=. --local dockerfile=cargo-container-tools --exporter=image --exporter-opt name=docker.io/denzp/cargo-container-tools

buildkit-frontend:
	buildctl build --frontend=dockerfile.v0 --local context=. --local dockerfile=buildkit-cargo-frontend --exporter=image --exporter-opt name=denzp/buildkit-cargo-frontend:experimental --exporter-opt push=true

buildkit-frontend-devel:
	buildctl build --frontend gateway.v0 --frontend-opt=gateway-devel=true --frontend-opt=source=dockerfile.v0 --local gateway-context=. --local gateway-dockerfile=./buildkit-cargo-frontend --local context=. --local=dockerfile=.
