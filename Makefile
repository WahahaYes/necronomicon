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

##@ Testing

.PHONY: test
test: ## Run the tests; quiet, failures in full
	@out=$$(cargo test --quiet 2>&1) || { echo "$$out"; exit 1; }

.PHONY: test-verbose
test-verbose: ## Run the tests with full output (CI)
	cargo test

.PHONY: lint
lint: ## Run every pre-commit hook over all files
	$(PRE_COMMIT) run --all-files

##@ Help

.PHONY: help
help: ## Show this help message
	@awk 'BEGIN {FS = ":.*?## "}; /^##@ / {printf "\n\033[1m%s\033[0m\n", substr($$0, 5)}; /^[a-zA-Z_-]+:.*?## / {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)
