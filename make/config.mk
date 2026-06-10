APP := go-fmt
CMD := ./cmd/fmt-go
GO_WORKDIR := packages/driver

ARGS ?= .## With '.', format changed tracked/untracked files first, then widen semantic formatting if needed; set a path to target a specific subtree
VERSION ?= $(shell git describe --tags --always --dirty 2>/dev/null || echo dev)## Build version injected into binaries
CGO_ENABLED ?= 0## CGO setting used for build and release
BUILD_DIR ?= storage/bin## Directory for local build binaries
BIN ?= $(BUILD_DIR)/$(APP)
DIST_DIR ?= storage/dist## Directory for release binaries
DIST_TEST_DIR ?= storage/dist-test## Directory for test build artifacts
RELEASE_PLATFORMS ?= darwin/amd64 darwin/arm64 linux/amd64 linux/arm64## Space-separated GOOS/GOARCH release targets
FORMATTER_IMAGE ?= go-fmt-full:local## Local Docker image used by make format-all
FORMATTER_DOCKERFILE ?= docker/Dockerfile.full## Dockerfile used for the local formatter image
FORMATTER_BUILD ?= auto## Formatter image build policy: auto, always, or never
FORMATTER_FINGERPRINT ?= $(shell { printf '%s\n' '$(FORMATTER_DOCKERFILE)' package.json pnpm-lock.yaml cmd/fmt-ts cmd/fmt-ts-files.sh cmd/fmt-lint cmd/fmt-ts-lint.sh cmd/fmt-all .oxlintrc.json packages/devx/package.json packages/devx/scripts/package.json; find packages/devx/scripts -maxdepth 1 -type f -name '*.ts' -print; } | sort | while IFS= read -r f; do [ -f "$$f" ] && shasum -a 256 "$$f"; done | shasum -a 256 | awk '{print $$1}')## Fingerprint used to invalidate cached local formatter images
GO_IMAGE ?= go-fmt-go:local## Local Go-only Docker image
NODE_TS_IMAGE ?= go-fmt-node-ts:local## Local Node/TS-only Docker image
FULL_IMAGE ?= go-fmt-full:local## Local full Docker image
GO_FMT_IMAGE ?= ghcr.io/oullin/go-fmt:latest## Container image used by host-* targets
GO_FMT_CACHE_VOLUME ?= go-fmt-cache## Named Docker volume reused across host-* runs
GO_FMT_PROJECT_DIR ?= $(CURDIR)## Project root bind-mounted into the container
