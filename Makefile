LINDERA_DICTIONARY_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-dictionary") | .version')
LINDERA_CC_CEDICT_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-cc-cedict") | .version')
LINDERA_IPADIC_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-ipadic") | .version')
LINDERA_IPADIC_NEOLOGD_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-ipadic-neologd") | .version')
LINDERA_KO_DIC_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-ko-dic") | .version')
LINDERA_UNIDIC_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-unidic") | .version')
LINDERA_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera") | .version')
LINDERA_CLI_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-cli") | .version')

USER_AGENT ?= $(shell curl --version | head -n1 | awk '{print $1"/"$2}')
USER ?= $(shell whoami)
HOSTNAME ?= $(shell hostname)

.DEFAULT_GOAL := help

clean: ## Clean the project
	cargo clean

format: ## Format the project
	cargo fmt

lint: ## Lint the project
	cargo clippy --all-targets --all-features -- -D warnings

test: ## Test the project
	cargo test --all-targets --all-features

build: ## Build the project
	cargo build --release --all-features

bench: ## Run all benchmarks
	@echo "🚀 Running all Lindera benchmarks..."
	@echo ""
	@echo "📊 Running IPADIC benchmark..."
	(cd lindera && cargo bench --bench bench_ipadic --features ipadic,embedded-ipadic) || true
	@echo ""
	@echo "📊 Running IPADIC-NEologd benchmark..."
	(cd lindera && cargo bench --bench bench_ipadic_neologd --features ipadic-neologd,embedded-ipadic-neologd) || true
	@echo ""
	@echo "📊 Running UniDic benchmark..."
	(cd lindera && cargo bench --bench bench_unidic --features unidic,embedded-unidic) || true
	@echo ""
	@echo "📊 Running KO-DIC benchmark..."
	(cd lindera && cargo bench --bench bench_ko_dic --features ko-dic,embedded-ko-dic) || true
	@echo ""
	@echo "📊 Running CC-CEDICT benchmark..."
	(cd lindera && cargo bench --bench bench_cc_cedict --features cc-cedict,embedded-cc-cedict) || true
	@echo ""
	@echo ""
	@echo "✅ All benchmarks completed!"
	@echo "📈 Results are available in lindera/target/criterion/"

bench-all: ## Run all benchmarks with all features enabled
	@echo "🚀 Running all Lindera benchmarks with all features..."
	(cd lindera && cargo bench --all-features)
	@echo "✅ All benchmarks completed!"
	@echo "📈 Results are available in lindera/target/criterion/"

tag: ## Make a tag
	git tag v$(LINDERA_VERSION)
	git push origin v$(LINDERA_VERSION)

publish: ## Publish package to crates.io
ifeq ($(shell curl -s -XGET -H "User-Agent: $(USER_AGENT) ($(USER)@$(HOSTNAME))" https://crates.io/api/v1/crates/lindera-dictionary | jq -r '.versions[].num' | grep $(LINDERA_DICTIONARY_VERSION)),)
	(cd lindera-dictionary && cargo package && cargo publish)
	sleep 10
endif
ifeq ($(shell curl -s -XGET -H "User-Agent: $(USER_AGENT) ($(USER)@$(HOSTNAME))" https://crates.io/api/v1/crates/lindera-cc-cedict | jq -r '.versions[].num' | grep $(LINDERA_CC_CEDICT_VERSION)),)
	(cd lindera-cc-cedict && cargo package && cargo publish)
	sleep 10
endif
ifeq ($(shell curl -s -XGET -H "User-Agent: $(USER_AGENT) ($(USER)@$(HOSTNAME))" https://crates.io/api/v1/crates/lindera-ipadic | jq -r '.versions[].num' | grep $(LINDERA_IPADIC_VERSION)),)
	(cd lindera-ipadic && cargo package && cargo publish)
	sleep 10
endif
ifeq ($(shell curl -s -XGET -H "User-Agent: $(USER_AGENT) ($(USER)@$(HOSTNAME))" https://crates.io/api/v1/crates/lindera-ipadic-neologd | jq -r '.versions[].num' | grep $(LINDERA_IPADIC_NEOLOGD_VERSION)),)
	(cd lindera-ipadic-neologd && cargo package && cargo publish)
	sleep 10
endif
ifeq ($(shell curl -s -XGET -H "User-Agent: $(USER_AGENT) ($(USER)@$(HOSTNAME))" https://crates.io/api/v1/crates/lindera-ko-dic | jq -r '.versions[].num' | grep $(LINDERA_KO_DIC_VERSION)),)
	(cd lindera-ko-dic && cargo package && cargo publish)
	sleep 10
endif
ifeq ($(shell curl -s -XGET -H "User-Agent: $(USER_AGENT) ($(USER)@$(HOSTNAME))" https://crates.io/api/v1/crates/lindera-unidic | jq -r '.versions[].num' | grep $(LINDERA_UNIDIC_VERSION)),)
	(cd lindera-unidic && cargo package && cargo publish)
	sleep 10
endif
ifeq ($(shell curl -s -XGET -H "User-Agent: $(USER_AGENT) ($(USER)@$(HOSTNAME))" https://crates.io/api/v1/crates/lindera | jq -r '.versions[].num' | grep $(LINDERA_VERSION)),)
	(cd lindera && cargo package && cargo publish)
	sleep 10
endif
ifeq ($(shell curl -s -XGET -H "User-Agent: $(USER_AGENT) ($(USER)@$(HOSTNAME))" https://crates.io/api/v1/crates/lindera-cli | jq -r '.versions[].num' | grep $(LINDERA_CLI_VERSION)),)
	(cd lindera-cli && cargo package && cargo publish)
endif

help: ## Show help
	@echo "Available targets:"
	@grep -E '^[a-zA-Z0-9_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  %-15s %s\n", $$1, $$2}'
