.PHONY: local-container-tools local-wharf-frontend

local-container-tools:
	docker buildx build --load -f cargo-container-tools/Cargo.toml . \
		--tag denzp/cargo-container-tools:local \
		--cache-from type=registry,ref=denzp/cargo-container-tools:cache

local-wharf-frontend:
	docker buildx build --load -f cargo-wharf-frontend/Cargo.toml . \
		--tag denzp/cargo-wharf-frontend:local \
		--cache-from type=registry,ref=denzp/cargo-wharf-frontend:cache
