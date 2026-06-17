SHELL := /bin/bash
.DEFAULT_GOAL := help

include make/config.mk
include make/help.mk
include make/format.mk
include make/images.mk
include make/host.mk
include make/build-release.mk
include make/test.mk
include make/clean.mk

.PHONY: help format format-all format-run image-go image-node-ts image-full docker-clean build release test test-race test-short vet gofmt install clean host-format host-format-go host-format-ts host-format-full host-check host-version host-help
