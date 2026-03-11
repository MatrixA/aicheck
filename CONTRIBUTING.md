# Contributing to AICheck

Thanks for your interest in contributing! Here's how to get started.

## Development Setup

```bash
# Clone the repo
git clone https://github.com/MatrixA/aicheck.git
cd aicheck

# Build
cargo build

# Run tests
cargo test

# Run the CLI
cargo run -- check photo.jpg
```

Requires **Rust 1.86+**.

## Before Submitting a PR

Please make sure all checks pass locally:

```bash
cargo fmt -- --check   # formatting
cargo clippy -- -D warnings   # lints
cargo test   # tests
```

## Pull Request Process

1. Fork the repo and create a branch from `main`
2. Make your changes
3. Ensure all checks above pass
4. Open a PR with a clear description of what changed and why

## Adding a New AI Tool

To add detection for a new AI tool, update the pattern list in `src/known_tools.rs`. Each entry needs a case-insensitive pattern and a canonical tool name.

## Adding a New Detection Method

New detectors go in `src/detector/`. Implement detection logic, then register it in `src/detector/mod.rs` within `run_all_detectors()`.

## Reporting Bugs

Use the [bug report template](https://github.com/MatrixA/aicheck/issues/new?template=bug_report.md) and include:
- The file you tested (or a description if you can't share it)
- Expected vs actual output
- Your OS and Rust version

## License

By contributing, you agree that your contributions will be licensed under AGPL-3.0-or-later.
