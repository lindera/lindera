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

# ── Node.js ─────────────────────────────────────────────────────────────────
# Dictionaries embedded in the Node.js test build, so the dictionary-backed
# tests run instead of being skipped (#1066). memcheck builds with the same
# features, so it does not switch the addon to a different build.
NODEJS_TEST_FEATURES = embed-ipadic,embed-ko-dic

# Run a cargo command with RbConfig values exported, as the Ruby binding build
# expects (used by both lint and test for lindera-ruby).
CARGO_TEST_WITH_RBCONFIG = ruby -rrbconfig -e 'RbConfig::CONFIG.each { |k, v| ENV["RBCONFIG_\#{k.upcase}"] = v }; exec(*ARGV)' --

# ── Per-crate configuration ───────────────────────────────────────────────────
#
# Crates driven by plain cargo commands are handled by the pattern rules below
# (clean-%, format-%, lint-%, test-%, build-%). The language bindings need extra
# tooling, so they keep explicit targets, which override the matching pattern.
#
# `make <verb>` (clean/format/lint/test/build) runs the verb for every crate in
# dependency order.

# Crates whose verbs are plain cargo invocations.
CARGO_CRATES := lindera-crf lindera-dictionary lindera-trainer lindera-ipadic \
	lindera-ipadic-neologd lindera-unidic lindera-sudachidict lindera-ko-dic \
	lindera-cc-cedict lindera-jieba \
	lindera lindera-analysis lindera-cli lindera-binding-core

# Language bindings with bespoke tooling (explicit targets below).
BINDING_CRATES := lindera-python lindera-nodejs lindera-ruby lindera-php lindera-wasm

# All crates, in the order the aggregate targets should visit them.
ALL_CRATES := $(CARGO_CRATES) $(BINDING_CRATES)

# Per-crate cargo features, used by lint/test/build unless a per-target
# override (TEST_FEATURES_* / BUILD_FEATURES_*) is set. Crates without an entry
# are built with no extra features.
FEATURES_lindera-dictionary     := --features build_rs
FEATURES_lindera-ipadic         := --features embed-ipadic
FEATURES_lindera-ipadic-neologd := --features embed-ipadic-neologd
FEATURES_lindera-unidic         := --features embed-unidic
FEATURES_lindera-sudachidict    := --features embed-sudachidict
FEATURES_lindera-ko-dic         := --features embed-ko-dic
FEATURES_lindera-cc-cedict      := --features embed-cc-cedict
FEATURES_lindera-jieba          := --features embed-jieba
FEATURES_lindera                := --features embed-ipadic,train
FEATURES_lindera-analysis       := --features embed-ipadic,embed-ko-dic
FEATURES_lindera-cli            := --features train

# Per-target feature overrides (where lint/test/build differ).
TEST_FEATURES_lindera-cli       := --features train,embed-ipadic
BUILD_FEATURES_lindera          := --features train

.DEFAULT_GOAL := help

help: ## Show help
	@echo "Available targets:"
	@grep -E '^[a-zA-Z0-9_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  %-30s %s\n", $$1, $$2}'
	@echo ""
	@echo "Per-crate targets (cargo crates): {clean,format,lint,test,build}-<crate>"
	@echo "  crates: $(CARGO_CRATES)"

# ── Python venv setup ───────────────────────────────────────────────────────

$(PYTHON_VENV_DIR):
	python3 -m venv $(PYTHON_VENV_DIR)
	$(PIP) install --quiet --upgrade pip

setup-venv: $(PYTHON_VENV_DIR) ## Create lindera-python venv and install dev dependencies
	$(PIP) install --quiet maturin pytest isort black flake8 mypy

clean-venv: ## Remove the lindera-python venv
	rm -rf $(PYTHON_VENV_DIR)

# ── Clean ───────────────────────────────────────────────────────────────────

clean-%:
	cargo clean -p $*

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
	rm -f lindera-nodejs/package-lock.json

clean-lindera-ruby: ## Clean lindera-ruby build artifacts
	rm -rf lindera-ruby/tmp
	rm -rf lindera-ruby/pkg
	rm -f lindera-ruby/lib/lindera/lindera_ruby.so
	rm -f lindera-ruby/Gemfile.lock

