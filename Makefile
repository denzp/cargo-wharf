TOOLS_VERSION    = $(shell cargo pkgid --manifest-path=cargo-container-tools/Cargo.toml | cut -d\# -f2 | cut -d: -f2)
FRONTEND_VERSION = $(shell cargo pkgid --manifest-path=buildkit-cargo-frontend/Cargo.toml | cut -d\# -f2 | cut -d: -f2)

with-docker:   docker-container-tools   docker-cargo-frontend
with-buildctl: buildctl-container-tools buildctl-cargo-frontend

docker-container-tools:
	docker build -t denzp/cargo-container-tools:$(TOOLS_VERSION) -f cargo-container-tools/Dockerfile .

docker-cargo-frontend:
	docker build -t denzp/buildkit-cargo-frontend:$(FRONTEND_VERSION) -f buildkit-cargo-frontend/Dockerfile .

buildctl-container-tools:
	buildctl build --frontend=dockerfile.v0 --local context=. --local dockerfile=cargo-container-tools --output type=image,push=false,docker.io/denzp/cargo-container-tools:$(TOOLS_VERSION)

buildctl-cargo-frontend:
	buildctl build --frontend=dockerfile.v0 --local context=. --local dockerfile=buildkit-cargo-frontend --output type=image,push=false,name=denzp/buildkit-cargo-frontend:$(FRONTEND_VERSION)

buildctl-cargo-frontend-devel:
	buildctl build --frontend gateway.v0 --opt=gateway-devel=true --opt=source=dockerfile.v0 --local gateway-context=. --local gateway-dockerfile=buildkit-cargo-frontend --local context=. --local=dockerfile=. --output type=local,dest=.debug-output
