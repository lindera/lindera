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
cargo test -p lindera-dictionary --features train
```

### All Features for a Crate

Run the full test suite for a single crate (matches CI):

```bash
cargo test -p <crate> --all-features
```

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
