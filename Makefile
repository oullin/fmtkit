SHELL := /bin/bash
.DEFAULT_GOAL := help

APP := go-fmt
CMD := ./cmd/fmt
GO_WORKDIR := packages/driver

ARGS ?= . ## With '.', format changed tracked/untracked files first, then widen semantic formatting if needed; set a path to target a specific subtree
VERSION ?= $(shell git describe --tags --always --dirty 2>/dev/null || echo dev) ## Build version injected into binaries
CGO_ENABLED ?= 0 ## CGO setting used for build and release
BUILD_DIR ?= storage/bin ## Directory for local build binaries
BIN ?= $(BUILD_DIR)/$(APP)
DIST_DIR ?= storage/dist ## Directory for release binaries
DIST_TEST_DIR ?= storage/dist-test ## Directory for test build artifacts
RELEASE_PLATFORMS ?= darwin/amd64 darwin/arm64 linux/amd64 linux/arm64 ## Space-separated GOOS/GOARCH release targets
FORMATTER_IMAGE ?= go-fmt-full:local ## Local Docker image used by make format-all
FORMATTER_DOCKERFILE ?= docker/Dockerfile.full ## Dockerfile used for the local formatter image
GO_IMAGE ?= go-fmt-go:local ## Local Go-only Docker image
NODE_TS_IMAGE ?= go-fmt-node-ts:local ## Local Node/TS-only Docker image
FULL_IMAGE ?= go-fmt-full:local ## Local full Docker image
GO_FMT_IMAGE ?= ghcr.io/oullin/go-fmt:latest ## Container image used by host-* targets
GO_FMT_CACHE_VOLUME ?= go-fmt-cache ## Named Docker volume reused across host-* runs
GO_FMT_PROJECT_DIR ?= $(CURDIR) ## Project root bind-mounted into the container

.PHONY: help format format-all format-run image-go image-node-ts image-full build release test test-race test-short vet gofmt install clean host-format host-format-go host-format-ts host-format-full host-check host-version host-help

help: ## Show available targets and override variables
	@# Parse Make metadata and render styled help output through the dedicated helper script.
	@./scripts/help.sh $(MAKEFILE_LIST)

format: ## Apply formatter changes to ARGS
	@# Run the full Dockerized formatter pipeline against ARGS.
	@$(MAKE) format-run FORMAT_ARGS='$(ARGS)'

format-all: ## Build and run the full Dockerized formatter pipeline against ARGS
	@# Run the full Dockerized formatter pipeline against the whole mounted repository.
	@$(MAKE) format-run FORMAT_ARGS='.'

format-run:
	@# Build the local formatter image, run TS/Vue support first, then run Go formatting.
	@bold=''; dim=''; cyan=''; green=''; red=''; reset=''; \
		if [[ -n "$${FORCE_COLOR:-}" || (-z "$${NO_COLOR:-}" && -t 2) ]]; then \
			bold=$$(printf '\033[1m'); dim=$$(printf '\033[2m'); cyan=$$(printf '\033[36m'); green=$$(printf '\033[32m'); red=$$(printf '\033[31m'); reset=$$(printf '\033[0m'); \
		fi; \
		printf '\n%s==>%s %sBuilding formatter image%s\n' "$$cyan" "$$reset" "$$bold" "$$reset"; \
		printf '    %s%-12s%s %s\n' "$$dim" image "$$reset" '$(strip $(FORMATTER_IMAGE))'; \
			build_log="$$(mktemp)"; \
			if docker build -f '$(strip $(FORMATTER_DOCKERFILE))' -t '$(strip $(FORMATTER_IMAGE))' . >"$$build_log" 2>&1; then \
			printf '    %s%-12s%s %s%s%s\n' "$$green" status "$$reset" "$$green" built "$$reset"; \
			rm -f "$$build_log"; \
		else \
			status="$$?"; \
			printf '\n%s!!%s %sDocker build failed%s\n' "$$red" "$$reset" "$$bold" "$$reset" >&2; \
			cat "$$build_log" >&2; \
			rm -f "$$build_log"; \
			exit "$$status"; \
		fi
	@bold=''; dim=''; cyan=''; reset=''; \
		if [[ -n "$${FORCE_COLOR:-}" || (-z "$${NO_COLOR:-}" && -t 2) ]]; then \
			bold=$$(printf '\033[1m'); dim=$$(printf '\033[2m'); cyan=$$(printf '\033[36m'); reset=$$(printf '\033[0m'); \
		fi; \
		printf '\n%s==>%s %sStarting formatter container%s\n' "$$cyan" "$$reset" "$$bold" "$$reset"; \
		printf '    %s%-12s%s %s\n' "$$dim" workdir "$$reset" '$(strip $(GO_FMT_PROJECT_DIR))'
	@docker run --rm \
		-v '$(strip $(GO_FMT_PROJECT_DIR)):/work' \
		-v '$(strip $(GO_FMT_CACHE_VOLUME)):/cache' \
		-w /work \
		-e 'HOST_PROJECT_PATH=$(strip $(GO_FMT_PROJECT_DIR))' \
		-e GOCACHE=/cache/go-build \
		-e GOPATH=/cache/gopath \
			-e GOMODCACHE=/cache/gopath/pkg/mod \
			'$(strip $(FORMATTER_IMAGE))' format $(FORMAT_ARGS)

