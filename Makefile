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

setup-venv: $(PYTHON_VENV_DIR) ## Create lindera-python venv and install dev dependencies
	$(PIP) install --quiet maturin pytest isort black flake8 mypy

clean-venv: ## Remove the lindera-python venv
	rm -rf $(PYTHON_VENV_DIR)

# ── Clean ───────────────────────────────────────────────────────────────────

clean-lindera-crf: ## Clean lindera-crf
	cargo clean -p lindera-crf

clean-lindera-dictionary: ## Clean lindera-dictionary
	cargo clean -p lindera-dictionary

clean-lindera-ipadic: ## Clean lindera-ipadic
	cargo clean -p lindera-ipadic

clean-lindera-ipadic-neologd: ## Clean lindera-ipadic-neologd
	cargo clean -p lindera-ipadic-neologd

clean-lindera-unidic: ## Clean lindera-unidic
	cargo clean -p lindera-unidic

clean-lindera-ko-dic: ## Clean lindera-ko-dic
	cargo clean -p lindera-ko-dic

clean-lindera-cc-cedict: ## Clean lindera-cc-cedict
	cargo clean -p lindera-cc-cedict

clean-lindera-jieba: ## Clean lindera-jieba
	cargo clean -p lindera-jieba

clean-lindera: ## Clean lindera
	cargo clean -p lindera

clean-lindera-cli: ## Clean lindera-cli
	cargo clean -p lindera-cli

clean-lindera-python: ## Clean lindera-python build artifacts
	rm -rf lindera-python/target
	rm -rf lindera-python/dist
	rm -rf lindera-python/__pycache__
	rm -rf lindera-python/tests/__pycache__
	rm -rf lindera-python/.pytest_cache

