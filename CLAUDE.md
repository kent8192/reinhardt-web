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

### FM-4 (MUST): Relative Path Restrictions

- **DO NOT USE** relative path references that go up more than one level (e.g., `../..`)
- **PREFER** absolute paths or single-level relative paths (e.g., `../`)
- Deep relative paths make code harder to understand and maintain
- Examples:
  - ❌ Bad: `../../config/settings.toml`
  - ❌ Bad: `../../../data/file.json`
  - ✅ Good: `../sibling_crate/module.rs`
  - ✅ Good: Use absolute paths or workspace-relative paths instead

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

### CS-3 (SHOULD): String Conversion Optimization

- **MINIMIZE** the use of `.to_string()` calls
- Prefer borrowing and string slices where possible
- Use `String::from()`, `format!()`, or other alternatives when appropriate
- Consider using `Cow<str>` for conditional ownership

### CS-4 (MUST): Code Cleanup

- **IMMEDIATELY** delete obsolete code once it is no longer needed
- Do not leave commented-out code or unused functions
- Remove deprecated functionality promptly

### CS-5 (MUST): Deletion Records

- **NEVER** leave comments documenting deleted code, tests, or files in the codebase
- **DO NOT** create "Removed empty test" or "Deleted file" comments
- Git history serves as the permanent record of deletions
- If documentation is needed, extract to `docs/IMPLEMENTATION_NOTES.md` instead
- Examples:
  - ❌ Bad: `// Removed empty test: test_foo - This test was empty`
  - ❌ Bad: `// Deleted: old_module.rs (deprecated)`
  - ✅ Good: Simply delete without comments
  - ✅ Good: Extract important notes to docs/IMPLEMENTATION_NOTES.md

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

#### TI-2 (MUST): Unimplemented Feature Notation

- **MUST** use one of the following for unimplemented features:
  - `// TODO:` comment with explanation
  - `todo!()` macro for features planned to be implemented in the future
  - `unimplemented!()` macro for features intentionally not implemented (will never be implemented)
- **NEVER** use alternative notations like:
  - ❌ Bad: `// Implementation Note:`
  - ❌ Bad: `// FIXME:`
  - ❌ Bad: `// NOTE: Not implemented yet`
  - ❌ Bad: Custom placeholder comments
- **Macro Selection Guidelines**:
  - Use `todo!()` for features that **WILL** be implemented later
  - Use `unimplemented!()` for features that **WILL NOT** be implemented (intentionally omitted)
  - Use `// TODO:` comments for planning without runtime panics
  - **DELETE** `todo!()` and `// TODO:` comments when the functionality is implemented
  - **KEEP** `unimplemented!()` for permanently excluded features
- **TODO Comment Guidelines**:
  - Explain what needs to be implemented and why it's pending
  - **DELETE** when the functionality is implemented
- **NOTE Comments**: Use `// NOTE:` for informational comments to users ONLY
  - **DO NOT** use NOTE comments for unimplemented features
  - Use TODO comments or `todo!()`/`unimplemented!()` macro instead
- **User-Facing Placeholders**:
  - **NEVER** use TODO or NOTE comments in user-facing code
  - Provide actual implementations or use `todo!()`/`unimplemented!()` macro for compile-time errors
- **Placeholder/Stub/Mock Implementation Rules**:
  - **ALL** placeholder implementations (excluding tests and documentation) **MUST** be marked with `todo!()` macro or `// TODO:` comment
  - This includes:
    - Empty function bodies that return default values (e.g., `Vec::new()`, `String::new()`, `Ok(())`)
    - Stub implementations with minimal logic that don't provide real functionality
    - Mock implementations intended to be replaced later
    - Temporary workarounds marked for future improvement
  - **Exception**: Tests and documentation examples may use simplified implementations without `todo!()` markers
  - ❌ Bad: Unmarked placeholder implementation
    ```rust
    pub fn get_cache_config() -> CacheConfig {
        CacheConfig::default()  // No marker - looks like production code
    }
    ```
  - ✅ Good: Marked placeholder implementation
    ```rust
    pub fn get_cache_config() -> CacheConfig {
        todo!("Implement cache configuration loading from settings")
    }
    ```
  - ✅ Good: Alternative with TODO comment
    ```rust
    pub fn get_cache_config() -> CacheConfig {
        // TODO: Load from settings file instead of using default
        CacheConfig::default()
    }
    ```
