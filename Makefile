SHELL := /bin/bash
.DEFAULT_GOAL := help

# fmtkit formats itself with the binary it ships: these targets build fmtkit and
# its Go helper from this checkout and run them.
ARGS ?= .

.PHONY: help build format format-all check lint test version

help: ## Show the available targets
	@printf 'fmtkit\n\n'
	@printf '  make build       Build fmtkit and the Go helper into storage/\n'
	@printf '  make format      Format ARGS (default ".", the whole repository)\n'
	@printf '  make format-all  Format the whole repository\n'
	@printf '  make check       Check ARGS without writing anything\n'
	@printf '  make lint        rustfmt, clippy, gofmt and go vet\n'
	@printf '  make test        Run the Rust and Go test suites\n'
	@printf '  make version     Print the version the working tree builds as\n'
	@printf '\nAdd --ts or --go to ARGS to run only that lane.\n'
	@printf '\nVariables: ARGS\n'

build: ## Build fmtkit and the Go helper
	@./scripts/task.sh build

format: ## Format ARGS
	@./scripts/task.sh format $(ARGS)

format-all: ## Format the whole repository
	@./scripts/task.sh fmtkit format --all

check: ## Check ARGS without writing anything
	@./scripts/task.sh fmtkit check --all $(filter-out .,$(ARGS))

lint: ## rustfmt, clippy, gofmt and go vet
	@./scripts/task.sh lint

test: ## Run the Rust and Go test suites
	@./scripts/task.sh test

version: ## Print the version the working tree builds as
	@./scripts/task.sh fmtkit version
