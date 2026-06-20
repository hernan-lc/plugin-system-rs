SHELL := /bin/bash

CARGO ?= cargo
ZIGBUILD ?= cargo zigbuild
PROFILE ?= debug
CARGO_FLAGS ?= --all-targets
PROFILE_FLAG := $(if $(filter release,$(PROFILE)),--release,)

LINUX_X64_TARGET := x86_64-unknown-linux-gnu
LINUX_ARM64_TARGET := aarch64-unknown-linux-gnu
WINDOWS_X64_TARGET := x86_64-pc-windows-gnu
WINDOWS_ARM64_TARGET := aarch64-pc-windows-gnullvm
MACOS_X64_TARGET := x86_64-apple-darwin
MACOS_ARM64_TARGET := aarch64-apple-darwin

AARCH64_SYSROOT := /usr/aarch64-linux-gnu
AARCH64_PKG_CONFIG_LIBDIR := $(AARCH64_SYSROOT)/usr/lib/pkgconfig:$(AARCH64_SYSROOT)/usr/share/pkgconfig

UNAME_S := $(shell uname -s)

.PHONY: help
help:
	@printf '%s\n' \
		'Build targets:' \
		'  make build                  Build for the current host' \
		'  make build-web              Build web frontend (Preact + Vite)' \
		'  make build-plugins          Build all plugins using sd-plugins CLI' \
		'  make build-plugins-release  Build all plugins in release mode' \
		'  make build-linux-x64        Build Linux x86_64' \
		'  make build-linux-arm64      Build Linux ARM64 (requires aarch64 sysroot with X11 libs)' \
		'  make build-windows-x64      Cross-build Windows x86_64 with cargo-zigbuild' \
		'  make build-windows-arm64    Cross-build Windows ARM64 with cargo-zigbuild' \
		'  make build-macos-x64        Build macOS x86_64 on a macOS host' \
		'  make build-macos-arm64      Build macOS ARM64 on a macOS host' \
		'  make build-multiplatform    Build supported local targets for this host' \
		'  make plugins-list           List discovered plugins' \
		'  make plugins-check          Validate plugin configurations' \
		'  make plugins-clean          Clean plugin build artifacts' \
		'  make dev CMD="..."          Watch plugins + auto-rebuild + run command' \
		'  make dev-release CMD="..."  Same as dev but in release mode' \
		'' \
		'Packaging targets:' \
		'  make package VERSION=0.1.0                       Package for current host (formats from packaging.toml)' \
		'  make package-all VERSION=0.1.0                   Package for every platform (formats from packaging.toml)' \
		'  make package-platform PLATFORM=linux-x64 VERSION=0.1.0' \
		'                                                  Package for one platform' \
		'  make package-formats VERSION=0.1.0 FORMATS="deb,rpm"  Override formats' \
		'' \
		'Tray icon system dependencies:' \
		'  Arch Linux: pacman -S gtk3 xdotool libappindicator-gtk3' \
		'  Debian/Ubuntu: apt install libgtk-3-dev libxdo-dev libappindicator3-dev' \
		'  Windows: No extra deps (uses native Win32)' \
		'  macOS: No extra deps (uses native Cocoa)'

.PHONY: build
build:
	$(CARGO) build --workspace $(CARGO_FLAGS) $(PROFILE_FLAG)

.PHONY: build-web
build-web:
	cd web && npm install && npm run build

.PHONY: build-plugins
build-plugins:
	$(CARGO) build -p sd-plugins-cli
	./target/debug/sd-plugins build

.PHONY: build-plugins-release
build-plugins-release:
	$(CARGO) build --release -p sd-plugins-cli
	./target/release/sd-plugins build --release

.PHONY: plugins-list
plugins-list:
	$(CARGO) build -p sd-plugins-cli 2>/dev/null
	./target/debug/sd-plugins list

.PHONY: plugins-check
plugins-check:
	$(CARGO) build -p sd-plugins-cli 2>/dev/null
	./target/debug/sd-plugins check

.PHONY: plugins-clean
plugins-clean:
	$(CARGO) build -p sd-plugins-cli 2>/dev/null
	./target/debug/sd-plugins clean

.PHONY: dev
dev:
	$(CARGO) build -p sd-plugins-cli 2>/dev/null
	./target/debug/sd-plugins dev -- $(CMD)

.PHONY: dev-release
dev-release:
	$(CARGO) build --release -p sd-plugins-cli 2>/dev/null
	./target/release/sd-plugins dev -r -- $(CMD)

.PHONY: ensure-linux-x64-target
ensure-linux-x64-target:
	@if ! rustup target list --installed | grep -qx '$(LINUX_X64_TARGET)'; then \
		rustup target add '$(LINUX_X64_TARGET)'; \
	fi

.PHONY: ensure-linux-arm64-target
ensure-linux-arm64-target:
	@if ! rustup target list --installed | grep -qx '$(LINUX_ARM64_TARGET)'; then \
		rustup target add '$(LINUX_ARM64_TARGET)'; \
	fi

.PHONY: ensure-windows-x64-target
ensure-windows-x64-target:
	@if ! rustup target list --installed | grep -qx '$(WINDOWS_X64_TARGET)'; then \
		rustup target add '$(WINDOWS_X64_TARGET)'; \
	fi

.PHONY: ensure-windows-arm64-target
ensure-windows-arm64-target:
	@if ! rustup target list --installed | grep -qx '$(WINDOWS_ARM64_TARGET)'; then \
		rustup target add '$(WINDOWS_ARM64_TARGET)'; \
	fi

