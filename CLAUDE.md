# CLAUDE.md

## Purpose

This file contains project-specific instructions and conventions for the Reinhardt project. These rules ensure code quality, maintainability, and consistent testing practices across the Rust codebase.

## Project Overview

For details about the Reinhardt project, please refer to README.md.
@README.md

---

## Project Information

### Tech Stack

- **Language**: Rust (2024 Edition)
- **Module System**: **MUST** use 2024 edition module system (NO `mod.rs`)
- **Testing Framework**: Rust's built-in test framework with TestContainers for infrastructure tests

### Project Structure

- **Functional crates**: Individual feature implementations
- **Tests crate**: Integration tests only
- **Each crate**: Contains its own unit tests

---

## File Management Rules

### FM-1 (MUST): Temporary File Locations

- **NEVER** save execution results, logs, or temporary files to the project directory
- **ALWAYS** create temporary files in `/tmp` directory
- Examples: MD files, Python scripts, Shell scripts, automation files

### FM-2 (MUST): File Cleanup

- **IMMEDIATELY** delete files from `/tmp` directory once they are no longer needed
- Clean up after each task completion

### FM-3 (SHOULD): Large File Handling

- If a file is too large to read in one operation, split it into multiple parts for reading
- Use streaming or chunked reading approaches when appropriate

---

## Code Style & Conventions

### CS-1 (MUST): Module System

- **DO NOT USE `mod.rs`**
- **MUST USE 2024 EDITION MODULE SYSTEM** for all modules
- Use `mod module_name;` in `lib.rs` or `main.rs`

### CS-2 (SHOULD): Code Organization

- Keep related functionality together
- Follow Rust naming conventions (snake_case for functions, PascalCase for types)
- Maintain clear separation of concerns

---

## Testing Best Practices

### Testing Philosophy

#### TP-1 (MUST): Test Completeness

- **NO skeleton implementations**: All tests MUST contain meaningful assertions
- A skeleton implementation is a test that always passes (e.g., empty test, `assert!(true)`)
- Tests MUST be capable of failing when the code is incorrect
- Documentation tests must be performed for all features you implement.
- Do not implement test cases that are identical to documentation tests as unit tests or integration tests.

#### TP-2 (MUST): Reinhardt Crate Usage

- **EVERY** test case MUST use at least one component from the Reinhardt crate
- Components include the following: functions, variables, methods, structs, traits, commands, and all components present within the Reinhardt crate.

### Test Organization

#### TO-1 (MUST): Unit vs Integration Tests

- **Unit tests**: Use exactly ONE Reinhardt crate
  - **MUST** be placed within the functional crate being tested
  - Test that specific crate's functionality in isolation
- **Integration tests**: Use TWO or MORE Reinhardt crates
  - **MUST** be placed in the `tests` crate
  - Test interactions between multiple crates

#### TO-2 (MUST): Dependency Rules

- Functional crates **MUST NOT** include other Reinhardt crates as `dev-dependencies`
- This ensures unit tests remain isolated
- Feature dependencies in `[dependencies]` are acceptable

### Test Implementation

#### TI-1 (SHOULD): TODO Comments

- If tests cannot be fully implemented, leave a `// TODO:` comment explaining why
- **DELETE** the TODO comment when the test is implemented

#### TI-2 (SHOULD): Note Comments

- Leave a `// NOTE:` comment for incomplete implementations
- Explain what is missing and why
- **DELETE** the NOTE comment when fully implemented

#### TI-3 (MUST): Test Cleanup

- **ALL** files, directories, or environmental changes created during tests **MUST** be deleted upon test completion
- Use test fixtures, `Drop` implementations, or explicit cleanup in test teardown
- Leave no artifacts after test execution

#### TI-4 (MUST): Global State Management

- **Tests that modify global state MUST be serialized** using the `serial_test` crate
- Use named serial groups: `#[serial(group_name)]` to serialize only related tests
- Common serial groups:
  - `#[serial(i18n)]` - for tests modifying translation state
  - `#[serial(url_overrides)]` - for tests modifying URL override registry
  - Create new groups as needed for other global state
- **ALWAYS** add `serial_test = { workspace = true }` to `[dev-dependencies]` when using
- **ALWAYS** call cleanup functions (e.g., `deactivate()`, `clear_url_overrides()`) in test teardown
- Example:
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

### Infrastructure Testing

#### IT-1 (SHOULD): TestContainers for Infrastructure

- Use **TestContainers** for tests requiring actual infrastructure (databases, message queues, etc.)
- This allows for longer test execution times
- Prefer real infrastructure over mocks when feasible

---

## Workflow

### W-1 (SHOULD): Iterative Development

- Run tests frequently during development
- Fix failing tests immediately
- Ensure all tests pass before committing

### W-2 (SHOULD): Test-Driven Development

- Consider writing tests before implementation when appropriate
- This helps clarify requirements and edge cases

---

## Additional Instructions

@CLAUDE.local.md

---

## Quick Reference

**Critical Rules Summary:**

- ❌ NO `mod.rs` files
- ❌ NO skeleton tests (tests must have real assertions)
- ❌ NO saving files to project directory (use `/tmp`)
- ❌ NO cross-crate dependencies for testing in functional crates
- ✅ USE 2024 edition module system
- ✅ CLEAN UP all test artifacts
- ✅ PLACE integration tests in `tests` crate only
- ✅ USE TestContainers for infrastructure tests
