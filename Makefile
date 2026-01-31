# Versions
GET_VERSION = $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="$(1)") | .version')

LINDERA_DICTIONARY_VERSION := $(call GET_VERSION,lindera-dictionary)
LINDERA_CC_CEDICT_VERSION := $(call GET_VERSION,lindera-cc-cedict)
LINDERA_IPADIC_VERSION := $(call GET_VERSION,lindera-ipadic)
LINDERA_IPADIC_NEOLOGD_VERSION := $(call GET_VERSION,lindera-ipadic-neologd)
LINDERA_KO_DIC_VERSION := $(call GET_VERSION,lindera-ko-dic)
LINDERA_UNIDIC_VERSION := $(call GET_VERSION,lindera-unidic)
LINDERA_VERSION := $(call GET_VERSION,lindera)
LINDERA_CLI_VERSION := $(call GET_VERSION,lindera-cli)
LINDERA_PYTHON_VERSION := $(call GET_VERSION,lindera-python)
LINDERA_WASM_VERSION := $(call GET_VERSION,lindera-wasm)

# Environment
USER_AGENT ?= $(shell curl --version | head -n1 | awk '{print $1"/"$2}')
USER ?= $(shell whoami)
HOSTNAME ?= $(shell hostname)

# Python Configuration
PYTHON_DIR = lindera-python
PYTHON_FEATURES = embed-ipadic,train
POETRY = cd $(PYTHON_DIR) && ../.venv/bin/poetry
POETRY_RUN = $(POETRY) run

# WASM Configuration
WASM_DIR = lindera-wasm
WASM_FEATURES = embed-ipadic

.DEFAULT_GOAL := help

.PHONY: help init clean format lint test build bench bench-all \
	format-all lint-all test-all build-all \
	python-update python-format python-lint python-clean python-build python-test python-develop python-run-examples \
	wasm-build wasm-test wasm-publish wasm-clean wasm-build-example wasm-run-example \
	tag publish

# Common targets
help: ## Show help
	@echo "Available targets:"
	@grep -E '^[a-zA-Z0-9_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  %-20s %s\n", $$1, $$2}'

init: ## Initialize the project
	python -m venv .venv
	.venv/bin/pip install poetry
	$(POETRY) self add poetry-plugin-export
	$(POETRY) config virtualenvs.in-project true
	$(POETRY) install --no-root

clean: python-clean wasm-clean ## Clean the project
	cargo clean

# Rust targets
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
	@$(foreach dict,ipadic ipadic_neologd unidic ko_dic cc_cedict, \
		echo "📊 Running $(dict) benchmark..."; \
		(cd lindera && cargo bench --bench=bench_$(dict) --features=embed-$(subst _,-,$(dict))) || true; \
		echo "";)
	@echo "✅ All benchmarks completed!"
	@echo "📈 Results are available in lindera/target/criterion/"

bench-all: ## Run all benchmarks with all features enabled
	@echo "🚀 Running all Lindera benchmarks with all features..."
	(cd lindera && cargo bench --all-features)
	@echo "✅ All benchmarks completed!"

# All-in-one targets
format-all: format python-format ## Format all projects

lint-all: lint python-lint ## Lint all projects

test-all: test python-test wasm-test ## Test all projects

build-all: build python-build wasm-build ## Build all projects

# Python targets
python-update: ## Update the python project dependencies
	$(POETRY) update

python-format: ## Format the python project
	$(POETRY_RUN) isort ./examples ./tests
	$(POETRY_RUN) black ./examples ./tests

python-lint: ## Lint the python project
	(cd $(PYTHON_DIR) && cargo clippy --features=$(PYTHON_FEATURES))
	$(POETRY_RUN) isort --check-only --diff ./examples ./tests
	$(POETRY_RUN) black --check ./examples ./tests
	$(POETRY_RUN) flake8 ./examples ./tests
	$(POETRY_RUN) mypy ./examples ./tests

python-clean: ## Clean the python project
	rm -rf $(PYTHON_DIR)/dist
	rm -rf $(PYTHON_DIR)/lindera_python.egg-info
	find . | grep -E "(__pycache__|.pytest_cache|.mypy_cache|\.pyc|\.pyo$$)" | xargs rm -rf

