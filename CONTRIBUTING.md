# Contributing to Shepherd

Thank you for considering contributing to Shepherd! This guide will help you get started.

## Getting Started

1. Fork the repository and clone your fork
2. Install Rust (stable) and Node.js 20+
3. Install system dependencies (Linux): `sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`
4. Run `npm ci` to install frontend dependencies

### Building

```bash
cargo build --workspace
npm run build
cargo build --manifest-path src-tauri/Cargo.toml
```

### Running Tests

```bash
# Rust tests
cargo test --workspace

# Frontend tests
npx vitest run

# E2E tests
npx playwright test

# Formatting
cargo fmt --all -- --check

# Linting
cargo clippy --workspace --all-targets -- -D warnings
```

## Pull Requests

1. Create a feature branch from `main`
2. Write tests for new functionality (TDD preferred)
3. Ensure all tests pass: `cargo test --workspace && npx vitest run`
4. Run `cargo fmt --all` before committing
5. Keep commits focused and well-described
6. Open a PR against `main` with a clear description

### PR Checklist

- [ ] Tests added/updated
- [ ] `cargo fmt --all` passes
- [ ] `cargo clippy --workspace` passes
- [ ] `cargo test --workspace` passes
- [ ] Frontend tests pass (`npx vitest run`)

## Code Style

- Follow existing patterns in the codebase
- Use `rustfmt` and `clippy` defaults (configured in `rustfmt.toml` and `clippy.toml`)
- Prefer small, focused commits

## License

By contributing, you agree that your contributions will be licensed under the Apache-2.0 License.
