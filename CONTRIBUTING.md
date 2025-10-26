# Contributing to Reinhardt

Thank you for your interest in contributing to Reinhardt! This document provides guidelines and best practices for contributing to the project.

## Table of Contents

- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Code Style & Conventions](#code-style--conventions)
- [Testing Guidelines](#testing-guidelines)
- [Commit Guidelines](#commit-guidelines)
- [Pull Request Process](#pull-request-process)
- [Documentation](#documentation)
- [Getting Help](#getting-help)

---

## Getting Started

### Prerequisites

- Rust 1.75+ (2024 Edition)
- PostgreSQL (for database-related tests)
- Docker (optional, for TestContainers)

### Fork and Clone

1. Fork the repository on GitHub
2. Clone your fork locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/reinhardt.git
   cd reinhardt
   ```
3. Add the upstream repository:
   ```bash
   git remote add upstream https://github.com/ORIGINAL_OWNER/reinhardt.git
   ```

### Building the Project

```bash
# Build the entire workspace
cargo build --workspace

# Build specific crate
cargo build --package reinhardt-orm

# Build with all features
cargo build --workspace --all-features
```

### Running Tests

```bash
# Run all tests
cargo test --workspace

# Run tests for a specific crate
cargo test --package reinhardt-orm

# Run integration tests
cargo test --package reinhardt-integration-tests
```

---

## Development Setup

### Project Structure

Reinhardt uses a workspace structure:

- **Functional crates**: `crates/reinhardt-*/` - Individual feature implementations
- **Tests crate**: `tests/` - Integration tests only
- **Each crate**: Contains its own unit tests in `tests/` or inline

### Module System

**IMPORTANT**: Reinhardt uses Rust 2024 Edition module system:

- L **DO NOT USE** `mod.rs` files
-  **USE** `module_name.rs` files instead
-  **DECLARE** modules in `lib.rs` or parent module with `mod module_name;`

Example:
```rust
// In lib.rs
pub mod http;
pub mod routing;

// File structure:
// src/
//   lib.rs
//   http.rs          //  Correct
//   routing.rs       //  Correct
//   http/mod.rs      // L DO NOT USE
```

---

## Code Style & Conventions

### General Guidelines

1. **Follow Rust naming conventions**:
   - `snake_case` for functions, variables, modules
   - `PascalCase` for types, traits, enums
   - `SCREAMING_SNAKE_CASE` for constants

2. **Code organization**:
   - Keep related functionality together
   - Maintain clear separation of concerns
   - Group imports logically (std, external crates, internal crates)

3. **String conversions**:
   - **MINIMIZE** `.to_string()` calls
   - Prefer borrowing and string slices where possible
   - Use `String::from()`, `format!()`, or `Cow<str>` when appropriate

### Code Cleanup

- **DELETE obsolete code immediately** - don't leave commented-out code
- **NO comments documenting deleted code** - Git history is the record
- Extract important notes to `docs/IMPLEMENTATION_NOTES.md` if needed

### TODO and Placeholder Policy

**TODO Comments**:
- Use `// TODO:` for planning and unimplemented features
- Explain what needs to be implemented and why it's pending
- **DELETE** when functionality is implemented

**Runtime Markers**:
- Use `todo!()` for features that **WILL** be implemented
- Use `unimplemented!()` for features that **WILL NOT** be implemented (intentionally omitted)

**Placeholder Implementations**:
- **ALL** placeholder implementations **MUST** be marked with `todo!()` or `// TODO:`
- This includes:
  - Empty function bodies returning default values
  - Stub implementations with minimal logic
  - Mock implementations intended to be replaced
- **Exception**: Tests and documentation may use simplified implementations

Examples:
```rust
//  Good: Marked placeholder
pub fn get_cache_config() -> CacheConfig {
    todo!("Implement cache configuration loading from settings")
}

//  Good: Using TODO comment
pub fn validate_input(data: &str) -> Result<()> {
    // TODO: Add input validation logic - planned for next sprint
    Ok(())
}

//  Good: Intentionally not implemented
fn legacy_api() -> String {
    unimplemented!("This legacy API is intentionally not supported")
}

// L Bad: Unmarked placeholder
pub fn get_config() -> Config {
    Config::default()  // Looks production-ready!
}
```

### Path References

- **DO NOT USE** relative paths going up more than one level (e.g., `../..`)
- **PREFER** absolute paths or single-level relative paths (e.g., `../`)
- Deep relative paths make code harder to understand

---

## Testing Guidelines

### Testing Philosophy

1. **NO skeleton implementations**: Tests MUST contain meaningful assertions
   - Tests must be capable of failing when code is incorrect
   - L Bad: `assert!(true)` or empty test bodies
   -  Good: Tests with real assertions

2. **Use Reinhardt components**: Every test MUST use at least one Reinhardt crate component

3. **Documentation tests**: Implement documentation tests for all features
   - Don't duplicate doc tests as unit/integration tests

### Test Organization

**Unit Tests**:
- Use exactly ONE Reinhardt crate
- Place within the functional crate being tested
- Test that specific crate's functionality in isolation

**Integration Tests**:
- Use TWO or MORE Reinhardt crates
- **MUST** be placed in the `tests` crate
- Test interactions between multiple crates

**Dependency Rules**:
- Functional crates **MUST NOT** include other Reinhardt crates as `dev-dependencies`
- This ensures unit tests remain isolated

### Test Implementation

**Global State Management**:
- Tests modifying global state MUST be serialized using `serial_test` crate
- Use named serial groups: `#[serial(group_name)]`
- Common groups: `i18n`, `url_overrides`, etc.
- Always call cleanup functions in teardown

Example:
```rust
use serial_test::serial;

#[test]
#[serial(i18n)]
fn test_translation() {
    activate("fr", catalog);
    // test code
    deactivate(); // cleanup
}
```

**Test Cleanup**:
- **ALL** files, directories, or environmental changes created during tests **MUST** be deleted upon completion
- Use test fixtures, `Drop` implementations, or explicit cleanup

**Infrastructure Testing**:
- Use **TestContainers** for tests requiring actual infrastructure (databases, message queues)
- Prefer real infrastructure over mocks when feasible

---

## Commit Guidelines

### Commit Policy

**CRITICAL RULES**:
- **NEVER** create commits without explicit user instruction
- **NEVER** push commits without explicit user instruction
- Always wait for user confirmation before committing

### Commit Granularity

- Each commit should represent a **single logical change**
- **Each commit MUST be small enough to be explained in a single line**
  - If you can't describe the commit clearly in one line, it's too large
  - Split large changes into multiple commits
- Group files by change purpose at a fine-grained level

### Partial File Commits

When a file contains changes with different purposes, use:

**Method 1: Editor-based Patch Editing (Recommended)**
```bash
git add -e <file>
```

**Method 2: Patch File Approach**
```bash
git diff <file> > /tmp/changes.patch
# Edit patch file
git apply --cached /tmp/changes.patch
```

### Commit Message Format

**Structure**:
```
type(scope): Brief description in English

Body explaining the changes in detail.

Module Section:
- file/path.rs: +XXX lines - Description
  - Sub-detail 1
  - Sub-detail 2
- Removed: old_file.rs (reason)

Features:
- Feature 1
- Feature 2

> Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
```

**Type Values**:
- `feat`: New feature
- `fix`: Bug fix
- `refactor`: Code refactoring
- `test`: Testing changes
- `docs`: Documentation changes
- `chore`: Build/tooling changes
- `perf`: Performance improvements
- `style`: Code style changes

**Scope**: Module or component name (e.g., `orm`, `http`, `shortcuts`)

**Requirements**:
- Subject line: Be specific, not vague
- Body: Organize by module/component, list file changes with line counts
- Footer: Include Claude Code attribution and Co-Authored-By line
- Exactly one blank line between body and footer

**Style Reference**:
Always examine recent commits before writing new ones:
```bash
git log --pretty=format:"%s%n%b" -10
```

For detailed commit guidelines, see [CLAUDE.commit.md](CLAUDE.commit.md).

---

## Pull Request Process

### Before Submitting

1. **Ensure all tests pass**:
   ```bash
   cargo test --workspace
   ```

2. **Run code formatting**:
   ```bash
   cargo fmt --all
   ```

3. **Check for linting issues**:
   ```bash
   cargo clippy --workspace -- -D warnings
   ```

4. **Update documentation** (see [Documentation](#documentation) section)

### Submitting a Pull Request

1. **Create a feature branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes** following the guidelines in this document

3. **Commit your changes** following commit guidelines

4. **Push to your fork**:
   ```bash
   git push origin feature/your-feature-name
   ```

5. **Open a Pull Request** on GitHub with:
   - Clear title describing the change
   - Description explaining what and why
   - References to related issues

### PR Review Process

- Maintainers will review your PR
- Address feedback and requested changes
- Once approved, maintainers will merge your PR

---

## Documentation

### Update Requirements

**ALWAYS** update documentation when implementing or modifying features:

- **README.md**: Project-level overview
- **Crate README.md**: Individual crate documentation
- **docs/** directory: Detailed guides and tutorials

### Documentation Consistency

- Ensure consistency across all documentation levels

### Documentation Scope

Update documentation for:
- **New features**: Add descriptions, usage examples, API references
- **Modified features**: Update affected sections
- **Deprecated features**: Mark as deprecated, provide migration guides
- **Removed features**: Remove documentation, add migration notes

### Documentation Quality

- Ensure examples are tested and working
- Update code snippets to reflect current API
- Verify all links and references are valid
- Maintain consistency in terminology and formatting

---

## Getting Help

### Resources

- =� [Getting Started Guide](docs/GETTING_STARTED.md)
- =� [Feature Flags Guide](docs/FEATURE_FLAGS.md)
- =� [Project Instructions](CLAUDE.md)
- =� GitHub Discussions (for questions and ideas)
- = GitHub Issues (for bug reports)

### Before Asking

Please check:
-  [Getting Started Guide](docs/GETTING_STARTED.md)
-  [Examples](examples/)
-  Existing GitHub Issues and Discussions
-  [CLAUDE.md](CLAUDE.md) for project-specific guidelines

---

## Quick Reference

### Critical Rules

**Code & Module System**:
- L NO `mod.rs` files
- L NO TODO/NOTE comments in user-facing placeholders
- L NO unmarked placeholder implementations
-  USE 2024 edition module system
-  MARK ALL placeholders with `todo!()` or `// TODO:`

**Testing**:
- L NO skeleton tests
- L NO cross-crate dev-dependencies in functional crates
-  CLEAN UP all test artifacts
-  SERIALIZE tests with global state using `#[serial]`

**File Management**:
- L NO saving files to project directory (use `/tmp`)
- L NO relative paths with more than one level up
-  DELETE `/tmp` files when done

**Documentation**:
- L NO outdated documentation after code changes
-  UPDATE documentation with code changes

**Commits**:
- L NO commits without user instruction
- L NO batch commits without confirmation
-  SPLIT commits by logical purpose
-  KEEP commits small and focused

---

## License

By contributing to Reinhardt, you agree that your contributions will be licensed under both the MIT License and Apache License 2.0.

---

Thank you for contributing to Reinhardt! >�
