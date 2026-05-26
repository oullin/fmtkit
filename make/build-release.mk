build: ## Build a host-native binary into ./storage/bin
	@# Compile the current version into a local binary for the host platform.
	@VERSION='$(strip $(VERSION))' BUILD_DIR='$(strip $(BUILD_DIR))' BIN='$(strip $(BIN))' ./scripts/build.sh

release: ## Build release binaries into $(DIST_DIR)
	@# Produce distributable binaries for every configured GOOS/GOARCH target.
	@VERSION='$(strip $(VERSION))' DIST_DIR='$(strip $(DIST_DIR))' RELEASE_PLATFORMS='$(strip $(RELEASE_PLATFORMS))' ./scripts/release.sh

install: ## Install the CLI with go install
	@# Install the CLI into the active Go bin directory for local use.
	./scripts/with-storage-env.sh go -C $(GO_WORKDIR) install $(CMD)
