BIN_DIR ?= $(CURDIR)/bin
VERSION ?=

ifeq ($(VERSION),)
  VERSION = $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera") | .version')
endif

clean:
	rm -rf $(BIN_DIR)
	cargo clean

format:
	cargo fmt

build:
	mkdir -p $(BIN_DIR)
	cargo build --release
	cp -p ./target/release/lindera $(BIN_DIR)

test:
	cargo test

.PHONY: docs
docs:
	cargo doc --no-deps
	rm -rf ./docs
	cp -pr ./target/doc ./docs
