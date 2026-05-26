format: ## Apply formatter changes to ARGS
	@# Run the full Dockerized formatter pipeline against ARGS.
	@$(MAKE) format-run FORMAT_ARGS='$(ARGS)'

format-all: ## Build and run the full Dockerized formatter pipeline against ARGS
	@# Run the full Dockerized formatter pipeline against the whole mounted repository.
	@$(MAKE) format-run FORMAT_ARGS='.'

format-run:
	@# Ensure the local formatter image exists and matches VERSION, then run formatting.
	@bold=''; dim=''; cyan=''; green=''; red=''; reset=''; \
		if [[ -n "$${FORCE_COLOR:-}" || (-z "$${NO_COLOR:-}" && -t 2) ]]; then \
			bold=$$(printf '\033[1m'); dim=$$(printf '\033[2m'); cyan=$$(printf '\033[36m'); green=$$(printf '\033[32m'); red=$$(printf '\033[31m'); reset=$$(printf '\033[0m'); \
		fi; \
		image='$(strip $(FORMATTER_IMAGE))'; \
		dockerfile='$(strip $(FORMATTER_DOCKERFILE))'; \
		policy='$(strip $(FORMATTER_BUILD))'; \
		expected_version='go-fmt $(strip $(VERSION))'; \
		build_reason=''; \
		printf '\n%s==>%s %sEnsuring formatter image%s\n' "$$cyan" "$$reset" "$$bold" "$$reset"; \
		printf '    %s%-12s%s %s\n' "$$dim" image "$$reset" "$$image"; \
		printf '    %s%-12s%s %s\n' "$$dim" policy "$$reset" "$$policy"; \
		case "$$policy" in \
			auto) \
				if ! docker image inspect "$$image" >/dev/null 2>&1; then \
					build_reason='missing'; \
				else \
					image_version="$$(docker run --rm "$$image" version 2>/dev/null || true)"; \
					if [[ "$$image_version" != "$$expected_version" ]]; then \
						build_reason='version changed'; \
						printf '    %s%-12s%s %s\n' "$$dim" current "$$reset" "$${image_version:-unknown}"; \
						printf '    %s%-12s%s %s\n' "$$dim" expected "$$reset" "$$expected_version"; \
					fi; \
				fi; \
				;; \
			always) \
				build_reason='forced'; \
				;; \
			never) \
				if ! docker image inspect "$$image" >/dev/null 2>&1; then \
					printf '\n%s!!%s %sFormatter image is missing and FORMATTER_BUILD=never%s\n' "$$red" "$$reset" "$$bold" "$$reset" >&2; \
					printf 'Build it with `make image-full` or rerun with FORMATTER_BUILD=auto.\n' >&2; \
					exit 1; \
				fi; \
				printf '    %s%-12s%s %s%s%s\n' "$$green" status "$$reset" "$$green" skipped "$$reset"; \
				;; \
			*) \
				printf '\n%s!!%s %sInvalid FORMATTER_BUILD value: %s%s\n' "$$red" "$$reset" "$$bold" "$$policy" "$$reset" >&2; \
				printf 'Expected one of: auto, always, never.\n' >&2; \
				exit 2; \
				;; \
		esac; \
		if [[ "$$policy" == auto && -z "$$build_reason" ]]; then \
			printf '    %s%-12s%s %s%s%s\n' "$$green" status "$$reset" "$$green" cached "$$reset"; \
		elif [[ -n "$$build_reason" ]]; then \
			printf '    %s%-12s%s %s\n' "$$dim" reason "$$reset" "$$build_reason"; \
			build_log="$$(mktemp)"; \
			if docker build --build-arg VERSION='$(strip $(VERSION))' -f "$$dockerfile" -t "$$image" . >"$$build_log" 2>&1; then \
				printf '    %s%-12s%s %s%s%s\n' "$$green" status "$$reset" "$$green" built "$$reset"; \
				rm -f "$$build_log"; \
			else \
				status="$$?"; \
				printf '\n%s!!%s %sDocker build failed%s\n' "$$red" "$$reset" "$$bold" "$$reset" >&2; \
				cat "$$build_log" >&2; \
				rm -f "$$build_log"; \
				exit "$$status"; \
			fi; \
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
