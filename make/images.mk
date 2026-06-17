image-go: ## Build the local Go-only formatter image
	@docker build --build-arg VERSION='$(strip $(VERSION))' -f docker/Dockerfile.go -t '$(strip $(GO_IMAGE))' .

image-node-ts: ## Build the local Node/TS-only formatter image
	@docker build --label 'local.go-fmt.formatter-fingerprint=$(strip $(FORMATTER_FINGERPRINT))' -f docker/Dockerfile.node-ts -t '$(strip $(NODE_TS_IMAGE))' .

image-full: ## Build the local full Go + TS formatter image
	@docker build --build-arg VERSION='$(strip $(VERSION))' --label 'local.go-fmt.formatter-fingerprint=$(strip $(FORMATTER_FINGERPRINT))' -f docker/Dockerfile.full -t '$(strip $(FULL_IMAGE))' .

docker-clean: ## Remove go-fmt's local images, fingerprinted images, and cache volume
	@# Stop and remove containers created from go-fmt local images.
	@# Guard with a non-empty check instead of `xargs -r` (a GNU extension absent on BSD/macOS).
	-@containers=$$(docker ps -aq --filter ancestor='$(strip $(GO_IMAGE))' \
		--filter ancestor='$(strip $(NODE_TS_IMAGE))' \
		--filter ancestor='$(strip $(FULL_IMAGE))'); \
		if [ -n "$$containers" ]; then echo "$$containers" | xargs docker rm -f; fi
	@# Remove the local build images.
	-@docker rmi -f '$(strip $(GO_IMAGE))' '$(strip $(NODE_TS_IMAGE))' '$(strip $(FULL_IMAGE))' 2>/dev/null
	@# Remove any image carrying the project fingerprint label (stale cached builds).
	-@images=$$(docker images -q --filter label=local.go-fmt.formatter-fingerprint | sort -u); \
		if [ -n "$$images" ]; then echo "$$images" | xargs docker rmi -f; fi
	@# Remove the shared cache volume.
	-@docker volume rm '$(strip $(GO_FMT_CACHE_VOLUME))' 2>/dev/null
	@# Drop dangling images left behind by go-fmt rebuilds.
	-@docker image prune -f --filter label=local.go-fmt.formatter-fingerprint
	@# Note: the global Docker build cache cannot be scoped per project; cap it via
	@# ~/.docker/daemon.json (see README "Docker maintenance") rather than pruning it here.
