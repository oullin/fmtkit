help: ## Show available targets and override variables
	@# Parse Make metadata and render styled help output through the dedicated helper script.
	@./scripts/help.sh $(MAKEFILE_LIST)
