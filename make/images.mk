image-go: ## Build the local Go-only formatter image
	@docker build -f docker/Dockerfile.go -t '$(strip $(GO_IMAGE))' .

image-node-ts: ## Build the local Node/TS-only formatter image
	@docker build -f docker/Dockerfile.node-ts -t '$(strip $(NODE_TS_IMAGE))' .

image-full: ## Build the local full Go + TS formatter image
	@docker build -f docker/Dockerfile.full -t '$(strip $(FULL_IMAGE))' .
