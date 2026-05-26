clean: ## Remove build artifacts and clean the Go cache
	@# Remove storage-managed binaries, release outputs, and caches.
	rm -rf $(BUILD_DIR) $(DIST_DIR) $(DIST_TEST_DIR) storage/.cache
	@# Remove workspace dependency installs.
	rm -rf node_modules packages/devx/node_modules packages/formatter/node_modules packages/vet/node_modules packages/driver/node_modules
