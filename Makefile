LINDERA_CORE_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-core") | .version')
LINDERA_CC_CEDICT_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-cc-cedict") | .version')
LINDERA_IPADIC_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-ipadic") | .version')
LINDERA_IPADIC_NEOLOGD_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-ipadic-neologd") | .version')
LINDERA_KO_DIC_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-ko-dic") | .version')
LINDERA_UNIDIC_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-unidic") | .version')
LINDERA_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera") | .version')
LINDERA_CLI_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-cli") | .version')

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
ifeq ($(shell curl -s -XGET https://crates.io/api/v1/crates/lindera-core | jq -r '.versions[].num' | grep $(LINDERA_CORE_VERSION)),)
	(cd lindera-core && cargo package && cargo publish)
	sleep 10
endif
ifeq ($(shell curl -s -XGET https://crates.io/api/v1/crates/lindera-cc-cedict | jq -r '.versions[].num' | grep $(LINDERA_CC_CEDICT_VERSION)),)
	(cd lindera-cc-cedict && cargo package && cargo publish)
	sleep 10
endif
ifeq ($(shell curl -s -XGET https://crates.io/api/v1/crates/lindera-ipadic | jq -r '.versions[].num' | grep $(LINDERA_IPADIC_VERSION)),)
	(cd lindera-ipadic && cargo package && cargo publish)
	sleep 10
endif
ifeq ($(shell curl -s -XGET https://crates.io/api/v1/crates/lindera-ipadic-neologd | jq -r '.versions[].num' | grep $(LINDERA_IPADIC_NEOLOGD_VERSION)),)
	(cd lindera-ipadic-neologd && cargo package && cargo publish)
	sleep 10
endif
ifeq ($(shell curl -s -XGET https://crates.io/api/v1/crates/lindera-ko-dic | jq -r '.versions[].num' | grep $(LINDERA_KO_DIC_VERSION)),)
	(cd lindera-ko-dic && cargo package && cargo publish)
	sleep 10
endif
ifeq ($(shell curl -s -XGET https://crates.io/api/v1/crates/lindera-unidic | jq -r '.versions[].num' | grep $(LINDERA_UNIDIC_VERSION)),)
	(cd lindera-unidic && cargo package && cargo publish)
	sleep 10
endif
ifeq ($(shell curl -s -XGET https://crates.io/api/v1/crates/lindera | jq -r '.versions[].num' | grep $(LINDERA_VERSION)),)
	(cd lindera && cargo package && cargo publish)
	sleep 10
endif
ifeq ($(shell curl -s -XGET https://crates.io/api/v1/crates/lindera-cli | jq -r '.versions[].num' | grep $(LINDERA_CLI_VERSION)),)
	(cd lindera-cli && cargo package && cargo publish)
endif