.PHONY: ensure-macos-x64-target
ensure-macos-x64-target:
	@if ! rustup target list --installed | grep -qx '$(MACOS_X64_TARGET)'; then \
		rustup target add '$(MACOS_X64_TARGET)'; \
	fi

.PHONY: ensure-macos-arm64-target
ensure-macos-arm64-target:
	@if ! rustup target list --installed | grep -qx '$(MACOS_ARM64_TARGET)'; then \
		rustup target add '$(MACOS_ARM64_TARGET)'; \
	fi

.PHONY: build-linux-x64
build-linux-x64: ensure-linux-x64-target
	$(CARGO) build --workspace --target $(LINUX_X64_TARGET) $(CARGO_FLAGS) $(PROFILE_FLAG)

.PHONY: build-linux-arm64
build-linux-arm64: ensure-linux-arm64-target
	PKG_CONFIG_ALLOW_CROSS=1 \
	PKG_CONFIG_SYSROOT_DIR=$(AARCH64_SYSROOT) \
	PKG_CONFIG_LIBDIR=$(AARCH64_PKG_CONFIG_LIBDIR) \
	$(ZIGBUILD) --target $(LINUX_ARM64_TARGET) --workspace $(CARGO_FLAGS) $(PROFILE_FLAG)

.PHONY: build-windows-x64
build-windows-x64: ensure-windows-x64-target
	PKG_CONFIG_ALLOW_CROSS=1 $(ZIGBUILD) --target $(WINDOWS_X64_TARGET) --workspace $(CARGO_FLAGS) $(PROFILE_FLAG)

.PHONY: build-windows-arm64
build-windows-arm64: ensure-windows-arm64-target
	$(ZIGBUILD) --target $(WINDOWS_ARM64_TARGET) --workspace $(CARGO_FLAGS) $(PROFILE_FLAG)

.PHONY: require-macos-host
require-macos-host:
	@if [ '$(UNAME_S)' != 'Darwin' ]; then \
		echo 'macOS targets must be built on macOS. Current host is $(UNAME_S).' >&2; \
		exit 1; \
	fi

.PHONY: build-macos-x64
build-macos-x64: require-macos-host ensure-macos-x64-target
	$(CARGO) build --workspace --target $(MACOS_X64_TARGET) $(CARGO_FLAGS) $(PROFILE_FLAG)

.PHONY: build-macos-arm64
build-macos-arm64: require-macos-host ensure-macos-arm64-target
	$(CARGO) build --workspace --target $(MACOS_ARM64_TARGET) $(CARGO_FLAGS) $(PROFILE_FLAG)

.PHONY: build-multiplatform
build-multiplatform: build-linux-x64 build-linux-arm64 build-windows-x64 build-windows-arm64
	@if [ '$(UNAME_S)' = 'Darwin' ]; then \
		$(MAKE) build-macos-x64 build-macos-arm64; \
	else \
		echo 'Skipping macOS targets on $(UNAME_S); run make build-macos-x64 build-macos-arm64 on macOS.'; \
	fi

# ---- Packaging ----------------------------------------------------------

SD_PLUGINS ?= $(CARGO_TARGET_DIR)/$(PROFILE)/sd-plugins

.PHONY: ensure-cli
ensure-cli:
	@if [ ! -x '$(SD_PLUGINS)' ]; then \
		echo 'Building sd-plugins CLI ($(PROFILE))...'; \
		$(CARGO) build $(if $(filter release,$(PROFILE)),--release,) -p sd-plugins-cli; \
	fi

# Package for the current host using packaging.toml defaults
.PHONY: package
package: ensure-cli
	@test -n '$(VERSION)' || (echo 'VERSION is required, e.g. make package VERSION=0.1.0' && exit 1)
	@CMD='$(SD_PLUGINS) pkg --version $(VERSION)'; \
	if [ -n '$(FORMATS)' ]; then CMD="$$CMD --formats $(FORMATS)"; fi; \
	if [ -n '$(PLATFORM)' ]; then CMD="$$CMD --platform $(PLATFORM)"; fi; \
	echo "Running: $$CMD"; \
	$$CMD

# Package for every platform using packaging.toml defaults
.PHONY: package-all
package-all: ensure-cli
	@test -n '$(VERSION)' || (echo 'VERSION is required, e.g. make package-all VERSION=0.1.0' && exit 1)
	$(SD_PLUGINS) pkg --all-platforms --version $(VERSION) $(if $(FORMATS),--formats $(FORMATS),)

.PHONY: package-platform
package-platform: ensure-cli
	@test -n '$(PLATFORM)' || (echo 'PLATFORM is required (linux-x64, linux-arm64, windows-x64, windows-arm64, macos-x64, macos-arm64)' && exit 1)
	@test -n '$(VERSION)' || (echo 'VERSION is required' && exit 1)
	$(SD_PLUGINS) pkg --platform $(PLATFORM) --version $(VERSION) $(if $(FORMATS),--formats $(FORMATS),)

.PHONY: package-formats
package-formats: ensure-cli
	@test -n '$(FORMATS)' || (echo 'FORMATS is required, e.g. FORMATS="deb,rpm,appimage"' && exit 1)
	@test -n '$(VERSION)' || (echo 'VERSION is required' && exit 1)
	$(SD_PLUGINS) pkg --version $(VERSION) --formats $(FORMATS) $(if $(PLATFORM),--platform $(PLATFORM),)
