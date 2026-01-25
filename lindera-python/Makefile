.DEFAULT_GOAL := help

VERSION := $(shell poetry version -s)
LINDERA_PYTHON_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-python") | .version')

USER_AGENT ?= $(shell curl --version | head -n1 | awk '{print $1"/"$2}')
USER ?= $(shell whoami)
HOSTNAME ?= $(shell hostname)

init: ## Initialize the project
	poetry self add poetry-plugin-export
	poetry config virtualenvs.in-project true
	poetry install --no-root

update: ## Update the project dependencies
	poetry update

clean: ## Clean the project
	cargo clean
	find . | grep -E "(__pycache__|.pytest_cache|.mypy_cache|\.pyc|\.pyo$$)" | xargs rm -rf

format: ## Format the project
	cargo fmt
	poetry run isort ./examples ./tests
	poetry run black ./examples ./tests

lint: ## Lint the project
	cargo clippy --features=embed-ipadic,train
	poetry run isort --check-only --diff ./examples ./tests
	poetry run black --check ./examples ./tests
	poetry run flake8 ./examples ./tests
	poetry run mypy ./examples ./tests

develop: ## Build Python module in development mode and install it into the current Python environment
	poetry run maturin develop --features=embed-ipadic,train

build: ## Build the project
	poetry run maturin build -i python --release --features=embed-ipadic,train

.PHONY: tests
test: ## Test the project
	cargo test --features=embed-ipadic,train
	poetry run maturin develop --features=embed-ipadic,train
	poetry run pytest -v ./tests

.PHONY: run-examples
run-examples: ## Run examples
	poetry run maturin develop --features=embed-ipadic,train
	poetry run python ./examples/build_ipadic.py
	poetry run python ./examples/tokenize.py
	poetry run python ./examples/tokenize_with_userdict.py
	poetry run python ./examples/tokenize_with_decompose.py
	poetry run python ./examples/tokenize_with_filters.py
	poetry run python ./examples/train_and_export.py

publish: ## Publish package to crates.io
ifeq ($(shell curl -s -XGET -H "User-Agent: $(USER_AGENT) ($(USER)@$(HOSTNAME))" https://crates.io/api/v1/crates/lindera-python | jq -r '.versions[].num' | grep $(LINDERA_PYTHON_VERSION)),)
	(cargo package && cargo publish)
endif

tag: ## Make a tag
	git tag v$(VERSION)
	git push origin v$(VERSION)

help: ## Show help
	@echo "Available targets:"
	@grep -E '^[a-zA-Z0-9_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  %-15s %s\n", $$1, $$2}'
