# Testing Standards

## Purpose

This document defines comprehensive testing standards for the Reinhardt project, ensuring high-quality, maintainable test coverage.

---

## Testing Philosophy

### TP-1 (MUST): Test Completeness

**NO skeleton implementations** - All tests MUST contain meaningful assertions.

**Definition of Skeleton Test:**
- A test that always passes (e.g., empty test, `assert!(true)`)
- Tests without any assertions
- Tests that don't actually verify behavior

**Requirements:**
- Tests MUST be capable of failing when the code is incorrect
- Documentation tests must be performed for all features you implement
- Do not implement test cases that are identical to documentation tests as unit tests or integration tests

**Examples:**

❌ **BAD - Skeleton Tests:**
```rust
#[test]
fn test_user_creation() {
    // Empty test - always passes
}

#[test]
fn test_validation() {
    let result = validate_email("test@example.com");
    // No assertion - useless test
}

#[test]
fn test_always_passes() {
    assert!(true);  // Meaningless assertion
}
```

✅ **GOOD - Meaningful Tests:**
```rust
#[test]
fn test_user_creation() {
    let user = User::new("Alice", "alice@example.com");
    assert_eq!(user.name, "Alice");
    assert_eq!(user.email, "alice@example.com");
}

#[test]
fn test_validation() {
    assert!(validate_email("test@example.com").is_ok());
    assert!(validate_email("invalid").is_err());
}
```

### TP-2 (MUST): Reinhardt Crate Usage

**EVERY** test case MUST use at least one component from the Reinhardt crate.

**Reinhardt Components Include:**
- Functions, variables, methods
- Structs, traits, enums
- Commands, macros
- All components present within the Reinhardt crate

**Why?** This ensures tests actually verify Reinhardt functionality rather than testing third-party libraries or standard library behavior.

---

## Test Organization

### TO-1 (MUST): Unit vs Integration Tests

Clear separation based on crate dependencies:

#### Unit Tests
**Definition:** Tests using exactly **ONE** Reinhardt crate

**Location:** Within the functional crate being tested

**Structure:**
```
crates/reinhardt-orm/
├── src/
│   ├── lib.rs
│   ├── query.rs
│   └── model.rs
└── tests/           // ❌ NO integration tests here
    └── ...

// Unit tests in the same file
// src/query.rs
#[cfg(test)]
mod tests {
    use super::*;  // Only using reinhardt-orm

    #[test]
    fn test_query_builder() {
        let query = QueryBuilder::new()
            .table("users")
            .build();
        assert_eq!(query.to_sql(), "SELECT * FROM users");
    }
}
```

#### Integration Tests
**Definition:** Tests using **TWO or MORE** Reinhardt crates

**Location:** MUST be placed in the `tests` crate at repository root

**Structure:**
```
tests/                    // Integration tests crate
├── Cargo.toml           // Dependencies on multiple Reinhardt crates
└── integration/
    └── tests/
        └── orm_serializer_integration.rs
```

**Example:**
```rust
// tests/integration/tests/orm_serializer_integration.rs
use reinhardt_orm::Model;          // Crate 1
use reinhardt_serializers::Serialize;  // Crate 2

#[test]
fn test_model_serialization() {
    let user = User { id: 1, name: "Alice".to_string() };
    let json = user.serialize();
    assert_eq!(json, r#"{"id":1,"name":"Alice"}"#);
}
```

### TO-2 (MUST): Dependency Rules

**Functional crates MUST NOT include other Reinhardt crates as `dev-dependencies`**

**Why?** This ensures unit tests remain isolated and test only the crate's own functionality.

❌ **BAD:**
```toml
# crates/reinhardt-orm/Cargo.toml
[dev-dependencies]
reinhardt-serializers = { path = "../reinhardt-serializers" }  # ❌ NEVER do this
```

✅ **GOOD:**
```toml
# crates/reinhardt-orm/Cargo.toml
[dependencies]
reinhardt-types = { path = "../reinhardt-types" }  # ✅ Feature dependencies OK

[dev-dependencies]
tokio = { version = "1.0", features = ["rt", "macros"] }  # ✅ External deps OK
```

```toml
# tests/Cargo.toml (integration tests)
[dependencies]
reinhardt-orm = { path = "../crates/reinhardt-orm" }           # ✅ Multiple crates OK
reinhardt-serializers = { path = "../crates/reinhardt-serializers" }
```

---

## Test Implementation

### TI-1 (SHOULD): TODO Comments

If tests cannot be fully implemented, leave a `// TODO:` comment explaining why.

**DELETE** the TODO comment when the test is implemented.

**Example:**
```rust
#[test]
fn test_complex_query() {
    // TODO: Implement after adding support for JOIN operations
    todo!("Waiting for JOIN support in query builder")
}
```

### TI-2 (MUST): Unimplemented Feature Notation

For unimplemented features, use one of the following:

#### Option 1: `todo!()` macro
Use for features that **WILL** be implemented later

```rust
fn validate_advanced_rules(data: &str) -> Result<()> {
    todo!("Add advanced validation logic - planned for next sprint")
}
```

#### Option 2: `unimplemented!()` macro
Use for features that **WILL NOT** be implemented (intentionally omitted)

```rust
fn legacy_api_endpoint() -> String {
    unimplemented!("This legacy API is intentionally not supported in Rust version")
}

#[cfg(not(target_os = "windows"))]
fn windows_only_feature() -> Result<()> {
    unimplemented!("This feature is only available on Windows");
}
```

#### Option 3: `// TODO:` comment
Use for planning without runtime panics

```rust
// TODO: Implement caching mechanism for frequently accessed data
fn get_cached_data() -> Vec<Data> {
    Vec::new()  // Temporary implementation
}
```