- **Examples**:
  ```rust
  // ✅ Good: Clear TODO comment (planning)
  // TODO: Implement caching mechanism for frequently accessed data

  // ✅ Good: Using todo!() macro (will be implemented)
  fn validate_input(data: &str) -> Result<()> {
      todo!("Add input validation logic - planned for next sprint")
  }

  // ✅ Good: Using unimplemented!() macro (intentionally not implemented)
  fn legacy_api_endpoint() -> String {
      unimplemented!("This legacy API is intentionally not supported in Rust version")
  }

  // ✅ Good: Another unimplemented!() example (never will be implemented)
  fn windows_only_feature() -> Result<()> {
      #[cfg(not(target_os = "windows"))]
      unimplemented!("This feature is only available on Windows");

      #[cfg(target_os = "windows")]
      Ok(())
  }

  // ✅ Good: Marked stub returning default value
  fn get_database_pool() -> Pool {
      todo!("Initialize actual database connection pool")
  }

  // ✅ Good: Marked mock with minimal logic
  fn send_email(recipient: &str, body: &str) -> Result<()> {
      // TODO: Integrate with actual email service provider
      println!("Would send email to {}: {}", recipient, body);
      Ok(())
  }

  // ❌ Bad: Custom notation
  // Implementation Note: This needs to be completed

  // ❌ Bad: Using NOTE for unimplemented features
  // NOTE: Not implemented yet

  // ❌ Bad: Using unimplemented!() for future work
  fn upcoming_feature() -> String {
      unimplemented!("Will implement next week")  // Use todo!() instead!
  }

  // ❌ Bad: Unmarked placeholder that looks production-ready
  fn calculate_metrics() -> Metrics {
      Metrics::default()  // No indication this is temporary!
  }
  ```

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

## Database Operations

### DB-1 (MUST): Layer Selection

- **Basic CRUD Operations**: Use `reinhardt-orm` for table-level data operations (Create, Read, Update, Delete)
- **Low-Level Operations**: Use `reinhardt-database` for schema management, raw queries, and database-specific operations
- Choose the appropriate abstraction level based on the task requirements

---

## Documentation Maintenance

### DM-1 (MUST): Documentation Updates with Code Changes

- **ALWAYS** update relevant documentation when implementing or modifying features
- Documentation updates **MUST** be done in the same workflow as the code changes
- **DO NOT** leave documentation outdated after code modifications

### DM-2 (MUST): Documentation Locations

When modifying features, check and update the following documentation as applicable:

- **README.md**: Project-level overview and features (English)
- **README.ja.md**: Project-level overview and features (Japanese)
- **Crate README.md**: Individual crate documentation (English)
- **Crate README.ja.md**: Individual crate documentation (Japanese)
- **docs/ directory**: Detailed guides, tutorials, and API documentation
  - `docs/GETTING_STARTED.md`: Getting started guide
  - `docs/FEATURE_FLAGS.md`: Feature flags documentation
  - `docs/tutorials/`: Tutorial files
  - Other relevant documentation files

### DM-3 (MUST): Documentation Synchronization

- Keep English and Japanese documentation synchronized
- When updating README.md, update README.ja.md as well
- Ensure consistency across all documentation levels (project, crate, docs/)

### DM-4 (SHOULD): Documentation Scope

Update documentation for:

- New features: Add feature descriptions, usage examples, and API references
- Modified features: Update affected sections to reflect changes
- Deprecated features: Mark as deprecated and provide migration guides
- Removed features: Remove documentation and add migration notes if necessary
- API changes: Update function signatures, parameters, and return types

