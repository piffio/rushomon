.PHONY: version-bump-patch version-bump-minor version-bump-major version-sync version-tag release version

# Get current version from Cargo.toml (package version only)
CURRENT_VERSION := $(shell grep -E '^version\s*=' Cargo.toml | head -1 | cut -d'"' -f2)

# Version bump targets
version-bump-patch:
	@echo "🔢 Bumping patch version..."
	@cargo install cargo-edit 2>/dev/null || true
	@cargo set-version --bump patch
	@$(MAKE) version-sync
	@NEW_VERSION=$$(grep -E '^version\s*=' Cargo.toml | head -1 | cut -d'"' -f2) && echo "✅ Version bumped to $$NEW_VERSION"

version-bump-minor:
	@echo "🔢 Bumping minor version..."
	@cargo install cargo-edit 2>/dev/null || true
	@cargo set-version --bump minor
	@$(MAKE) version-sync
	@NEW_VERSION=$$(grep -E '^version\s*=' Cargo.toml | head -1 | cut -d'"' -f2) && echo "✅ Version bumped to $$NEW_VERSION"

version-bump-major:
	@echo "🔢 Bumping major version..."
	@cargo install cargo-edit 2>/dev/null || true
	@cargo set-version --bump major
	@$(MAKE) version-sync
	@NEW_VERSION=$$(grep -E '^version\s*=' Cargo.toml | head -1 | cut -d'"' -f2) && echo "✅ Version bumped to $$NEW_VERSION"

# Sync version without bumping
version-sync:
	@echo "🔄 Syncing version to frontend..."
	@cargo build --quiet
	@cd frontend && npm install --package-lock-only --silent
	@echo "✅ Version synchronized"

# Create git tag for current version (reads version at runtime)
version-tag:
	@VERSION=$$(grep -E '^version\s*=' Cargo.toml | head -1 | cut -d'"' -f2) && \
	echo "🏷️  Creating git tag for v$$VERSION..." && \
	git add Cargo.toml frontend/package.json frontend/package-lock.json && \
	git commit -m "Bump version to v$$VERSION" && \
	git tag -a "v$$VERSION" -m "Release v$$VERSION" && \
	echo "✅ Tag v$$VERSION created. Run 'git push origin v$$VERSION' to push."

# Full release process (pattern rule for patch/minor/major)
release-%: version-bump-% version-tag
	#@$(MAKE) version-tag
	@NEW_VERSION=$$(grep -E '^version\s*=' Cargo.toml | head -1 | cut -d'"' -f2) && \
	echo "🚀 Release v$$NEW_VERSION ready!" && \
	echo "📝 Run 'git push origin main && git push origin --tags' to complete release."

# Show current version
version:
	@echo "Current version: $(CURRENT_VERSION)"

# Help
help:
	@echo "📦 Version Management Targets:"
	@echo "  make release-patch   - Bump patch version (0.6.3 → 0.6.4) and create git tag"
	@echo "  make release-minor   - Bump minor version (0.6.3 → 0.7.0) and create git tag"
	@echo "  make release-major   - Bump major version (0.6.3 → 1.0.0) and create git tag"
	@echo "  make version         - Show current version"
	@echo ""
	@echo "🔧 Low-level targets:"
	@echo "  make version-sync    - Sync version from Cargo.toml to frontend"
	@echo "  make version-tag     - Create git tag for current version"
