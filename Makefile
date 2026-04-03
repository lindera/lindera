LINDERA_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera") | .version')

USER_AGENT ?= $(shell curl --version | head -n1 | awk '{print $1"/"$2}')
USER ?= $(shell whoami)
HOSTNAME ?= $(shell hostname)

# ── Python venv ─────────────────────────────────────────────────────────────
PYTHON_VENV_DIR := lindera-python/.venv
PYTHON          := $(PYTHON_VENV_DIR)/bin/python
PIP             := $(PYTHON_VENV_DIR)/bin/pip
MATURIN         := $(PYTHON_VENV_DIR)/bin/maturin
PYTEST          := $(PYTHON_VENV_DIR)/bin/pytest

# ── WASM ────────────────────────────────────────────────────────────────────
WASM_FEATURES = embed-ipadic

.DEFAULT_GOAL := help

help: ## Show help
	@echo "Available targets:"
	@grep -E '^[a-zA-Z0-9_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  %-30s %s\n", $$1, $$2}'

# ── Python venv setup ───────────────────────────────────────────────────────

$(PYTHON_VENV_DIR):
	python3 -m venv $(PYTHON_VENV_DIR)
	$(PIP) install --quiet --upgrade pip

venv: $(PYTHON_VENV_DIR) ## Create lindera-python venv and install dev dependencies
	$(PIP) install --quiet maturin pytest isort black flake8 mypy

venv-clean: ## Remove the lindera-python venv
	rm -rf $(PYTHON_VENV_DIR)

# ── Clean ───────────────────────────────────────────────────────────────────

clean: venv-clean clean-lindera-nodejs clean-lindera-wasm ## Clean all build artifacts
	cargo clean

