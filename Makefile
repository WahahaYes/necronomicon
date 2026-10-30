SHELL := /bin/bash
.DEFAULT_GOAL := help
PRE_COMMIT := uvx pre-commit@4.6.1 # same pin as CI
MATURIN := uvx maturin@1.15.0 # builds the PyPI wheel around the binary

##@ Build

.PHONY: build
build: ## Build the release binary (target/release/necro)
	cargo build --release

.PHONY: wheel
wheel: ## Build the wheel for this platform into target/wheels/
	$(MATURIN) build --release

.PHONY: package
package: ## Build and verify the crate as crates.io gets it (target/package/)
	cargo package --locked

##@ Testing

.PHONY: test
test: ## Run the tests; quiet, failures in full
	@out=$$(cargo test --quiet 2>&1) || { echo "$$out"; exit 1; }

.PHONY: test-verbose
test-verbose: ## Run the tests with full output (CI)
	cargo test

##@ Linting

.PHONY: lint
lint: ## Run clippy and this repo's house rules (src/necro.yaml, current build)
	@out=$$(cargo clippy --all-targets --quiet -- -D warnings 2>&1) || { echo "$$out"; exit 1; }
	@cargo run -q -- -C src

.PHONY: format
format: ## Format the Rust sources
	cargo fmt

.PHONY: format-check
format-check: ## Check Rust formatting without rewriting (CI)
	cargo fmt --check

.PHONY: check
check: ## Every pre-commit hook on every file, as CI's precommit job runs them
	$(PRE_COMMIT) run --all-files --show-diff-on-failure

##@ Cleanup

# A fresh `make test lint build wheel` leaves target/ at ~240 MB; much more is
# stale builds (cargo keeps one per toolchain, lockfile and flags).
.PHONY: clean
clean: ## Remove built wheels (target/wheels/)
	@$(call remove,target/wheels)

.PHONY: clean-cache
clean-cache: ## Also remove cargo's build cache (target/); the next build starts cold
	@$(call remove,target)

# Removes a path and says how much it held.
define remove
if [ -e $(1) ]; then echo "removed $(1) ($$(du -sh $(1) | cut -f1))"; rm -rf $(1); else echo "nothing to remove at $(1)"; fi
endef

##@ Help

.PHONY: help
help: ## Show this help message
	@awk 'BEGIN {FS = ":.*?## "}; /^##@ / {printf "\n\033[1m%s\033[0m\n", substr($$0, 5)}; /^[a-zA-Z_-]+:.*?## / {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)
