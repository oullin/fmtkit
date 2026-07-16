SHELL := /bin/bash
.DEFAULT_GOAL := help

# fmtkit formats itself with the binary it ships: these targets run the same Go
# orchestrator and bun-compiled TS sidecar that a release carries.
ARGS ?= .

.PHONY: help format format-all check version

help: ## Show the available targets
	@printf 'fmtkit\n\n'
	@printf '  make format      Run the formatter pipeline against ARGS (default ".")\n'
	@printf '  make format-all  Run the formatter pipeline against the whole repository\n'
	@printf '  make check       Run the Go formatter in check mode against ARGS\n'
	@printf '  make version     Print the version the working tree builds as\n'
	@printf '\nAdd --ts or --go to ARGS to run only that half of the pipeline.\n'
	@printf '\nVariables: ARGS\n'

format: ## Run the formatter pipeline against ARGS
	@./infra/scripts/tasks/format.sh $(ARGS)

format-all: ## Run the formatter pipeline against the whole repository
	@./infra/scripts/tasks/fmtkit.sh format-all

check: ## Run the Go formatter in check mode against ARGS
	@./infra/scripts/tasks/fmtkit.sh check $(ARGS)

version: ## Print the version the working tree builds as
	@./infra/scripts/tasks/fmtkit.sh version
