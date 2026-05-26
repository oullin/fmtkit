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
