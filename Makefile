.PHONY: version-bump-patch version-bump-minor version-bump-major version-sync version-tag release version

# Get current version from Cargo.toml (package version only)
CURRENT_VERSION := $(shell grep -E '^version\s*=' Cargo.toml | head -1 | cut -d'"' -f2)

# Function to get current version from Cargo.toml (evaluated at runtime)
define get-version
	$(shell grep -E '^version\s*=' Cargo.toml | head -1 | cut -d'"' -f2)
endef

# Version bump targets
version-bump-patch:
	@echo "🔢 Bumping patch version..."
	@cargo install cargo-edit 2>/dev/null || true
	@cargo set-version --bump patch
	@$(MAKE) version-sync
	@echo "✅ Version bumped to $(call get-version)"

version-bump-minor:
	@echo "🔢 Bumping minor version..."
	@cargo install cargo-edit 2>/dev/null || true
	@cargo set-version --bump minor
	@$(MAKE) version-sync
	@echo "✅ Version bumped to $(call get-version)"

version-bump-major:
	@echo "🔢 Bumping major version..."
	@cargo install cargo-edit 2>/dev/null || true
	@cargo set-version --bump major
	@$(MAKE) version-sync
	@echo "✅ Version bumped to $(call get-version)"

# Sync version without bumping
version-sync:
	@echo "🔄 Syncing version to frontend..."
	@cargo build --quiet
	@echo "✅ Version synchronized"

# Create git tag for current version
version-tag:
	@echo "🏷️  Creating git tag for v$(call get-version)..."
	@git add Cargo.toml frontend/package.json
	@git commit -m "Bump version to v$(call get-version)"
	@git tag -a "v$(call get-version)" -m "Release v$(call get-version)"
	@echo "✅ Tag v$(call get-version) created. Run 'git push origin v$(call get-version)' to push."

# Full release process
release: version-bump-patch version-tag
	@echo "🚀 Release v$(call get-version) ready!"
	@echo "📝 Run 'git push origin main && git push origin --tags' to complete release."

# Show current version
version:
	@echo "Current version: $(CURRENT_VERSION)"
