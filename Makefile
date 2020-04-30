LINDERA_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera") | .version')

.DEFAULT_GOAL := build

clean:
	cargo clean

format:
	cargo fmt

build:
	cargo build --release

test:
	cargo test

tag:
	git tag v$(LINDERA_VERSION)
	git push origin v$(LINDERA_VERSION)

publish:
ifeq ($(shell cargo show --json lindera | jq -r '.versions[].num' | grep $(LINDERA_VERSION)),)
	cargo package && cargo publish
endif