### DM-5 (MUST): Documentation Quality

- Ensure examples in documentation are tested and working
- Update code snippets to reflect current API
- Verify that all links and references are valid
- Maintain consistency in terminology and formatting

---

## Workflow

### W-1 (SHOULD): Iterative Development

- Run tests frequently during development
- Fix failing tests immediately
- Ensure all tests pass before committing

### W-2 (SHOULD): Test-Driven Development

- Consider writing tests before implementation when appropriate
- This helps clarify requirements and edge cases

### W-3 (MUST): Git Commit Policy

For detailed commit guidelines including message format, granularity, and execution policy, refer to:
@CLAUDE.commit.md

**Summary:**

- **NEVER** create commits without explicit user instruction
- **NEVER** push commits without explicit user instruction
- Always wait for user confirmation before committing changes
- Prepare changes and inform the user, but let them decide when to commit

### W-4 (MUST): Batch Script Safety

- When using `sed` or bash scripts for bulk replacements:
  1. **ALWAYS** create a dry-run script first to preview changes
  2. Execute the dry-run and verify the scope of changes
  3. **ONLY** proceed with actual replacement after confirming appropriateness
- Never perform bulk changes without verification

### W-5 (SHOULD): Parallel Task Execution

- When tasks involve editing independent files:
  - **USE multiple agents in parallel** to implement changes concurrently
  - This improves efficiency and reduces overall completion time
- Ensure tasks are truly independent before parallelizing

---

## Additional Instructions

@CLAUDE.local.md

---

## Quick Reference

**Critical Rules Summary:**

### Code & Module System

- ❌ NO `mod.rs` files
- ❌ NO TODO/NOTE comments in user-facing placeholders
- ❌ NO unmarked placeholder/stub/mock implementations
- ❌ NO keeping obsolete code
- ❌ NO excessive `.to_string()` calls
- ❌ NO comments documenting deleted code/tests
- ❌ NO alternative notations like `Implementation Note:`, `FIXME:`, etc.
- ❌ NO using `unimplemented!()` for future work (use `todo!()` instead)
- ✅ USE 2024 edition module system
- ✅ MARK ALL placeholders/stubs/mocks with `todo!()` or `// TODO:` comment
- ✅ DELETE old code immediately
- ✅ USE `// TODO:` comments for planning
- ✅ USE `todo!()` for features that WILL be implemented
- ✅ USE `unimplemented!()` for features that WILL NOT be implemented (intentionally omitted)
- ✅ USE `// NOTE:` for informational comments only
- ✅ EXTRACT important notes to docs/IMPLEMENTATION_NOTES.md

### Testing

- ❌ NO skeleton tests (tests must have real assertions)
- ❌ NO cross-crate dependencies for testing in functional crates
- ✅ CLEAN UP all test artifacts
- ✅ PLACE integration tests in `tests` crate only
- ✅ USE TestContainers for infrastructure tests

### File Management

- ❌ NO saving files to project directory (use `/tmp`)
- ❌ NO relative paths with more than one level up (e.g., `../..`)
- ✅ DELETE `/tmp` files when done
- ✅ USE absolute paths or single-level relative paths

### Documentation

- ❌ NO outdated documentation after code changes
- ❌ NO unsynchronized English/Japanese documentation
- ✅ UPDATE documentation with code changes in the same workflow
- ✅ SYNCHRONIZE README.md and README.ja.md
- ✅ UPDATE all relevant crate and docs/ files
- ✅ VERIFY examples and code snippets are working

### Workflow

- ❌ NO commits without explicit user instruction
- ❌ NO bulk replacements without dry-run verification
- ✅ CREATE dry-run scripts for batch operations
- ✅ USE parallel agents for independent file edits

### Database

- ✅ USE `reinhardt-orm` for CRUD operations
- ✅ USE `reinhardt-database` for low-level operations
