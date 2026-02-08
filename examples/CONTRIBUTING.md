# Contributing to Reinhardt Examples

Thank you for your interest in contributing to Reinhardt Examples! This document provides guidelines for contributing to the examples repository.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [How to Contribute](#how-to-contribute)
- [Adding New Examples](#adding-new-examples)
- [Testing Guidelines](#testing-guidelines)
- [Code Style Requirements](#code-style-requirements)
- [Git Workflow](#git-workflow)
- [Pull Request Process](#pull-request-process)

## Code of Conduct

This project follows the same Code of Conduct as the main Reinhardt Web Framework repository. Please be respectful and constructive in all interactions.

## How to Contribute

### Types of Contributions

1. **New Examples**: Add examples demonstrating framework features
2. **Bug Fixes**: Fix issues in existing examples
3. **Documentation**: Improve README files and comments
4. **Tests**: Add or improve test coverage
5. **Performance**: Optimize example code

## Adding New Examples

### Step 1: Choose Directory

- **Local Examples** (`local/`): For development using latest framework code
- **Remote Examples** (`remote/`): For published versions (after framework is on crates.io)

Most new examples should start in `local/`.

### Step 2: Create Example Structure

```bash
cd local
mkdir examples-my-feature
cd examples-my-feature

# Create necessary files
touch src/main.rs
touch Cargo.toml
touch README.md
```

### Step 3: Configure Cargo.toml

```toml
[package]
name = "examples-my-feature"
version = "0.1.0-alpha.1"
edition = "2024"
publish = false

[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["core", "conf"] }
example-common = { path = "../../remote/common" }

[dev-dependencies]
example-test-macros = { path = "../../remote/test-macros" }
reinhardt-test = { git = "https://github.com/kent8192/reinhardt-web" }
rstest = "0.24"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
```

### Step 4: Add to Workspace

Edit `local/Cargo.toml` and add your example to the `members` array:

```toml
[workspace]
members = [
	"examples-hello-world",
	"examples-rest-api",
	"examples-my-feature",  # Add here
]
```

### Step 5: Write Code and Tests

Follow the [Testing Guidelines](#testing-guidelines) below.

### Step 6: Document Your Example

Create a comprehensive `README.md` in your example directory:

```markdown
# Example: My Feature

Brief description of what this example demonstrates.

## Features

- Feature 1
- Feature 2

## Running

\`\`\`bash
cargo run
\`\`\`

## Testing

\`\`\`bash
cargo test
\`\`\`
```

## Testing Guidelines

### Use rstest for All Tests

All tests MUST use `#[rstest]` instead of plain `#[test]`:

```rust
use rstest::*;

#[rstest]
#[tokio::test]
async fn test_example() {
	// Test code
}
```

### Use Standard Fixtures

Use fixtures from `reinhardt-test` for consistent testing:

```rust
use reinhardt_test::fixtures::test_server_guard;
use reinhardt_test::resource::TeardownGuard;

#[rstest]
#[tokio::test]
async fn test_endpoint(
	#[future] test_server_guard: TeardownGuard<reinhardt_test::fixtures::TestServerGuard>,
) {
	let server = test_server_guard.await;
	let base_url = server.base_url();

	// Test code
}
```

### Test Coverage Requirements

- **Normal cases**: Test expected behavior
- **Error cases**: Test 4xx/5xx responses
- **Edge cases**: Test boundary conditions
- **Cleanup**: Use fixtures that handle cleanup automatically

### No Skeleton Tests

All tests MUST have meaningful assertions:

```rust
// ✅ GOOD
#[rstest]
#[tokio::test]
async fn test_endpoint() {
	let response = client.get("/api/users").send().await.unwrap();
	assert_eq!(response.status(), StatusCode::OK);
	assert!(response.text().await.unwrap().contains("users"));
}

// ❌ BAD: No assertions
#[rstest]
#[tokio::test]
async fn test_endpoint() {
	let response = client.get("/api/users").send().await.unwrap();
	// Missing assertions!
}
```

## Code Style Requirements

### Follow Project Standards

This repository inherits standards from the main Reinhardt repository:

- **Rust 2024 Edition**: Use latest features
- **Module System**: Use `module.rs` + `module/` directory (NOT `mod.rs`)
- **Comments**: ALL code comments MUST be in English
- **Formatting**: Run `cargo fmt` before committing
- **Linting**: Run `cargo clippy` and fix all warnings

### Code Quality Checks

Before submitting:

```bash
# Format check
cargo fmt --check

# Clippy
cargo clippy --workspace --all --all-features -- -D warnings

# Build
cargo build --workspace --all --all-features

# Test
cargo test --workspace --all --all-features

# Or with nextest
cargo nextest run --workspace --all --all-features
```

## Git Workflow

### Branch Naming

Use descriptive branch names:

- `feat/add-websocket-example` - New feature
- `fix/rest-api-bug` - Bug fix
- `docs/improve-readme` - Documentation
- `test/add-coverage` - Test improvements

### Commit Message Format

Follow [Conventional Commits v1.0.0](https://www.conventionalcommits.org/):

```
<type>[scope]: <description>

[optional body]

[optional footer]
```

**Types**:
- `feat`: New example or feature
- `fix`: Bug fix
- `docs`: Documentation only
- `test`: Adding or updating tests
- `refactor`: Code refactoring
- `chore`: Maintenance tasks

**Examples**:
```bash
feat(rest-api): add authentication example
fix(hello-world): correct port binding issue
docs: update README with new examples
test(database): add migration test coverage
```

**Important**:
- Start description with lowercase letter
- No period at the end
- Keep first line under 72 characters

## Pull Request Process

### 1. Fork and Clone

If you haven't already, fork the repository and clone your fork:

```bash
git clone https://github.com/YOUR_USERNAME/reinhardt-examples.git
cd reinhardt-examples
```

### 2. Create Branch

```bash
git checkout -b feat/my-new-example
```

### 3. Make Changes

- Add your example
- Write tests
- Update documentation
- Run quality checks

### 4. Commit Changes

```bash
git add .
git commit -m "feat(my-feature): add new example demonstrating X"
```

### 5. Push to Fork

```bash
git push origin feat/my-new-example
```

### 6. Create Pull Request

- Go to GitHub and create a PR
- Fill out the PR template
- Link related issues
- Request review

### PR Requirements

Before your PR can be merged:

- ✅ All CI checks must pass
- ✅ Code review approval required
- ✅ All conversations resolved
- ✅ Commits follow Conventional Commits format
- ✅ Tests added/updated
- ✅ Documentation updated

### PR Template

When creating a PR, include:

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] New example
- [ ] Bug fix
- [ ] Documentation
- [ ] Test coverage

## Testing
Describe how you tested your changes

## Checklist
- [ ] Code follows project style
- [ ] Tests pass locally
- [ ] Documentation updated
- [ ] Commit messages follow format
```

## Additional Guidelines

### File Organization

```
examples-my-feature/
├── src/
│   └── main.rs
├── tests/
│   └── integration.rs
├── settings/
│   ├── base.toml
│   └── local.toml
├── Cargo.toml
├── Makefile.toml
├── README.md
└── .gitignore
```

### Documentation

- **README.md**: Required in every example
- **Code Comments**: Explain non-obvious logic
- **API Documentation**: Use doc comments (`///`) for public items

### Performance

- Avoid unnecessary dependencies
- Use async/await appropriately
- Profile if performance is critical

### Security

- No hardcoded credentials
- Use environment variables for secrets
- Document security considerations

## Getting Help

- **Issues**: Open an issue for bugs or questions
- **Discussions**: Use GitHub Discussions for general questions
- **Main Repository**: See [reinhardt-web](https://github.com/kent8192/reinhardt-web)

## License

By contributing, you agree that your contributions will be licensed under the same dual MIT OR Apache-2.0 license as the project.

---

Thank you for contributing to Reinhardt Examples!
