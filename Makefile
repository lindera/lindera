LINDERA_IPADIC_DIR ?= ./lindera-ipadic
LINDERA_IPADIC_VERSION ?= 0.1.4

clean:
	cargo clean

clean-dict:
	rm -rf $(LINDERA_IPADIC_DIR)
	rm -rf lindera-ipadic-*.tar.bz2

format:
	cargo fmt

lindera-ipadic:
	curl -L https://github.com/bayard-search/lindera-ipadic/releases/download/v$(LINDERA_IPADIC_VERSION)/lindera-ipadic-$(LINDERA_IPADIC_VERSION).tar.bz2 > ./lindera-ipadic-$(LINDERA_IPADIC_VERSION).tar.bz2
	tar -xvjf ./lindera-ipadic-$(LINDERA_IPADIC_VERSION).tar.bz2

build:
	cargo build --release

test:
	cargo test
