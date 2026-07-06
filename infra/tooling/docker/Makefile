SHELL := /bin/bash
.DEFAULT_GOAL := help

ARGS ?= .
VERSION ?= $(shell git describe --tags --always --dirty 2>/dev/null || echo dev)
FORMATTER_IMAGE ?= fmtkit-full:local
FORMATTER_DOCKERFILE ?= infra/docker/Dockerfile.full
FORMATTER_BUILD ?= auto
GO_IMAGE ?= fmtkit-go:local
NODE_TS_IMAGE ?= fmtkit-node-ts:local
FULL_IMAGE ?= fmtkit-full:local
FMTKIT_IMAGE ?= ghcr.io/oullin/fmtkit:latest
FMTKIT_CACHE_VOLUME ?= fmtkit-cache
FMTKIT_PROJECT_DIR ?= $(CURDIR)

export VERSION FORMATTER_IMAGE FORMATTER_DOCKERFILE FORMATTER_BUILD
export GO_IMAGE NODE_TS_IMAGE FULL_IMAGE
export FMTKIT_IMAGE FMTKIT_CACHE_VOLUME FMTKIT_PROJECT_DIR

.PHONY: help format format-all check image-go image-node-ts image-full docker-clean host-format host-format-go host-format-ts host-format-full host-check host-version host-help

help: ## Show Docker compatibility targets
	@printf 'fmtkit Docker compatibility targets\n\n'
	@printf '  make format         Dockerized full formatter against ARGS (default ".")\n'
	@printf '  make format-all     Dockerized full formatter against the whole repository\n'
	@printf '  make check          Dockerized fmtkit check against ARGS (default ".")\n'
	@printf '  make image-go       Build the local Go-only formatter image\n'
	@printf '  make image-node-ts  Build the local Node/TS-only formatter image\n'
	@printf '  make image-full     Build the local full formatter image\n'
	@printf '  make docker-clean   Remove fmtkit local images and cache volume\n'
	@printf '\nVariables: ARGS VERSION FORMATTER_IMAGE FORMATTER_BUILD FMTKIT_IMAGE FMTKIT_CACHE_VOLUME\n'

format: ## Run the dockerized full formatter pipeline against ARGS
	@./infra/scripts/tasks/format-docker.sh format $(ARGS)

format-all: ## Run the dockerized full formatter pipeline against the whole repository
	@./infra/scripts/tasks/format-docker.sh format-all

check: ## Run dockerized fmtkit check against ARGS
	@./infra/scripts/tasks/format-docker.sh check $(ARGS)

image-go: ## Build the local Go-only formatter image
	@./infra/scripts/tasks/docker-image.sh go

image-node-ts: ## Build the local Node/TS-only formatter image
	@./infra/scripts/tasks/docker-image.sh node-ts

image-full: ## Build the local full Go + TS formatter image
	@./infra/scripts/tasks/docker-image.sh full

docker-clean: ## Remove fmtkit's local images and cache volume
	@./infra/scripts/tasks/docker-image.sh clean

host-format: ## Run the published dockerized full formatter pipeline against ARGS
	@./infra/scripts/tasks/fmtkit-host.sh format $(ARGS)

host-format-go: ## Run the published dockerized Go-only formatter against ARGS
	@FMTKIT_IMAGE=ghcr.io/oullin/fmtkit:latest-go ./infra/scripts/tasks/fmtkit-host.sh format $(ARGS)

host-format-ts: ## Run the published dockerized Node/TS-only formatter against ARGS
	@FMTKIT_IMAGE=ghcr.io/oullin/fmtkit:latest-node-ts ./infra/scripts/tasks/fmtkit-host.sh $(ARGS)

host-format-full: ## Run the published dockerized full formatter against ARGS
	@FMTKIT_IMAGE=ghcr.io/oullin/fmtkit:latest-full ./infra/scripts/tasks/fmtkit-host.sh format $(ARGS)

host-check: ## Run published dockerized fmtkit check against ARGS
	@./infra/scripts/tasks/fmtkit-host.sh go check $(ARGS)

host-version: ## Print the published dockerized fmtkit version
	@./infra/scripts/tasks/fmtkit-host.sh go version

host-help: ## Print the published dockerized fmtkit usage
	@./infra/scripts/tasks/fmtkit-host.sh go help
