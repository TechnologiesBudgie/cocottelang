# Makefile for Cocotte
# Usage: make [target]
# Targets: build, release, test, run, clean, install, fmt, check

BINARY    := cocotte
CARGO     := cargo
DESTDIR   ?= /usr/local

.PHONY: all build release test run clean install uninstall fmt check help

all: build

## Build debug binary
build:
	$(CARGO) build

## Build optimised release binary
release:
	$(CARGO) build --release

## Build without GUI (smaller binary, no Charlotte)
build-nogui:
	$(CARGO) build --no-default-features

## Build release without GUI
release-nogui:
	$(CARGO) build --release --no-default-features

## Run the test suite
test: build
	./target/debug/$(BINARY) test

## Run the demo
run: build
	./target/debug/$(BINARY) run demos/native_demo.cot

## Check code without building
check:
	$(CARGO) check

## Format code
fmt:
	$(CARGO) fmt

## Format check (CI)
fmt-check:
	$(CARGO) fmt --check

## Lint with clippy
lint:
	$(CARGO) clippy -- -D warnings

## Install to DESTDIR (default: /usr/local)
install: release
	install -Dm755 target/release/$(BINARY) $(DESTDIR)/bin/$(BINARY)
	@echo "Installed to $(DESTDIR)/bin/$(BINARY)"

## Uninstall
uninstall:
	rm -f $(DESTDIR)/bin/$(BINARY)
	@echo "Removed $(DESTDIR)/bin/$(BINARY)"

## Clean build artefacts
clean:
	$(CARGO) clean
	rm -rf dist/ build/ .cocotte_cache

## Show this help
help:
	@grep -E '^## ' Makefile | sed 's/## /  /'
	@echo ""
	@echo "Usage: make [target]"