clean-lindera-nodejs: ## Clean lindera-nodejs build artifacts
	rm -rf lindera-nodejs/node_modules
	rm -rf lindera-nodejs/npm
	rm -f lindera-nodejs/*.node
	rm -f lindera-nodejs/index.js
	rm -f lindera-nodejs/index.d.ts
	rm -f lindera-nodejs/package-lock.json

clean-lindera-wasm: ## Clean lindera-wasm build artifacts
	rm -rf lindera-wasm/pkg
	rm -rf lindera-wasm/example/dist
	rm -rf lindera-wasm/example/node_modules
	rm -f lindera-wasm/example/package-lock.json
	rm -f lindera-wasm/example/temp.json

# ── Format ──────────────────────────────────────────────────────────────────

format: ## Format all crates
	cargo fmt

format-lindera-crf: ## Format lindera-crf
	cargo fmt -p lindera-crf

format-lindera-dictionary: ## Format lindera-dictionary
	cargo fmt -p lindera-dictionary

format-lindera-ipadic: ## Format lindera-ipadic
	cargo fmt -p lindera-ipadic

format-lindera-ipadic-neologd: ## Format lindera-ipadic-neologd
	cargo fmt -p lindera-ipadic-neologd

format-lindera-unidic: ## Format lindera-unidic
	cargo fmt -p lindera-unidic

format-lindera-ko-dic: ## Format lindera-ko-dic
	cargo fmt -p lindera-ko-dic

format-lindera-cc-cedict: ## Format lindera-cc-cedict
	cargo fmt -p lindera-cc-cedict

format-lindera-jieba: ## Format lindera-jieba
	cargo fmt -p lindera-jieba

format-lindera: ## Format lindera
	cargo fmt -p lindera

format-lindera-cli: ## Format lindera-cli
	cargo fmt -p lindera-cli

format-lindera-python: ## Format lindera-python
	cargo fmt -p lindera-python

format-lindera-nodejs: ## Format lindera-nodejs
	cargo fmt -p lindera-nodejs

format-lindera-wasm: ## Format lindera-wasm
	cargo fmt -p lindera-wasm

# ── Lint ────────────────────────────────────────────────────────────────────

lint: ## Lint all crates
	cargo clippy --workspace --all-targets -- -D warnings

lint-lindera-crf: ## Lint lindera-crf
	cargo clippy -p lindera-crf --all-targets -- -D warnings

lint-lindera-dictionary: ## Lint lindera-dictionary
	cargo clippy -p lindera-dictionary --all-targets -- -D warnings

lint-lindera-ipadic: ## Lint lindera-ipadic
	cargo clippy -p lindera-ipadic --all-targets -- -D warnings

lint-lindera-ipadic-neologd: ## Lint lindera-ipadic-neologd
	cargo clippy -p lindera-ipadic-neologd --all-targets -- -D warnings

lint-lindera-unidic: ## Lint lindera-unidic
	cargo clippy -p lindera-unidic --all-targets -- -D warnings

lint-lindera-ko-dic: ## Lint lindera-ko-dic
	cargo clippy -p lindera-ko-dic --all-targets -- -D warnings

lint-lindera-cc-cedict: ## Lint lindera-cc-cedict
	cargo clippy -p lindera-cc-cedict --all-targets -- -D warnings

lint-lindera-jieba: ## Lint lindera-jieba
	cargo clippy -p lindera-jieba --all-targets -- -D warnings

lint-lindera: ## Lint lindera
	cargo clippy -p lindera --all-targets -- -D warnings

lint-lindera-cli: ## Lint lindera-cli
	cargo clippy -p lindera-cli --all-targets -- -D warnings

lint-lindera-python: ## Lint lindera-python
	cargo clippy -p lindera-python -- -D warnings

lint-lindera-nodejs: ## Lint lindera-nodejs
	cargo clippy -p lindera-nodejs -- -D warnings

lint-lindera-wasm: ## Lint lindera-wasm (wasm32 target)
	cargo clippy -p lindera-wasm --target wasm32-unknown-unknown -- -D warnings

# ── Test ────────────────────────────────────────────────────────────────────

test: ## Test all crates
	cargo test --workspace

test-lindera-crf: ## Test lindera-crf
	cargo test -p lindera-crf

test-lindera-dictionary: ## Test lindera-dictionary
	cargo test -p lindera-dictionary

test-lindera-ipadic: ## Test lindera-ipadic
	cargo test -p lindera-ipadic

test-lindera-ipadic-neologd: ## Test lindera-ipadic-neologd
	cargo test -p lindera-ipadic-neologd

test-lindera-unidic: ## Test lindera-unidic
	cargo test -p lindera-unidic

test-lindera-ko-dic: ## Test lindera-ko-dic
	cargo test -p lindera-ko-dic

test-lindera-cc-cedict: ## Test lindera-cc-cedict
	cargo test -p lindera-cc-cedict

test-lindera-jieba: ## Test lindera-jieba
	cargo test -p lindera-jieba

test-lindera: ## Test lindera
	cargo test -p lindera

test-lindera-cli: ## Test lindera-cli
	cargo test -p lindera-cli

test-lindera-python: venv ## Test lindera-python (Rust unit tests + Python pytest)
	cargo test -p lindera-python
	cd lindera-python && VIRTUAL_ENV=$(abspath $(PYTHON_VENV_DIR)) $(abspath $(MATURIN)) develop --quiet --features=embed-ipadic,train && $(abspath $(PYTEST)) tests/ -v

test-lindera-nodejs: ## Test lindera-nodejs (Rust unit tests + Node.js test)
	cd lindera-nodejs && npm install --quiet && npx napi build --platform -p lindera-nodejs && npm test

test-lindera-wasm: ## Build-test lindera-wasm (wasm32 target)
	cd lindera-wasm && wasm-pack test --node --features=$(WASM_FEATURES)

# ── Build ───────────────────────────────────────────────────────────────────

build: ## Build all crates (release)
	cargo build --release --all-features

build-lindera-crf: ## Build lindera-crf (release)
	cargo build -p lindera-crf --release

build-lindera-dictionary: ## Build lindera-dictionary (release)
	cargo build -p lindera-dictionary --release

build-lindera: ## Build lindera (release)
	cargo build -p lindera --release

build-lindera-cli: ## Build lindera-cli (release)
	cargo build -p lindera-cli --release

build-lindera-python: venv ## Build lindera-python wheel (release)
	cd lindera-python && VIRTUAL_ENV=$(abspath $(PYTHON_VENV_DIR)) $(abspath $(MATURIN)) build --release --features=embed-ipadic,train

build-lindera-nodejs: ## Build lindera-nodejs (release)
	cd lindera-nodejs && npm install --quiet && npx napi build --platform --release -p lindera-nodejs

build-lindera-wasm: ## Build lindera-wasm (wasm-pack, --target web)
	cd lindera-wasm && wasm-pack build --release --features=$(WASM_FEATURES) --target=web

# ── Benchmark ───────────────────────────────────────────────────────────────

bench: ## Run all benchmarks
	@echo "Running all Lindera benchmarks..."
	@$(foreach dict,ipadic ipadic_neologd unidic ko_dic cc_cedict, \
		echo "Running $(dict) benchmark..."; \
		(cd lindera && cargo bench --bench=bench_$(dict) --features=embed-$(subst _,-,$(dict))) || true; \
		echo "";)
	@echo "All benchmarks completed!"
	@echo "Results are available in lindera/target/criterion/"

bench-all: ## Run all benchmarks with all features enabled
	@echo "Running all Lindera benchmarks with all features..."
	(cd lindera && cargo bench --all-features)
	@echo "All benchmarks completed!"

# ── WASM example ────────────────────────────────────────────────────────────

build-lindera-wasm-example: ## Build the WASM example application
	cd lindera-wasm && wasm-pack build --release --features=embed-ipadic --target=web
	cd lindera-wasm/example && \
	LINDERA_WASM_VERSION=$$(cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-wasm") | .version') && \
	jq ".version = \"$$LINDERA_WASM_VERSION\"" ./package.json > ./temp.json && mv ./temp.json ./package.json && \
	npm install && \
	npm run build && \
	cp index.html dist/index.html

run-lindera-wasm-example: ## Run the WASM example application
	cd lindera-wasm/example && npm run start

# ── Tag & Publish ───────────────────────────────────────────────────────────

tag: ## Make a tag for the current version
	git tag v$(LINDERA_VERSION)
	git push origin v$(LINDERA_VERSION)

define PUBLISH_CRATE
	@if [ -z "$$(curl -s -XGET -H "User-Agent: $(USER_AGENT) ($(USER)@$(HOSTNAME))" https://crates.io/api/v1/crates/$(1) | jq -r '.versions[].num | select(. == "$(2)")')" ]; then \
		echo "Publishing $(1) v$(2)..."; \
		(cd $(1) && cargo package && cargo publish); \
		sleep 10; \
	else \
		echo "$(1) v$(2) is already published."; \
	fi
endef

publish: ## Publish packages to crates.io
	$(call PUBLISH_CRATE,lindera-crf,$(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-crf") | .version'))
	$(call PUBLISH_CRATE,lindera-dictionary,$(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-dictionary") | .version'))
	$(call PUBLISH_CRATE,lindera-cc-cedict,$(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-cc-cedict") | .version'))
	$(call PUBLISH_CRATE,lindera-ipadic,$(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-ipadic") | .version'))
	$(call PUBLISH_CRATE,lindera-ipadic-neologd,$(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-ipadic-neologd") | .version'))
	$(call PUBLISH_CRATE,lindera-ko-dic,$(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-ko-dic") | .version'))
	$(call PUBLISH_CRATE,lindera-unidic,$(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-unidic") | .version'))
	$(call PUBLISH_CRATE,lindera,$(LINDERA_VERSION))
	$(call PUBLISH_CRATE,lindera-python,$(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-python") | .version'))
	$(call PUBLISH_CRATE,lindera-nodejs,$(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-nodejs") | .version'))
	$(call PUBLISH_CRATE,lindera-cli,$(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-cli") | .version'))
	$(call PUBLISH_CRATE,lindera-wasm,$(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-wasm") | .version'))