# Files that phpize / configure / make leave inside lindera-php/ (the build
# path PIE uses). Listed by name because `phpize --clean` also deletes
# tests/*.php, which would remove the PHPUnit suite. Keep in sync with
# lindera-php/.gitignore.
PHPIZE_ARTIFACTS = build modules autom4te.cache .libs configure configure.ac \
	config.h config.h.in config.h.in~ config.log config.nice config.status \
	config.cache Makefile Makefile.fragments Makefile.objects libtool \
	run-tests.php tmp-php.ini

# composer.json lives at the repository root (Packagist reads it there), so
# Composer's output lands at the root too.
clean-lindera-php: ## Clean lindera-php build artifacts
	rm -rf vendor composer.lock .tmp/pie-stage
	cd lindera-php && rm -rf $(PHPIZE_ARTIFACTS) *.lo *.la .phpunit.cache .phpunit.result.cache

clean-lindera-wasm: ## Clean lindera-wasm build artifacts
	rm -rf lindera-wasm/pkg
	rm -rf lindera-wasm/example/dist
	rm -rf lindera-wasm/example/node_modules
	rm -f lindera-wasm/example/package-lock.json
	rm -f lindera-wasm/example/temp.json

clean: ## Clean all build artifacts
	$(foreach c,$(ALL_CRATES),make clean-$(c) &&) true

# ── Format ──────────────────────────────────────────────────────────────────

format-%:
	cargo fmt -p $*

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
	vendor/bin/php-cs-fixer fix --config=lindera-php/.php-cs-fixer.dist.php

format-lindera-wasm: ## Format lindera-wasm
	cargo fmt -p lindera-wasm
	prettier --write lindera-wasm/js/ lindera-wasm/example/

format: ## Format all crates
	$(foreach c,$(ALL_CRATES),make format-$(c) &&) true

# ── Lint ────────────────────────────────────────────────────────────────────

lint-%:
	cargo clippy -p $* $(FEATURES_$*) -- -D warnings

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
	$(foreach c,$(ALL_CRATES),make lint-$(c) &&) true

# ── Test ────────────────────────────────────────────────────────────────────

test-%:
	cargo test -p $* $(if $(TEST_FEATURES_$*),$(TEST_FEATURES_$*),$(FEATURES_$*))

test-lindera-python: setup-venv ## Test lindera-python (Rust unit tests + Python pytest)
	cargo test -p lindera-python --lib
	cd lindera-python && VIRTUAL_ENV=$(abspath $(PYTHON_VENV_DIR)) $(abspath $(MATURIN)) develop --quiet --features=embed-ipadic,train && $(abspath $(PYTEST)) tests/ -v

test-lindera-nodejs: ## Test lindera-nodejs (Rust unit tests + Node.js test + generated type check)
	cargo test -p lindera-nodejs --lib
	cd lindera-nodejs && npm install --quiet && npx napi build --platform -p lindera-nodejs --features $(NODEJS_TEST_FEATURES) && npm test && npm run test:types

memcheck-lindera-nodejs: ## Check lindera-nodejs releases token memory in synchronous loops
	cd lindera-nodejs && npm install --quiet && npx napi build --platform -p lindera-nodejs --features $(NODEJS_TEST_FEATURES)
	node --expose-gc scripts/benchmarks/memcheck_nodejs.mjs

memcheck-lindera-wasm: ## Check lindera-wasm releases token memory in synchronous loops
	cd lindera-wasm && wasm-pack build --target nodejs --features=$(WASM_FEATURES) --out-dir pkg-node
	node --expose-gc scripts/benchmarks/memcheck_wasm.mjs

test-lindera-ruby: ## Test lindera-ruby (Rust unit tests + minitest)
	$(CARGO_TEST_WITH_RBCONFIG) cargo test -p lindera-ruby --lib
	cd lindera-ruby && bundle install --quiet && LINDERA_FEATURES="embed-ipadic,train" bundle exec rake compile && bundle exec rake test

# Installs the source gem with `gem install` in a clean Ruby container, as a
# user would, and tokenizes with it (lindera-ruby/test/gem/). The lindera
# crates are compiled from this checkout rather than crates.io, so unreleased
# changes are tested too; release.yml runs the same script against crates.io.
RUBY_GEM_TEST_IMAGE ?= ruby:3.3

