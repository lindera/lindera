# Build & Test

## Build

### Default Build

Build the workspace with default features (`mmap`):

```bash
cargo build
```

### Build with Training Support

Include CRF-based dictionary training functionality:

```bash
cargo build --features train
```

### Build CLI Only

```bash
cargo build -p lindera-cli
```

The CLI has the `train` feature enabled by default.

## Test

### Single Test

Run a specific test within a crate (recommended for development):

```bash
cargo test -p <crate> <test_name>
```

### Training Feature Tests

```bash
cargo test -p lindera-trainer
```

### All Features for a Crate

Run the full test suite for a single crate:

```bash
cargo test -p <crate> --all-features
```

> Note: CI does not use `--all-features` -- it runs each crate with a curated, crate-specific feature combination (see `.github/workflows/regression.yml`). The Makefile's per-crate pattern targets (`make test-<crate>`, `make lint-<crate>`) apply the same feature combinations CI uses and are the closest local equivalent. CI also tests `lindera-segmenter` and `lindera` once more with every dictionary embedded (on Linux only), which runs the tests gated on the other dictionaries; to do the same locally, run `cargo test -p lindera --features embed-ipadic,embed-ipadic-neologd,embed-unidic,embed-sudachidict,embed-ko-dic,embed-cc-cedict,embed-jieba` (and likewise for `lindera-segmenter`).

### Workspace-Wide Tests

```bash
cargo test
```

## Quality Checks

### Format Check

Verify code formatting matches the project style:

```bash
cargo fmt --all -- --check
```

To auto-fix formatting:

```bash
cargo fmt --all
```

### Lint

Run Clippy with warnings treated as errors:

```bash
cargo clippy -- -D warnings
```

> Note: CI runs `cargo clippy --all-targets -- -D warnings` for each crate, with the features it tests that crate with (see `.github/workflows/test-crate.yml`). Run clippy locally (e.g. via `make lint`) before opening a PR.

## Documentation

### API Documentation

Generate and open Rust API documentation:

```bash
cargo doc --no-deps --open
```

### mdBook Documentation

Build the user-facing documentation:

```bash
mdbook build docs
```

Preview locally at `http://localhost:3000`:

```bash
mdbook serve docs
```

### Markdown Lint

Check documentation for Markdown style issues:

```bash
markdownlint-cli2 "docs/src/**/*.md"
```

Rules are configured in `.markdownlint.json` at the repository root.
