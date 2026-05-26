format: ## Apply formatter changes to ARGS
	@# Run the full Dockerized formatter pipeline against ARGS.
	@$(MAKE) format-run FORMAT_ARGS='$(ARGS)'

format-all: ## Build and run the full Dockerized formatter pipeline against ARGS
	@# Run the full Dockerized formatter pipeline against the whole mounted repository.
	@$(MAKE) format-run FORMAT_ARGS='.'

format-run:
	@# Build the local formatter image, run TS/Vue support first, then run Go formatting.
	@bold=''; dim=''; cyan=''; green=''; red=''; reset=''; \
		if [[ -n "$${FORCE_COLOR:-}" || (-z "$${NO_COLOR:-}" && -t 2) ]]; then \
			bold=$$(printf '\033[1m'); dim=$$(printf '\033[2m'); cyan=$$(printf '\033[36m'); green=$$(printf '\033[32m'); red=$$(printf '\033[31m'); reset=$$(printf '\033[0m'); \
		fi; \
		printf '\n%s==>%s %sBuilding formatter image%s\n' "$$cyan" "$$reset" "$$bold" "$$reset"; \
		printf '    %s%-12s%s %s\n' "$$dim" image "$$reset" '$(strip $(FORMATTER_IMAGE))'; \
			build_log="$$(mktemp)"; \
			if docker build -f '$(strip $(FORMATTER_DOCKERFILE))' -t '$(strip $(FORMATTER_IMAGE))' . >"$$build_log" 2>&1; then \
			printf '    %s%-12s%s %s%s%s\n' "$$green" status "$$reset" "$$green" built "$$reset"; \
			rm -f "$$build_log"; \
		else \
			status="$$?"; \
			printf '\n%s!!%s %sDocker build failed%s\n' "$$red" "$$reset" "$$bold" "$$reset" >&2; \
			cat "$$build_log" >&2; \
			rm -f "$$build_log"; \
			exit "$$status"; \
		fi
	@bold=''; dim=''; cyan=''; reset=''; \
		if [[ -n "$${FORCE_COLOR:-}" || (-z "$${NO_COLOR:-}" && -t 2) ]]; then \
			bold=$$(printf '\033[1m'); dim=$$(printf '\033[2m'); cyan=$$(printf '\033[36m'); reset=$$(printf '\033[0m'); \
		fi; \
		printf '\n%s==>%s %sStarting formatter container%s\n' "$$cyan" "$$reset" "$$bold" "$$reset"; \
		printf '    %s%-12s%s %s\n' "$$dim" workdir "$$reset" '$(strip $(GO_FMT_PROJECT_DIR))'
	@docker run --rm \
		-v '$(strip $(GO_FMT_PROJECT_DIR)):/work' \
		-v '$(strip $(GO_FMT_CACHE_VOLUME)):/cache' \
		-w /work \
		-e 'HOST_PROJECT_PATH=$(strip $(GO_FMT_PROJECT_DIR))' \
		-e GOCACHE=/cache/go-build \
		-e GOPATH=/cache/gopath \
			-e GOMODCACHE=/cache/gopath/pkg/mod \
			'$(strip $(FORMATTER_IMAGE))' format $(FORMAT_ARGS)
