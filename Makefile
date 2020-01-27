BIN_DIR ?= $(CURDIR)/bin
VERSION ?=

ifeq ($(VERSION),)
  VERSION = $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera") | .version')
endif

clean:
	cargo clean

format:
	cargo fmt

build:
	cargo build --release

test:
	cargo test

.PHONY: docs
docs:
	cargo doc --no-deps
	rm -rf ./docs
	cp -pr ./target/doc ./docs