test-lindera-ruby-gem: package-lindera-ruby ## Test installing the lindera-ruby gem in a clean container (needs Docker)
	docker run --rm -v "$(CURDIR):/src:ro" -e LINDERA_SOURCE_DIR=/src $(RUBY_GEM_TEST_IMAGE) \
		bash /src/lindera-ruby/test/gem/install_and_test.sh --setup /src/lindera-ruby/pkg/lindera-$(LINDERA_VERSION).gem

# The PHPUnit suite loads embedded://ipadic, so the test build embeds IPADIC.
# Embedded dictionaries do not change the PHP API, so the same build also
# serves the stub check (StubsTest) and `make stubs-lindera-php`.
PHP_TEST_FEATURES = embed-ipadic,train
PHP_TEST_LIB = $$(find target/debug -maxdepth 1 \( -name 'liblindera_php.so' -o -name 'liblindera_php.dylib' \) | head -1)

test-lindera-php: ## Test lindera-php (Rust unit tests + PHPUnit)
	cargo test -p lindera-php --lib
	cargo build -p lindera-php --features $(PHP_TEST_FEATURES)
	composer validate --strict --no-check-publish
	composer install --quiet --no-interaction
	php -d extension=$(PHP_TEST_LIB) vendor/bin/phpunit -c lindera-php/phpunit.xml.dist

# Exercises the build path PIE uses (phpize / configure / make / make install)
# without touching the system: the module is staged under .tmp/ and loaded
# into a php that reads no ini files. The second `make -n` must not mention
# cargo, which proves `sudo make install` after `make` would not rebuild.
test-lindera-php-pie: ## Test lindera-php through the phpize build that PIE runs
	cd lindera-php && rm -rf $(PHPIZE_ARTIFACTS) && \
		phpize && \
		./configure --with-php-config="$$(command -v php-config)" && \
		make && \
		php -n -d extension="$$PWD/modules/lindera.so" -m | grep -qx lindera && \
		{ ! make -n 2>&1 | grep -q cargo; } && \
		make install INSTALL_ROOT="$$PWD/../.tmp/pie-stage" && \
		test -f "$$PWD/../.tmp/pie-stage$$(php-config --extension-dir)/lindera.so" && \
		rm -rf "$$PWD/../.tmp/pie-stage" $(PHPIZE_ARTIFACTS)

# The stub file is generated from the compiled extension so it cannot drift
# from the Rust source; StubsTest fails when it is out of date.
stubs-lindera-php: ## Regenerate lindera-php/stubs/lindera.stubs.php from the built extension
	cargo build -p lindera-php --features $(PHP_TEST_FEATURES)
	php -n -d extension=$(PHP_TEST_LIB) lindera-php/tools/generate-stubs.php > lindera-php/stubs/lindera.stubs.php

test-lindera-wasm: ## Build-test lindera-wasm (wasm32 target)
	cargo test -p lindera-wasm --lib
	cd lindera-wasm && wasm-pack test --node --features=$(WASM_FEATURES)

test: ## Test all crates
	$(foreach c,$(ALL_CRATES),make test-$(c) &&) true

# ── Build ───────────────────────────────────────────────────────────────────

build-%:
	cargo build -p $* --release $(if $(BUILD_FEATURES_$*),$(BUILD_FEATURES_$*),$(FEATURES_$*))

build-lindera-python: setup-venv ## Build lindera-python wheel (release)
	cd lindera-python && VIRTUAL_ENV=$(abspath $(PYTHON_VENV_DIR)) $(abspath $(MATURIN)) build --release --features train

build-lindera-nodejs: ## Build lindera-nodejs (release)
	cd lindera-nodejs && npm install --quiet && npx napi build --platform --release -p lindera-nodejs

build-lindera-ruby: ## Build lindera-ruby (release)
	cd lindera-ruby && bundle install --quiet && LINDERA_FEATURES="embed-ipadic,train" bundle exec rake compile

package-lindera-ruby: ## Build the lindera-ruby source gem into lindera-ruby/pkg/
	cd lindera-ruby && bundle install --quiet && bundle exec rake build

