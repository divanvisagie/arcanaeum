APP_NAME := arcanaeum
VERSION := $(shell awk -F\" '/^version = / { print $$2; exit }' Cargo.toml)

# A throwaway config and data dir, so screenshots and test runs never touch
# the settings of a copy the user is running.
SCRATCH_ENV := XDG_CONFIG_HOME=$(CURDIR)/target/scratch/config XDG_DATA_HOME=$(CURDIR)/target/scratch/data

# Whose newest save the screenshots show: `make screenshot CHARACTER=Name`.
CHARACTER ?= Divan

.DEFAULT_GOAL := help

.PHONY: help run dev build test check screenshot clean

help: ## Show this help
	@echo "Usage: make <target>"
	@echo
	@awk 'BEGIN { FS = ":.*## " } /^[a-z-]+:.*## / { printf "  \033[1m%-14s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

run: ## Run the app
	cargo run

dev: ## Run with live reload (requires cargo-watch)
	@command -v cargo-watch >/dev/null 2>&1 || { echo "cargo-watch is required: cargo install cargo-watch"; exit 1; }
	cargo watch -x "run --bin $(APP_NAME)"

build: ## Build a release binary
	cargo build --release

test: ## Run the tests
	cargo test

# Not -D warnings yet: the save parser and the unused features/ code
# predate Patina and still warn.
check: test ## Run the tests and clippy, as before a commit or release
	cargo clippy --all-targets

screenshot: ## Regenerate docs/screenshot-*.png of Divan's newest save (CHARACTER=Name for another)
	@rm -rf target/scratch
	@mkdir -p docs target/scratch/config target/scratch/data
	@test ! -f $(HOME)/.config/user-dirs.dirs || cp $(HOME)/.config/user-dirs.dirs target/scratch/config/
	$(SCRATCH_ENV) ARCANAEUM_CHARACTER="$(CHARACTER)" ARCANAEUM_SCREENSHOT=docs/screenshot-default.png cargo run
	$(SCRATCH_ENV) ARCANAEUM_CHARACTER="$(CHARACTER)" ARCANAEUM_SCREENSHOT=docs/screenshot-dark-orange.png ARCANAEUM_SKIN="Dark Orange" cargo run

clean: ## Remove build output
	cargo clean