**Macro Selection Guidelines:**
- `todo!()` → Features that WILL be implemented
- `unimplemented!()` → Features that WILL NOT be implemented
- `// TODO:` → Planning notes

**DELETE `todo!()` and `// TODO:` when implemented**
**KEEP `unimplemented!()` for permanently excluded features**

#### Placeholder/Stub/Mock Implementation Rules

**ALL** placeholder implementations (excluding tests and documentation) **MUST** be marked with `todo!()` macro or `// TODO:` comment.

**Applies to:**
- Empty function bodies returning default values
- Stub implementations with minimal logic
- Mock implementations intended to be replaced
- Temporary workarounds

❌ **BAD - Unmarked Placeholder:**
```rust
pub fn get_cache_config() -> CacheConfig {
    CacheConfig::default()  // ❌ Looks like production code!
}
```

✅ **GOOD - Marked Placeholder:**
```rust
pub fn get_cache_config() -> CacheConfig {
    todo!("Implement cache configuration loading from settings")
}

// OR

pub fn get_cache_config() -> CacheConfig {
    // TODO: Load from settings file instead of using default
    CacheConfig::default()
}
```

### TI-3 (MUST): Test Cleanup

**ALL** files, directories, or environmental changes created during tests **MUST** be deleted upon test completion.

**Techniques:**
- Test fixtures
- `Drop` implementations
- Explicit cleanup in test teardown
- `tempfile` crate for temporary files

**Example:**
```rust
#[test]
fn test_file_operations() {
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    // Test code that creates files
    std::fs::write(&file_path, "test data").unwrap();

    // Cleanup happens automatically when temp_dir drops
}

#[test]
fn test_database_operations() {
    let db = setup_test_database();

    // Test code
    db.insert_test_data();

    // Explicit cleanup
    db.clear_all_data();
    db.close();
}
```

### TI-4 (MUST): Global State Management

Tests that modify global state MUST be serialized using the `serial_test` crate.

#### Using Serial Test Groups

Use named serial groups: `#[serial(group_name)]` to serialize only related tests.

**Common Serial Groups:**
- `#[serial(i18n)]` - For tests modifying translation state
- `#[serial(url_overrides)]` - For tests modifying URL override registry
- Create new groups as needed for other global state

**Setup:**
```toml
# Cargo.toml
[dev-dependencies]
serial_test = { workspace = true }
```

**Example:**
```rust
use serial_test::serial;

#[test]
#[serial(i18n)]
fn test_translation_activation() {
    activate("fr", catalog);
    assert_eq!(get_language(), "fr");
    deactivate();  // ✅ Cleanup
}

#[test]
#[serial(i18n)]
fn test_translation_fallback() {
    activate("es", catalog);
    assert_eq!(get_language(), "es");
    deactivate();  // ✅ Cleanup
}

#[test]
#[serial(url_overrides)]
fn test_url_override() {
    set_url_override("home", "/custom-home");
    assert_eq!(resolve_url("home"), "/custom-home");
    clear_url_overrides();  // ✅ Cleanup
}
```

**ALWAYS** call cleanup functions (e.g., `deactivate()`, `clear_url_overrides()`) in test teardown.

---

## Infrastructure Testing

### IT-1 (SHOULD): TestContainers for Infrastructure

Use **TestContainers** for tests requiring actual infrastructure:
- Databases (PostgreSQL, MySQL, SQLite)
- Message queues (Redis, RabbitMQ)
- Cache systems (Memcached, Redis)

**Benefits:**
- Tests use real infrastructure, not mocks
- Longer test execution times are acceptable
- More confidence in production behavior

**Example:**
```rust
use testcontainers::{clients, images};

#[tokio::test]
async fn test_database_integration() {
    let docker = clients::Cli::default();
    let postgres = docker.run(images::postgres::Postgres::default());
    let port = postgres.get_host_port_ipv4(5432);

    let database_url = format!("postgres://postgres@localhost:{}/postgres", port);
    let pool = create_pool(&database_url).await;

    // Test with real database
    let result = pool.execute("SELECT 1").await;
    assert!(result.is_ok());
}
```

---

## Quick Reference

### Testing Checklist

Before submitting a PR with tests:

- [ ] All tests have meaningful assertions (no skeletons)
- [ ] Each test uses at least one Reinhardt component
- [ ] Unit tests (1 crate) are in functional crate
- [ ] Integration tests (2+ crates) are in `tests/` crate
- [ ] No Reinhardt crates in functional crate `dev-dependencies`
- [ ] Placeholders marked with `todo!()` or `// TODO:`
- [ ] All test artifacts cleaned up
- [ ] Global state tests use `#[serial(group_name)]`
- [ ] Cleanup functions called in serial tests
- [ ] Infrastructure tests use TestContainers where appropriate

### Common Test Patterns

#### Pattern: Basic Unit Test
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_functionality() {
        let result = my_function(input);
        assert_eq!(result, expected);
    }
}
```

#### Pattern: Async Test
```rust
#[tokio::test]
async fn test_async_operation() {
    let result = async_function().await;
    assert!(result.is_ok());
}
```

#### Pattern: Serial Test with Cleanup
```rust
#[test]
#[serial(state_group)]
fn test_with_global_state() {
    setup_state();
    // Test code
    cleanup_state();  // ✅ Always cleanup
}
```

#### Pattern: TestContainers
```rust
#[tokio::test]
async fn test_with_infrastructure() {
    let docker = clients::Cli::default();
    let container = docker.run(images::postgres::Postgres::default());
    // Test with real infrastructure
}
```

---

## Related Documentation

- Main standards: @CLAUDE.md
- Anti-patterns: @docs/ANTI_PATTERNS.md
- Module system: @docs/MODULE_SYSTEM.md
