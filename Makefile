LINDERA_CORE_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-core") | .version')
LINDERA_DECOMPRESS_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-decompress") | .version')
LINDERA_COMPRESS_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-compress") | .version')
LINDERA_IPADIC_BUILDER_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-ipadic-builder") | .version')
LINDERA_IPADIC_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-ipadic") | .version')
LINDERA_UNIDIC_BUILDER_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-unidic-builder") | .version')
LINDERA_UNIDIC_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-unidic") | .version')
LINDERA_KO_DIC_BUILDER_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-ko-dic-builder") | .version')
LINDERA_KO_DIC_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-ko-dic") | .version')
LINDERA_CC_CEDICT_BUILDER_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-cc-cedict-builder") | .version')
LINDERA_CC_CEDICT_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-cc-cedict") | .version')
LINDERA_DICTIONARY_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-dictionary") | .version')
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
ifeq ($(shell curl -s -XGET https://crates.io/api/v1/crates/lindera-decompress | jq -r '.versions[].num' | grep $(LINDERA_DECOMPRESS_VERSION)),)
	(cd lindera-decompress && cargo package && cargo publish)
	sleep 10
endif
ifeq ($(shell curl -s -XGET https://crates.io/api/v1/crates/lindera-compress | jq -r '.versions[].num' | grep $(LINDERA_COMPRESS_VERSION)),)
	(cd lindera-compress && cargo package && cargo publish)
	sleep 10
endif
ifeq ($(shell curl -s -XGET https://crates.io/api/v1/crates/lindera-ipadic-builder | jq -r '.versions[].num' | grep $(LINDERA_IPADIC_BUILDER_VERSION)),)
	(cd lindera-ipadic-builder && cargo package && cargo publish)
	sleep 10
endif
ifeq ($(shell curl -s -XGET https://crates.io/api/v1/crates/lindera-ipadic | jq -r '.versions[].num' | grep $(LINDERA_IPADIC_VERSION)),)
	(cd lindera-ipadic && cargo package && cargo publish)
	sleep 10
endif
ifeq ($(shell curl -s -XGET https://crates.io/api/v1/crates/lindera-unidic-builder | jq -r '.versions[].num' | grep $(LINDERA_UNIDIC_BUILDER_VERSION)),)
	(cd lindera-unidic-builder && cargo package && cargo publish)
	sleep 10
endif
ifeq ($(shell curl -s -XGET https://crates.io/api/v1/crates/lindera-unidic | jq -r '.versions[].num' | grep $(LINDERA_UNIDIC_VERSION)),)
	(cd lindera-unidic && cargo package && cargo publish)
	sleep 10
endif
ifeq ($(shell curl -s -XGET https://crates.io/api/v1/crates/lindera-ko-dic-builder | jq -r '.versions[].num' | grep $(LINDERA_KO_DIC_BUILDER_VERSION)),)
	(cd lindera-ko-dic-builder && cargo package && cargo publish)
	sleep 10
endif
ifeq ($(shell curl -s -XGET https://crates.io/api/v1/crates/lindera-ko-dic | jq -r '.versions[].num' | grep $(LINDERA_KO_DIC_VERSION)),)
	(cd lindera-ko-dic && cargo package && cargo publish)
	sleep 10
endif
ifeq ($(shell curl -s -XGET https://crates.io/api/v1/crates/lindera-cc-cedict-builder | jq -r '.versions[].num' | grep $(LINDERA_CC_CEDICT_BUILDER_VERSION)),)
	(cd lindera-cc-cedict-builder && cargo package && cargo publish)
	sleep 10
endif
ifeq ($(shell curl -s -XGET https://crates.io/api/v1/crates/lindera-cc-cedict | jq -r '.versions[].num' | grep $(LINDERA_CC_CEDICT_VERSION)),)
	(cd lindera-cc-cedict && cargo package && cargo publish)
	sleep 10
endif
ifeq ($(shell curl -s -XGET https://crates.io/api/v1/crates/lindera-dictionary | jq -r '.versions[].num' | grep $(LINDERA_DICTIONARY_VERSION)),)
	(cd lindera-dictionary && cargo package && cargo publish)
	sleep 10
endif
ifeq ($(shell curl -s -XGET https://crates.io/api/v1/crates/lindera | jq -r '.versions[].num' | grep $(LINDERA_VERSION)),)
	(cd lindera && cargo package && cargo publish)
	sleep 10
endif
ifeq ($(shell curl -s -XGET https://crates.io/api/v1/crates/lindera-cli | jq -r '.versions[].num' | grep $(LINDERA_CLI_VERSION)),)
	(cd lindera-cli && cargo package && cargo publish)
endif

docker-build:
ifeq ($(shell curl -s 'https://registry.hub.docker.com/v2/repositories/linderamorphology/lindera/tags' | jq -r '."results"[]["name"]' | grep $(LINDERA_VERSION)),)
	docker build --tag=linderamorphology/lindera:latest --build-arg="LINDERA_VERSION=$(LINDERA_VERSION)" .
	docker tag linderamorphology/lindera:latest linderamorphology/lindera:$(LINDERA_VERSION)
endif

docker-push:
ifeq ($(shell curl -s 'https://registry.hub.docker.com/v2/repositories/linderamorphology/lindera/tags' | jq -r '."results"[]["name"]' | grep $(LINDERA_VERSION)),)
	docker push linderamorphology/lindera:latest
	docker push linderamorphology/lindera:$(LINDERA_VERSION)
endif

docker-clean:
ifneq ($(shell docker ps -f 'status=exited' -q),)
	docker rm $(shell docker ps -f 'status=exited' -q)
endif
ifneq ($(shell docker images -f 'dangling=true' -q),)
	docker rmi -f $(shell docker images -f 'dangling=true' -q)
endif
ifneq ($(docker volume ls -f 'dangling=true' -q),)
	docker volume rm $(docker volume ls -f 'dangling=true' -q)
endif
