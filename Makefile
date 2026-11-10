SHELL := /bin/bash
.DEFAULT_GOAL := help
PREK := uvx prek@0.5.9 # same pin as CI
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

# Coverage: a signal, never a gate. target/llvm-cov/ is local only (under
# /target/, never committed) and instrumented runs take minutes, so it stays
# out of hooks. CI's test job runs this and uploads the LCOV to Codecov.
CARGO_LLVM_COV_VERSION := 0.9.1
COVERAGE_DIR := target/llvm-cov
COVERAGE_IGNORE := --ignore-filename-regex '/tests/'

.PHONY: coverage
coverage: ## Run the tests instrumented: summary, HTML and LCOV in target/llvm-cov (local only)
	@cargo llvm-cov --version 2>/dev/null | grep -qx "cargo-llvm-cov $(CARGO_LLVM_COV_VERSION)" || \
		cargo install cargo-llvm-cov --version $(CARGO_LLVM_COV_VERSION) --locked
	@cargo llvm-cov clean --workspace
	@cargo llvm-cov --no-report
	@cargo llvm-cov report --html --output-dir $(COVERAGE_DIR) $(COVERAGE_IGNORE)
	@cargo llvm-cov report --lcov --output-path $(COVERAGE_DIR)/lcov.info $(COVERAGE_IGNORE)
	@cargo llvm-cov report --summary-only $(COVERAGE_IGNORE)
	@echo "HTML report: $(COVERAGE_DIR)/html/index.html"

##@ Linting

.PHONY: necro
necro: ## Check every file against necro.yaml with the current build: house rules, rustfmt, clippy's warnings and the rust packs (CI)
	@cargo run -q --

.PHONY: necro-fix
necro-fix: ## Fix what can be fixed (rustfmt among them) with the current build; reports what's left
	@cargo run -q -- --fix

.PHONY: check
check: ## Every hook in .pre-commit-config.yaml on every file, as CI's precommit job runs them
	$(PREK) run --all-files --show-diff-on-failure

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
