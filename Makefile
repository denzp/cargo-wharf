.PHONY: local-images
local-images: local-container-tools local-wharf-frontend

.PHONY: local-container-tools
local-container-tools:
	docker buildx build --load -f cargo-container-tools/Cargo.toml . \
		--tag denzp/cargo-container-tools:local \
		--cache-from type=registry,ref=denzp/cargo-container-tools:cache

.PHONY: local-wharf-frontend
local-wharf-frontend:
	docker buildx build --load -f cargo-wharf-frontend/Cargo.toml . \
		--tag denzp/cargo-wharf-frontend:local \
		--cache-from type=registry,ref=denzp/cargo-wharf-frontend:cache