clean-lindera-nodejs: ## Clean lindera-nodejs build artifacts
	rm -rf lindera-nodejs/node_modules
	rm -rf lindera-nodejs/npm
	rm -f lindera-nodejs/*.node
	rm -f lindera-nodejs/index.js
	rm -f lindera-nodejs/index.d.ts
	rm -f lindera-nodejs/package-lock.json

clean-lindera-ruby: ## Clean lindera-ruby build artifacts
	rm -rf lindera-ruby/tmp
	rm -f lindera-ruby/lib/lindera/lindera_ruby.so
	rm -f lindera-ruby/Gemfile.lock

clean-lindera-php: ## Clean lindera-php build artifacts
	rm -f lindera-php/target/debug/liblindera_php.so
	rm -f lindera-php/target/debug/liblindera_php.dylib

clean-lindera-wasm: ## Clean lindera-wasm build artifacts
	rm -rf lindera-wasm/pkg
	rm -rf lindera-wasm/example/dist
	rm -rf lindera-wasm/example/node_modules
	rm -f lindera-wasm/example/package-lock.json
	rm -f lindera-wasm/example/temp.json

clean: ## Clean all build artifacts
	make clean-lindera-crf
	make clean-lindera-dictionary
	make clean-lindera-ipadic
	make clean-lindera-ipadic-neologd
	make clean-lindera-unidic
	make clean-lindera-ko-dic
	make clean-lindera-cc-cedict
	make clean-lindera-jieba
	make clean-lindera
	make clean-lindera-cli
	make clean-lindera-python
	make clean-lindera-nodejs
	make clean-lindera-ruby
	make clean-lindera-php
	make clean-lindera-wasm

# ── Format ──────────────────────────────────────────────────────────────────

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
	ruff format lindera-python/tests/ lindera-python/examples/
	ruff check --fix lindera-python/tests/ lindera-python/examples/

format-lindera-nodejs: ## Format lindera-nodejs
	cargo fmt -p lindera-nodejs
	prettier --write lindera-nodejs/tests/ lindera-nodejs/examples/

format-lindera-ruby: ## Format lindera-ruby
	cargo fmt -p lindera-ruby
	rubocop -a lindera-ruby/test/ lindera-ruby/examples/ lindera-ruby/lib/

format-lindera-php: ## Format lindera-php
	cargo fmt -p lindera-php

format-lindera-wasm: ## Format lindera-wasm
	cargo fmt -p lindera-wasm
	prettier --write lindera-wasm/js/ lindera-wasm/example/

format: ## Format all crates
	make format-lindera-crf
	make format-lindera-dictionary
	make format-lindera-ipadic
	make format-lindera-ipadic-neologd
	make format-lindera-unidic
	make format-lindera-ko-dic
	make format-lindera-cc-cedict
	make format-lindera-jieba
	make format-lindera
	make format-lindera-cli
	make format-lindera-python
	make format-lindera-nodejs
	make format-lindera-ruby
	make format-lindera-php
	make format-lindera-wasm

# ── Lint ────────────────────────────────────────────────────────────────────

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
	ruff format --check lindera-python/tests/ lindera-python/examples/
	ruff check lindera-python/tests/ lindera-python/examples/

lint-lindera-nodejs: ## Lint lindera-nodejs
	cargo clippy -p lindera-nodejs -- -D warnings
	prettier --check lindera-nodejs/tests/ lindera-nodejs/examples/

lint-lindera-ruby: ## Lint lindera-ruby
	$(CARGO_TEST_WITH_RBCONFIG) cargo clippy -p lindera-ruby -- -D warnings
	rubocop lindera-ruby/test/ lindera-ruby/examples/ lindera-ruby/lib/

lint-lindera-php: ## Lint lindera-php
	cargo clippy -p lindera-php -- -D warnings

lint-lindera-wasm: ## Lint lindera-wasm (wasm32 target)
	cargo clippy -p lindera-wasm --target wasm32-unknown-unknown -- -D warnings
	prettier --check lindera-wasm/js/ lindera-wasm/example/

lint: ## Lint all crates
	make lint-lindera-crf
	make lint-lindera-dictionary
	make lint-lindera-ipadic
	make lint-lindera-ipadic-neologd
	make lint-lindera-unidic
	make lint-lindera-ko-dic
	make lint-lindera-cc-cedict
	make lint-lindera-jieba
	make lint-lindera
	make lint-lindera-cli
	make lint-lindera-python
	make lint-lindera-nodejs
	make lint-lindera-ruby
	make lint-lindera-php
	make lint-lindera-wasm

# ── Test ────────────────────────────────────────────────────────────────────

test-lindera-crf: ## Test lindera-crf
	cargo test -p lindera-crf --all-features

test-lindera-dictionary: ## Test lindera-dictionary
	cargo test -p lindera-dictionary --all-features

test-lindera-ipadic: ## Test lindera-ipadic
	cargo test -p lindera-ipadic --all-features

test-lindera-ipadic-neologd: ## Test lindera-ipadic-neologd
	cargo test -p lindera-ipadic-neologd --all-features

test-lindera-unidic: ## Test lindera-unidic
	cargo test -p lindera-unidic --all-features

test-lindera-ko-dic: ## Test lindera-ko-dic
	cargo test -p lindera-ko-dic --all-features

test-lindera-cc-cedict: ## Test lindera-cc-cedict
	cargo test -p lindera-cc-cedict --all-features

test-lindera-jieba: ## Test lindera-jieba
	cargo test -p lindera-jieba --all-features

test-lindera: ## Test lindera
	cargo test -p lindera --all-features

test-lindera-cli: ## Test lindera-cli
	cargo test -p lindera-cli --all-features

test-lindera-python: setup-venv ## Test lindera-python (Rust unit tests + Python pytest)
	cargo test -p lindera-python --lib
	cd lindera-python && VIRTUAL_ENV=$(abspath $(PYTHON_VENV_DIR)) $(abspath $(MATURIN)) develop --quiet --features=embed-ipadic,train && $(abspath $(PYTEST)) tests/ -v

test-lindera-nodejs: ## Test lindera-nodejs (Rust unit tests + Node.js test)
	cargo test -p lindera-nodejs --lib
	cd lindera-nodejs && npm install --quiet && npx napi build --platform -p lindera-nodejs && npm test

CARGO_TEST_WITH_RBCONFIG = ruby -rrbconfig -e 'RbConfig::CONFIG.each { |k, v| ENV["RBCONFIG_\#{k.upcase}"] = v }; exec(*ARGV)' --

test-lindera-ruby: ## Test lindera-ruby (Rust unit tests + minitest)
	$(CARGO_TEST_WITH_RBCONFIG) cargo test -p lindera-ruby --lib
	cd lindera-ruby && bundle install --quiet && LINDERA_FEATURES="embed-ipadic,train" bundle exec rake compile && bundle exec rake test

test-lindera-php: ## Test lindera-php (Rust unit tests + PHPUnit)
	cargo test -p lindera-php --lib
	cargo build -p lindera-php --features embed-ipadic,train
	cd lindera-php && composer install --quiet && \
		LIB=$$(find ../target/debug -maxdepth 1 -name 'liblindera_php.*' \( -name '*.so' -o -name '*.dylib' \) | head -1) && \
		php -d extension=$$LIB vendor/bin/phpunit tests/LinderaTest.php

test-lindera-wasm: ## Build-test lindera-wasm (wasm32 target)
	cargo test -p lindera-wasm --lib
	cd lindera-wasm && wasm-pack test --node --features=$(WASM_FEATURES)

test: ## Test all crates
	make test-lindera-crf
	make test-lindera-dictionary
	make test-lindera-ipadic
	make test-lindera-ipadic-neologd
	make test-lindera-unidic
	make test-lindera-ko-dic
	make test-lindera-cc-cedict
	make test-lindera-jieba
	make test-lindera
	make test-lindera-cli
	make test-lindera-python
	make test-lindera-nodejs
	make test-lindera-ruby
	make test-lindera-php
	make test-lindera-wasm

# ── Build ───────────────────────────────────────────────────────────────────

build-lindera-crf: ## Build lindera-crf (release)
	cargo build -p lindera-crf --release --all-features

build-lindera-dictionary: ## Build lindera-dictionary (release)
	cargo build -p lindera-dictionary --release --all-features

build-lindera-ipadic: ## Build lindera-ipadic (release)
	cargo build -p lindera-ipadic --release --all-features

build-lindera-ipadic-neologd: ## Build lindera-ipadic-neologd (release)
	cargo build -p lindera-ipadic-neologd --release --all-features

build-lindera-unidic: ## Build lindera-unidic (release)
	cargo build -p lindera-unidic --release --all-features

build-lindera-ko-dic: ## Build lindera-ko-dic (release)
	cargo build -p lindera-ko-dic --release --all-features

build-lindera-cc-cedict: ## Build lindera-cc-cedict (release)
	cargo build -p lindera-cc-cedict --release --all-features

build-lindera-jieba: ## Build lindera-jieba (release)
	cargo build -p lindera-jieba --release --all-features

build-lindera: ## Build lindera (release)
	cargo build -p lindera --release --all-features

build-lindera-cli: ## Build lindera-cli (release)
	cargo build -p lindera-cli --release --all-features

build-lindera-python: setup-venv ## Build lindera-python wheel (release)
	cd lindera-python && VIRTUAL_ENV=$(abspath $(PYTHON_VENV_DIR)) $(abspath $(MATURIN)) build --release --all-features

build-lindera-nodejs: ## Build lindera-nodejs (release)
	cd lindera-nodejs && npm install --quiet && npx napi build --platform --release -p lindera-nodejs

build-lindera-ruby: ## Build lindera-ruby (release)
	cd lindera-ruby && bundle install --quiet && LINDERA_FEATURES="embed-ipadic,train" bundle exec rake compile

build-lindera-php: ## Build lindera-php (release)
	cargo build -p lindera-php --release --all-features

build-lindera-wasm: ## Build lindera-wasm (wasm-pack, --target web)
	cd lindera-wasm && wasm-pack build --release --features=$(WASM_FEATURES) --target=web
	cp lindera-wasm/js/opfs.js lindera-wasm/pkg/
	cp lindera-wasm/js/opfs.d.ts lindera-wasm/pkg/

build: ## Build all crates (release)
	make build-lindera-crf
	make build-lindera-dictionary
	make build-lindera-ipadic
	make build-lindera-ipadic-neologd
	make build-lindera-unidic
	make build-lindera-ko-dic
	make build-lindera-cc-cedict
	make build-lindera-jieba
	make build-lindera
	make build-lindera-cli
	make build-lindera-python
	make build-lindera-nodejs
	make build-lindera-ruby
	make build-lindera-php
	make build-lindera-wasm

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

build-lindera-wasm-example: ## Build the WASM example application (OPFS, no embedded dictionary)
	cd lindera-wasm && wasm-pack build --release --target=web
	cp lindera-wasm/js/opfs.js lindera-wasm/pkg/
	cp lindera-wasm/js/opfs.d.ts lindera-wasm/pkg/
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
	$(call PUBLISH_CRATE,lindera-jieba,$(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-jieba") | .version'))
	$(call PUBLISH_CRATE,lindera-ko-dic,$(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-ko-dic") | .version'))
	$(call PUBLISH_CRATE,lindera-unidic,$(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-unidic") | .version'))
	$(call PUBLISH_CRATE,lindera,$(LINDERA_VERSION))
	$(call PUBLISH_CRATE,lindera-cli,$(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-cli") | .version'))
	$(call PUBLISH_CRATE,lindera-python,$(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-python") | .version'))
	$(call PUBLISH_CRATE,lindera-nodejs,$(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-nodejs") | .version'))
	$(call PUBLISH_CRATE,lindera-ruby,$(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-ruby") | .version'))
	$(call PUBLISH_CRATE,lindera-php,$(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-php") | .version'))
	$(call PUBLISH_CRATE,lindera-wasm,$(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-wasm") | .version'))
