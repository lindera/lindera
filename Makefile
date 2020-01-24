BIN_DIR ?= $(CURDIR)/bin
VERSION ?=

ifeq ($(VERSION),)
  VERSION = $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="mokuzu") | .version')
endif

clean:
	cargo clean

format:
	cargo fmt

build:
	cargo build --release

test:
	cargo test

doc:
	cargo doc --no-deps
	rm -rf ./docs
	cp -pr ./target/doc ./docs

