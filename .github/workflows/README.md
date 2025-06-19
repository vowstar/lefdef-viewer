# GitHub Actions Workflows

This directory contains GitHub Actions workflows for the lefdef-viewer project.

## Available Workflows

### CI Workflow (`ci.yml`)

The CI workflow runs on every push to the main branch and on pull requests. It performs the following tasks:

- Builds the project on Ubuntu, Windows, and macOS
- Runs all tests
- Caches dependencies to speed up future builds

### Lint Workflow (`lint.yml`)

The Lint workflow runs on every push to the main branch and on pull requests. It performs the following tasks:

- Runs `cargo check` to verify that the code compiles
- Runs `cargo fmt` to check code formatting
- Runs `cargo clippy` to check for common mistakes and improve code quality

### Release Workflow (`release.yml`)

The Release workflow runs when a tag with the format `v*.*.*` is pushed. It performs the following tasks:

- Builds release binaries for Ubuntu, Windows, and macOS
- Creates a GitHub release
- Uploads the binaries as assets to the release

## How to Use

### Creating a Release

To create a new release:

1. Update the version in `Cargo.toml`
2. Commit the changes
3. Create and push a new tag:

   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```

This will trigger the Release workflow, which will create a GitHub release with the compiled binaries.

### Running Tests Locally

Before pushing changes, you can run the same checks locally that the CI and Lint workflows run:

```bash
# Build and run tests
cargo test

# Check code formatting
cargo fmt -- --check

# Run clippy
cargo clippy -- -D warnings
```

## Workflow Status

You can check the status of the workflows in the [Actions tab](https://github.com/vowstar/lefdef-viewer/actions) of the repository.

The status badges are also displayed in the main README.md file.
