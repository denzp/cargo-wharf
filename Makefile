TOOLS_VERSION    = v$(shell cargo pkgid --manifest-path=cargo-container-tools/Cargo.toml | cut -d\# -f2 | cut -d: -f2)
FRONTEND_VERSION = v$(shell cargo pkgid --manifest-path=cargo-wharf-frontend/Cargo.toml | cut -d\# -f2 | cut -d: -f2)

.PHONY: local-images
local-images: local-container-tools local-wharf-frontend
remote-images: remote-container-tools remote-wharf-frontend

local-container-tools:
	docker build -f cargo-container-tools/Dockerfile -t denzp/cargo-container-tools:local .
	# docker build -f cargo-container-tools/Cargo.toml -t denzp/cargo-container-tools:local .

local-wharf-frontend:
	docker build -f cargo-wharf-frontend/Dockerfile -t denzp/cargo-wharf-frontend:local .
	# docker build -f cargo-wharf-frontend/Cargo.toml -t denzp/cargo-wharf-frontend:local .

remote-container-tools: local-container-tools
	docker tag denzp/cargo-container-tools:local denzp/cargo-container-tools:$(TOOLS_VERSION)

remote-wharf-frontend: local-wharf-frontend
	docker tag denzp/cargo-wharf-frontend:local denzp/cargo-wharf-frontend:$(FRONTEND_VERSION)

push: remote-container-tools remote-wharf-frontend
	docker push denzp/cargo-container-tools:$(TOOLS_VERSION)
	docker push denzp/cargo-wharf-frontend:$(FRONTEND_VERSION)
