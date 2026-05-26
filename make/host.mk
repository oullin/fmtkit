host-format: ## Run the dockerized full formatter pipeline against ARGS (default `.`)
	@# Format files inside the shared go-fmt container; forwards ARGS as target paths.
	@GO_FMT_IMAGE='$(strip $(GO_FMT_IMAGE))' \
		GO_FMT_CACHE_VOLUME='$(strip $(GO_FMT_CACHE_VOLUME))' \
		GO_FMT_PROJECT_DIR='$(strip $(GO_FMT_PROJECT_DIR))' \
		./scripts/go-fmt-host.sh format $(ARGS)

host-format-go: ## Run the dockerized Go-only formatter against ARGS (default `.`)
	@GO_FMT_IMAGE='ghcr.io/oullin/go-fmt:latest-go' \
		GO_FMT_CACHE_VOLUME='$(strip $(GO_FMT_CACHE_VOLUME))' \
		GO_FMT_PROJECT_DIR='$(strip $(GO_FMT_PROJECT_DIR))' \
		./scripts/go-fmt-host.sh format $(ARGS)

host-format-ts: ## Run the dockerized Node/TS-only formatter against ARGS (default `.`)
	@GO_FMT_IMAGE='ghcr.io/oullin/go-fmt:latest-node-ts' \
		GO_FMT_CACHE_VOLUME='$(strip $(GO_FMT_CACHE_VOLUME))' \
		GO_FMT_PROJECT_DIR='$(strip $(GO_FMT_PROJECT_DIR))' \
		./scripts/go-fmt-host.sh $(ARGS)

host-format-full: ## Run the dockerized full formatter against ARGS (default `.`)
	@GO_FMT_IMAGE='ghcr.io/oullin/go-fmt:latest-full' \
		GO_FMT_CACHE_VOLUME='$(strip $(GO_FMT_CACHE_VOLUME))' \
		GO_FMT_PROJECT_DIR='$(strip $(GO_FMT_PROJECT_DIR))' \
		./scripts/go-fmt-host.sh format $(ARGS)

host-check: ## Run the dockerized `go-fmt check` against ARGS (default `.`)
	@# Verify formatting inside the shared go-fmt container without writing changes.
	@GO_FMT_IMAGE='$(strip $(GO_FMT_IMAGE))' \
		GO_FMT_CACHE_VOLUME='$(strip $(GO_FMT_CACHE_VOLUME))' \
		GO_FMT_PROJECT_DIR='$(strip $(GO_FMT_PROJECT_DIR))' \
		./scripts/go-fmt-host.sh go check $(ARGS)

host-version: ## Print the dockerized go-fmt version
	@# Show the version baked into the configured GO_FMT_IMAGE.
	@GO_FMT_IMAGE='$(strip $(GO_FMT_IMAGE))' \
		GO_FMT_CACHE_VOLUME='$(strip $(GO_FMT_CACHE_VOLUME))' \
		GO_FMT_PROJECT_DIR='$(strip $(GO_FMT_PROJECT_DIR))' \
		./scripts/go-fmt-host.sh go version

host-help: ## Print the dockerized go-fmt usage
	@# Show the upstream CLI help text from the configured GO_FMT_IMAGE.
	@GO_FMT_IMAGE='$(strip $(GO_FMT_IMAGE))' \
		GO_FMT_CACHE_VOLUME='$(strip $(GO_FMT_CACHE_VOLUME))' \
		GO_FMT_PROJECT_DIR='$(strip $(GO_FMT_PROJECT_DIR))' \
		./scripts/go-fmt-host.sh go help
