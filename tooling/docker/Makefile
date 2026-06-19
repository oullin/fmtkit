SHELL := /bin/bash
.DEFAULT_GOAL := help

ARGS ?= .
VERSION ?= $(shell git describe --tags --always --dirty 2>/dev/null || echo dev)
FORMATTER_IMAGE ?= go-fmt-full:local
FORMATTER_DOCKERFILE ?= docker/Dockerfile.full
FORMATTER_BUILD ?= auto
GO_IMAGE ?= go-fmt-go:local
NODE_TS_IMAGE ?= go-fmt-node-ts:local
FULL_IMAGE ?= go-fmt-full:local
GO_FMT_IMAGE ?= ghcr.io/oullin/go-fmt:latest
GO_FMT_CACHE_VOLUME ?= go-fmt-cache
GO_FMT_PROJECT_DIR ?= $(CURDIR)

export VERSION FORMATTER_IMAGE FORMATTER_DOCKERFILE FORMATTER_BUILD
export GO_IMAGE NODE_TS_IMAGE FULL_IMAGE
export GO_FMT_IMAGE GO_FMT_CACHE_VOLUME GO_FMT_PROJECT_DIR

.PHONY: help format format-all check image-go image-node-ts image-full docker-clean host-format host-format-go host-format-ts host-format-full host-check host-version host-help

help: ## Show Docker compatibility targets
	@printf 'go-fmt Docker compatibility targets\n\n'
	@printf '  make format         Dockerized full formatter against ARGS (default ".")\n'
	@printf '  make format-all     Dockerized full formatter against the whole repository\n'
	@printf '  make check          Dockerized go-fmt check against ARGS (default ".")\n'
	@printf '  make image-go       Build the local Go-only formatter image\n'
	@printf '  make image-node-ts  Build the local Node/TS-only formatter image\n'
	@printf '  make image-full     Build the local full formatter image\n'
	@printf '  make docker-clean   Remove go-fmt local images and cache volume\n'
	@printf '\nVariables: ARGS VERSION FORMATTER_IMAGE FORMATTER_BUILD GO_FMT_IMAGE GO_FMT_CACHE_VOLUME\n'

format: ## Run the dockerized full formatter pipeline against ARGS
	@./scripts/format-docker.sh format $(ARGS)

format-all: ## Run the dockerized full formatter pipeline against the whole repository
	@./scripts/format-docker.sh format-all

check: ## Run dockerized go-fmt check against ARGS
	@./scripts/format-docker.sh check $(ARGS)

image-go: ## Build the local Go-only formatter image
	@./scripts/docker-image.sh go

image-node-ts: ## Build the local Node/TS-only formatter image
	@./scripts/docker-image.sh node-ts

image-full: ## Build the local full Go + TS formatter image
	@./scripts/docker-image.sh full

docker-clean: ## Remove go-fmt's local images and cache volume
	@./scripts/docker-image.sh clean

host-format: ## Run the published dockerized full formatter pipeline against ARGS
	@./scripts/go-fmt-host.sh format $(ARGS)

host-format-go: ## Run the published dockerized Go-only formatter against ARGS
	@GO_FMT_IMAGE=ghcr.io/oullin/go-fmt:latest-go ./scripts/go-fmt-host.sh format $(ARGS)

host-format-ts: ## Run the published dockerized Node/TS-only formatter against ARGS
	@GO_FMT_IMAGE=ghcr.io/oullin/go-fmt:latest-node-ts ./scripts/go-fmt-host.sh $(ARGS)

host-format-full: ## Run the published dockerized full formatter against ARGS
	@GO_FMT_IMAGE=ghcr.io/oullin/go-fmt:latest-full ./scripts/go-fmt-host.sh format $(ARGS)

host-check: ## Run published dockerized go-fmt check against ARGS
	@./scripts/go-fmt-host.sh go check $(ARGS)

host-version: ## Print the published dockerized go-fmt version
	@./scripts/go-fmt-host.sh go version

host-help: ## Print the published dockerized go-fmt usage
	@./scripts/go-fmt-host.sh go help
