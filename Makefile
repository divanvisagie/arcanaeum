APP_NAME := arcanaeum
APP_ID := com.github.divanvisagie.Arcanaeum
VERSION := $(shell awk -F\" '/^version = / { print $$2; exit }' Cargo.toml)

PREFIX ?= $(HOME)/.local
BINDIR := $(PREFIX)/bin
APPSDIR := $(PREFIX)/share/applications
ICONDIR := $(PREFIX)/share/icons/hicolor

# A throwaway config and data dir, so screenshots and test runs never touch
# the settings of a copy the user is running.
SCRATCH_ENV := XDG_CONFIG_HOME=$(CURDIR)/target/scratch/config XDG_DATA_HOME=$(CURDIR)/target/scratch/data

# Whose newest save the screenshots show: `make screenshot CHARACTER=Name`.
CHARACTER ?= Divan

.DEFAULT_GOAL := help

.PHONY: help run dev build test check screenshot icons install uninstall clean

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

# Not -D warnings yet: the unused features/ code predates Patina and
# still warns.
check: test ## Run the tests and clippy, as before a commit or release
	cargo clippy --all-targets

screenshot: ## Regenerate docs/screenshot-*.png of Divan's newest save (CHARACTER=Name for another)
	@rm -rf target/scratch
	@mkdir -p docs target/scratch/config target/scratch/data
	@test ! -f $(HOME)/.config/user-dirs.dirs || cp $(HOME)/.config/user-dirs.dirs target/scratch/config/
	$(SCRATCH_ENV) ARCANAEUM_CHARACTER="$(CHARACTER)" ARCANAEUM_SCREENSHOT=docs/screenshot-default.png cargo run
	$(SCRATCH_ENV) ARCANAEUM_CHARACTER="$(CHARACTER)" ARCANAEUM_SCREENSHOT=docs/screenshot-dark-orange.png ARCANAEUM_SKIN="Dark Orange" cargo run

icons: ## Render assets/arcanaeum-icon-128.png from packaging/linux/arcanaeum.svg
	cargo run -q --example render_icon -- 128 assets/arcanaeum-icon-128.png

# For the current user; no sudo. The launcher's Exec gets the absolute path,
# so it works even if $(BINDIR) isn't on the GNOME session's PATH. On
# Wayland the window icon comes from this launcher, matched by app id.
install: build ## Install Arcanaeum and its launcher for the current user (PREFIX=~/.local)
	install -Dm755 target/release/$(APP_NAME) $(BINDIR)/$(APP_NAME)
	install -Dm644 packaging/linux/$(APP_NAME).svg $(ICONDIR)/scalable/apps/$(APP_ID).svg
	install -Dm644 assets/$(APP_NAME)-icon-128.png $(ICONDIR)/128x128/apps/$(APP_ID).png
	install -d $(APPSDIR)
	sed 's|^Exec=$(APP_NAME)|Exec=$(BINDIR)/$(APP_NAME)|' packaging/linux/$(APP_ID).desktop > $(APPSDIR)/$(APP_ID).desktop
	-update-desktop-database -q $(APPSDIR)
	@# GTK trusts an icon cache over the folder, so refresh one if it's there.
	-@test ! -f $(ICONDIR)/icon-theme.cache || gtk-update-icon-cache -q -t -f $(ICONDIR)
	@echo "Installed $(BINDIR)/$(APP_NAME) and the $(APP_ID) launcher"

uninstall: ## Remove what make install put in place
	rm -f $(BINDIR)/$(APP_NAME) $(APPSDIR)/$(APP_ID).desktop
	rm -f $(ICONDIR)/scalable/apps/$(APP_ID).svg $(ICONDIR)/128x128/apps/$(APP_ID).png
	-update-desktop-database -q $(APPSDIR)
	-@test ! -f $(ICONDIR)/icon-theme.cache || gtk-update-icon-cache -q -t -f $(ICONDIR)

clean: ## Remove build output
	cargo clean
