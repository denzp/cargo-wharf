TOOLS_VERSION    = v$(shell cargo pkgid --manifest-path=cargo-container-tools/Cargo.toml | cut -d\# -f2 | cut -d: -f2)
FRONTEND_VERSION = v$(shell cargo pkgid --manifest-path=cargo-wharf-frontend/Cargo.toml | cut -d\# -f2 | cut -d: -f2)

.PHONY: local-images
local-images: local-container-tools local-cargo-wharf-frontend
remote-images: remote-container-tools remote-cargo-wharf-frontend

local-container-tools:
	docker build -f cargo-container-tools/Dockerfile -t denzp/cargo-container-tools:local .

local-cargo-wharf-frontend:
	docker build -f cargo-wharf-frontend/Dockerfile -t denzp/cargo-wharf-frontend:local .

remote-container-tools: local-container-tools
	docker tag denzp/cargo-container-tools:local denzp/cargo-container-tools:$(TOOLS_VERSION)

remote-cargo-wharf-frontend: local-cargo-wharf-frontend
	docker tag denzp/cargo-wharf-frontend:local denzp/cargo-wharf-frontend:$(TOOLS_VERSION)