build-lindera-php: ## Build lindera-php (release)
	cargo build -p lindera-php --release --features train

build-lindera-wasm: ## Build lindera-wasm (wasm-pack, --target web)
	cd lindera-wasm && wasm-pack build --release --features=$(WASM_FEATURES) --target=web
	cp lindera-wasm/js/opfs.js lindera-wasm/pkg/
	cp lindera-wasm/js/opfs.d.ts lindera-wasm/pkg/

build: ## Build all crates (release)
	$(foreach c,$(ALL_CRATES),make build-$(c) &&) true

# ── Benchmark ───────────────────────────────────────────────────────────────

bench: ## Run all benchmarks
	@echo "Running all Lindera benchmarks..."
	@$(foreach dict,ipadic ipadic_neologd unidic sudachidict ko_dic cc_cedict, \
		echo "Running $(dict) benchmark..."; \
		(cd lindera && cargo bench --bench=bench_$(dict) --features=embed-$(subst _,-,$(dict))) || true; \
		echo "";)
	@echo "All benchmarks completed!"
	@echo "Results are available in lindera/target/criterion/"

bench-all: ## Run all benchmarks with CJK dictionaries
	@echo "Running all Lindera benchmarks with CJK dictionaries..."
	(cd lindera && cargo bench --features embed-cjk)
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

# Resolve a workspace crate's version from cargo metadata.
crate_version = $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="$(1)") | .version')

define PUBLISH_CRATE
	@if [ -z "$$(curl -s -XGET -H "User-Agent: $(USER_AGENT) ($(USER)@$(HOSTNAME))" https://crates.io/api/v1/crates/$(1) | jq -r '.versions[].num | select(. == "$(2)")')" ]; then \
		echo "Publishing $(1) v$(2)..."; \
		(cd $(1) && cargo package && cargo publish); \
		sleep 10; \
	else \
		echo "$(1) v$(2) is already published."; \
	fi
endef

# Crates published to crates.io, in dependency order. Each call is a separate
# recipe line (the multi-line PUBLISH_CRATE macro cannot be folded with foreach).
publish: ## Publish packages to crates.io
	$(call PUBLISH_CRATE,lindera-crf,$(call crate_version,lindera-crf))
	$(call PUBLISH_CRATE,lindera-dictionary,$(call crate_version,lindera-dictionary))
	$(call PUBLISH_CRATE,lindera-trainer,$(call crate_version,lindera-trainer))
	$(call PUBLISH_CRATE,lindera-cc-cedict,$(call crate_version,lindera-cc-cedict))
	$(call PUBLISH_CRATE,lindera-ipadic,$(call crate_version,lindera-ipadic))
	$(call PUBLISH_CRATE,lindera-ipadic-neologd,$(call crate_version,lindera-ipadic-neologd))
	$(call PUBLISH_CRATE,lindera-jieba,$(call crate_version,lindera-jieba))
	$(call PUBLISH_CRATE,lindera-ko-dic,$(call crate_version,lindera-ko-dic))
	$(call PUBLISH_CRATE,lindera-unidic,$(call crate_version,lindera-unidic))
	$(call PUBLISH_CRATE,lindera-sudachidict,$(call crate_version,lindera-sudachidict))
	$(call PUBLISH_CRATE,lindera,$(LINDERA_VERSION))
	$(call PUBLISH_CRATE,lindera-analysis,$(call crate_version,lindera-analysis))
	$(call PUBLISH_CRATE,lindera-cli,$(call crate_version,lindera-cli))
	$(call PUBLISH_CRATE,lindera-binding-core,$(call crate_version,lindera-binding-core))
	$(call PUBLISH_CRATE,lindera-python,$(call crate_version,lindera-python))
	$(call PUBLISH_CRATE,lindera-nodejs,$(call crate_version,lindera-nodejs))
	$(call PUBLISH_CRATE,lindera-ruby,$(call crate_version,lindera-ruby))
	$(call PUBLISH_CRATE,lindera-php,$(call crate_version,lindera-php))
	$(call PUBLISH_CRATE,lindera-wasm,$(call crate_version,lindera-wasm))
