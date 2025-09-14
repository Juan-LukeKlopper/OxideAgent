# CI/CD Setup

This project uses GitHub Actions for continuous integration and deployment.

## Workflow

The CI workflow is defined in `.github/workflows/ci.yml` and includes the following steps:

1. **Code Formatting Check** - Ensures consistent code formatting using `cargo fmt`
2. **Clippy Linting** - Runs Rust linter to catch common mistakes and improve code quality
3. **Build** - Compiles the project to ensure there are no compilation errors
4. **Tests** - Runs all unit and integration tests
5. **Documentation** - Builds the documentation

## Badges

You can add badges to your README to show the status of your CI pipeline:

```markdown
![CI](https://github.com/your-username/OxideAgent/workflows/CI/badge.svg)
```

Replace `your-username` with your actual GitHub username.

## Local Testing

To run the same checks locally:

```bash
# Check code formatting
cargo fmt --check

# Run clippy linter
cargo clippy -- -D warnings

# Build the project
cargo build

# Run tests
cargo test

# Build documentation
cargo doc
```