image-go: ## Build the local Go-only formatter image
	@docker build -f docker/Dockerfile.go -t '$(strip $(GO_IMAGE))' .

image-node-ts: ## Build the local Node/TS-only formatter image
	@docker build -f docker/Dockerfile.node-ts -t '$(strip $(NODE_TS_IMAGE))' .

image-full: ## Build the local full Go + TS formatter image
	@docker build -f docker/Dockerfile.full -t '$(strip $(FULL_IMAGE))' .

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

build: ## Build a host-native binary into ./storage/bin
	@# Compile the current version into a local binary for the host platform.
	@VERSION='$(strip $(VERSION))' BUILD_DIR='$(strip $(BUILD_DIR))' BIN='$(strip $(BIN))' ./scripts/build.sh

release: ## Build release binaries into $(DIST_DIR)
	@# Produce distributable binaries for every configured GOOS/GOARCH target.
	@VERSION='$(strip $(VERSION))' DIST_DIR='$(strip $(DIST_DIR))' RELEASE_PLATFORMS='$(strip $(RELEASE_PLATFORMS))' ./scripts/release.sh

test: ## Run all tests with verbose output
	@# Execute the workspace test suite through the package-level test script.
	pnpm test

test-race: ## Run all tests with the race detector
	@# Run Go tests with race detection enabled for concurrency-sensitive changes.
	for dir in packages/formatter packages/vet packages/driver; do \
		CGO_ENABLED=1 ./scripts/with-storage-env.sh go -C $$dir test ./... -race -v; \
	done

test-short: ## Run tests in short mode
	@# Run the fast Go test subset intended for quick local verification.
	for dir in packages/formatter packages/vet packages/driver; do \
		./scripts/with-storage-env.sh go -C $$dir test ./... -short; \
	done

vet: ## Run go vet across the module
	@# Run static analysis checks configured for the repository workspace.
	pnpm vet

gofmt: ## Rewrite Go source formatting in the repository
	@# Normalize Go source formatting across tracked repository files.
	@./scripts/fmt-source.sh

install: ## Install the CLI with go install
	@# Install the CLI into the active Go bin directory for local use.
	./scripts/with-storage-env.sh go -C $(GO_WORKDIR) install $(CMD)

clean: ## Remove build artifacts and clean the Go cache
	@# Remove storage-managed binaries, release outputs, and caches.
	rm -rf $(BUILD_DIR) $(DIST_DIR) $(DIST_TEST_DIR) storage/.cache
	@# Remove workspace dependency installs.
	rm -rf node_modules packages/devx/node_modules packages/formatter/node_modules packages/vet/node_modules packages/driver/node_modules
