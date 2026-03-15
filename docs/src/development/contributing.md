# Contributing

Thank you for your interest in contributing to Lindera! This page provides guidelines to help you get started.

## Getting Started

1. Fork the repository on GitHub.
2. Clone your fork locally:

    ```bash
    git clone https://github.com/<your-username>/lindera.git
    cd lindera
    ```

3. Create a feature branch:

    ```bash
    git checkout -b feature/my-feature
    ```

4. Make your changes, then verify they pass all checks:

    ```bash
    cargo fmt --all -- --check
    cargo clippy -- -D warnings
    cargo test
    ```

5. Commit and push your changes, then open a pull request.

## Code Style

- Follow the existing code style in the repository.
- Run `cargo fmt` before committing.
- All public and private items (types, functions, modules, fields, constants, type aliases) must have documentation comments (`///`).
- Trait implementation methods should also have documentation comments describing implementation-specific behavior.
- Function and method documentation should include `# Arguments` and `# Returns` sections where applicable.
- Code comments, documentation comments, commit messages, log messages, and error messages should be written in English.
- Avoid `unwrap()` and `expect()` in production code (test code is fine).
- Use `unsafe` blocks only when necessary, and always include a `// SAFETY: ...` comment.
- Use file-based module style (`src/tokenizer.rs`) instead of `mod.rs` style.

## Testing

- Write unit tests for all new functionality.
- Run the relevant test(s) during development for fast feedback:

    ```bash
    cargo test -p <crate> <test_name>
    ```

- When working with the `train` feature, include the feature flag:

    ```bash
    cargo test -p lindera-dictionary --features train
    ```

## Commit Messages

Follow the [Conventional Commits](https://www.conventionalcommits.org/) specification. Write commit messages in English.

Examples:

- `feat: add Korean dictionary support`
- `fix: correct character category ID in trainer`
- `docs: update installation instructions`
- `refactor: split large training method into smaller functions`

## Documentation

- If your change affects user-facing documentation, update the relevant files in `docs/src/`.
- After editing Markdown files, verify there are no lint errors:

    ```bash
    markdownlint-cli2 "docs/src/**/*.md"
    ```

- Rules are configured in `.markdownlint.json` at the repository root.

## Dependencies

When adding new dependencies, verify license compatibility. Lindera uses the MIT / Apache-2.0 dual license.

## Feature Flags

Use `#[cfg(feature = "train")]` for conditional compilation of training-related code. See [Feature Flags](./feature_flags.md) for a full list.

## Reporting Issues

When reporting a bug, please include:

- Lindera version (`lindera --version` or check `Cargo.toml`)
- Rust version (`rustc --version`)
- Operating system
- Steps to reproduce the issue
- Expected and actual behavior