python-build: ## Build the python project
	$(POETRY_RUN) maturin build -i python --release --features=$(PYTHON_FEATURES)

python-test: python-develop ## Test the python project
	(cd $(PYTHON_DIR) && cargo test --features=$(PYTHON_FEATURES))
	$(POETRY_RUN) python -m pytest -v ./tests

python-develop: ## Build Python module in development mode
	$(POETRY_RUN) maturin develop --features=$(PYTHON_FEATURES)

python-run-examples: python-develop ## Run python examples
	$(foreach example,build_ipadic tokenize tokenize_with_userdict tokenize_with_decompose tokenize_with_filters train_and_export, \
		$(POETRY_RUN) python ./examples/$(example).py;)

# WASM targets
wasm-build: ## Build the WASM project
	(cd $(WASM_DIR) && wasm-pack build --release --features=$(WASM_FEATURES) --target=web)

wasm-test: ## Test the WASM project
	(cd $(WASM_DIR) && wasm-pack test --node --features=$(WASM_FEATURES))

wasm-publish: ## Publish the WASM project to npm
	(cd $(WASM_DIR) && wasm-pack publish --access=public --target=web)

wasm-clean: ## Clean the WASM project
	(cd $(WASM_DIR) && cargo clean)
	rm -rf $(WASM_DIR)/pkg
	rm -rf $(WASM_DIR)/example/dist
	rm -rf $(WASM_DIR)/example/node_modules
	rm -f $(WASM_DIR)/example/package-lock.json
	rm -f $(WASM_DIR)/example/temp.json

wasm-build-example: ## Build the WASM example application
	(cd $(WASM_DIR) && wasm-pack build --release --features=embed-ipadic --target=web)
	cd $(WASM_DIR)/example && \
	jq '.version = "$(LINDERA_WASM_VERSION)"' ./package.json > ./temp.json && mv ./temp.json ./package.json && \
	npm install && \
	npm run build && \
	cp index.html dist/index.html

wasm-run-example: ## Run the WASM example application
	(cd $(WASM_DIR)/example && npm run start)

# Release targets
tag: ## Make a tag
	git tag v$(LINDERA_VERSION)
	git push origin v$(LINDERA_VERSION)

# Publish Macro
define PUBLISH_CRATE
	@if [ -z "$$(curl -s -XGET -H "User-Agent: $(USER_AGENT) ($(USER)@$(HOSTNAME))" https://crates.io/api/v1/crates/$(1) | jq -r '.versions[].num | select(. == "$(2)")')" ]; then \
		echo "🚀 Publishing $(1) v$(2)..."; \
		(cd $(1) && cargo package && cargo publish); \
		sleep 10; \
	else \
		echo "✅ $(1) v$(2) is already published."; \
	fi
endef

publish: ## Publish packages to crates.io
	$(call PUBLISH_CRATE,lindera-dictionary,$(LINDERA_DICTIONARY_VERSION))
	$(call PUBLISH_CRATE,lindera-cc-cedict,$(LINDERA_CC_CEDICT_VERSION))
	$(call PUBLISH_CRATE,lindera-ipadic,$(LINDERA_IPADIC_VERSION))
	$(call PUBLISH_CRATE,lindera-ipadic-neologd,$(LINDERA_IPADIC_NEOLOGD_VERSION))
	$(call PUBLISH_CRATE,lindera-ko-dic,$(LINDERA_KO_DIC_VERSION))
	$(call PUBLISH_CRATE,lindera-unidic,$(LINDERA_UNIDIC_VERSION))
	$(call PUBLISH_CRATE,lindera,$(LINDERA_VERSION))
	$(call PUBLISH_CRATE,lindera-python,$(LINDERA_PYTHON_VERSION))
	$(call PUBLISH_CRATE,lindera-cli,$(LINDERA_CLI_VERSION))
	$(call PUBLISH_CRATE,lindera-wasm,$(LINDERA_WASM_VERSION))
