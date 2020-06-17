BIN_DIR ?= ./bin

LINDERA_CORE_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-core") | .version')
LINDERA_IPADIC_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-ipadic") | .version')
LINDERA_DICTIONARY_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-dictionary") | .version')
LINDERA_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera") | .version')
LINDERA_CLI_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-cli") | .version')

.DEFAULT_GOAL := build

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

tag:
	git tag v$(LINDERA_VERSION)
	git push origin v$(LINDERA_VERSION)

publish:
ifeq ($(shell curl -s -XGET https://crates.io/api/v1/crates/lindera-core | jq -r '.versions[].num' | grep $(LINDERA_CORE_VERSION)),)
	(cd lindera-core && cargo package && cargo publish)
	sleep 10
endif
ifeq ($(shell curl -s -XGET https://crates.io/api/v1/crates/lindera-ipadic | jq -r '.versions[].num' | grep $(LINDERA_IPADIC_VERSION)),)
	(cd lindera-ipadic && cargo package && cargo publish)
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
ifeq ($(shell curl -s 'https://registry.hub.docker.com/v2/repositories/linderamorphology/lindera-cli/tags' | jq -r '."results"[]["name"]' | grep $(LINDERA_CLI_VERSION)),)
	docker build --tag=linderamorphology/lindera-cli:latest --build-arg="LINDERA_CLI_VERSION=$(LINDERA_CLI_VERSION)" .
	docker tag linderamorphology/lindera-cli:latest linderamorphology/lindera-cli:$(LINDERA_CLI_VERSION)
endif

docker-push:
ifeq ($(shell curl -s 'https://registry.hub.docker.com/v2/repositories/linderamorphology/lindera-cli/tags' | jq -r '."results"[]["name"]' | grep $(LINDERA_CLI_VERSION)),)
	docker push linderamorphology/lindera-cli:latest
	docker push linderamorphology/lindera-cli:$(LINDERA_CLI_VERSION)
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
