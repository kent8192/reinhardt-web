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

Clear separation based on the nature of what is being tested:

#### Unit Tests
**Definition:** Tests that verify the behavior of a **single component**

**Component:** A single function, method, struct, trait, enum, or closely related group of items that serve a unified purpose.

**Clarification:**
- ✅ Single component: A `QueryBuilder` struct with its methods
- ✅ Single component: A `redirect()` function
- ✅ Single component: A `MessageCatalog` struct
- ❌ Multiple components: `QueryBuilder` + `Connection` (these are separate components)
- ❌ Multiple components: `FilterBackend` + `ORM` (integration across components)
- ❌ Multiple components: Different crates (always cross-crate integration)

**Note:** A module may contain multiple components. Testing how these components interact is integration testing, not unit testing.

**Location:** Within the functional crate being tested

**Characteristics:**
- Tests a component in isolation
- Verifies the component's behavior and edge cases
- Does not test interactions between multiple components

**Structure:**
```
crates/reinhardt-orm/
├── src/
│   ├── lib.rs
│   ├── query.rs
│   └── model.rs
└── tests/
    └── unit_tests.rs

// Unit tests in the same file
// src/query.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_builder_table_name() {
        // Testing a single component's internal behavior
        let query = QueryBuilder::new()
            .table("users")
            .build();
        assert_eq!(query.to_sql(), "SELECT * FROM users");
    }
}
```

#### Integration Tests
**Definition:** Tests that verify the **integration points** (interfaces) between **two or more components**

**Integration Point:** The interface, interaction, or data exchange between components

**Location:**
- **Cross-crate integration:** MUST be placed in the `tests` crate at repository root
- **Within-crate integration:** Can be placed in the functional crate

**Characteristics:**
- Tests how components work together
- Verifies data flow and communication between components
- Focuses on interfaces and interactions

**Structure:**
```
tests/                    // Cross-crate integration tests
├── Cargo.toml           // Dependencies on multiple Reinhardt crates
└── integration/
    └── tests/
        └── orm_serializer_integration.rs

crates/reinhardt-orm/
└── tests/
    └── integration_tests.rs  // Within-crate integration (if needed)
```

**Example (Cross-crate integration):**
```rust
// tests/integration/tests/orm_serializer_integration.rs
use reinhardt_orm::Model;
use reinhardt_serializers::Serialize;

#[test]
fn test_model_serialization() {
    // Testing the integration between ORM and Serializer components
    let user = User { id: 1, name: "Alice".to_string() };
    let json = user.serialize();
    assert_eq!(json, r#"{"id":1,"name":"Alice"}"#);
}
```

**Example (Within-crate integration):**
```rust
// crates/reinhardt-orm/tests/integration_tests.rs
use reinhardt_orm::{QueryBuilder, Connection};

#[test]
fn test_query_execution() {
    // Testing the integration between QueryBuilder and Connection components
    let conn = Connection::new_in_memory();
    let query = QueryBuilder::new().table("users").build();
    let result = conn.execute(query);
    assert!(result.is_ok());
}
```

#### How to Determine Test Type

**Ask these questions:**

1. **How many Reinhardt crates does this test import?**
   - 1 crate → Unit or within-crate integration
   - 2+ crates → Cross-crate integration (→ `tests/` at repo root)

2. **How many distinct components does this test verify?**
   - 1 component → Unit test (→ inline `#[cfg(test)]` or `crate/tests/`)
   - 2+ components within same crate → Within-crate integration (→ `crate/tests/`)
   - 2+ components across crates → Cross-crate integration (→ `tests/` at repo root)

3. **What is the test verifying?**
   - Internal behavior of a single component → Unit test
   - Interface/interaction between components → Integration test

**Examples:**

✅ **Cross-crate integration** (→ `tests/integration/tests/`):
```rust
// Imports from multiple Reinhardt crates
use reinhardt_filters::SimpleSearchBackend;
use reinhardt_orm::QueryBuilder;

#[test]
fn test_filter_with_orm() {
    // Tests integration between filters and ORM
    let backend = SimpleSearchBackend::new("search");
    let query = QueryBuilder::new().table("users");
    // Test how filter modifies ORM query
}
```

✅ **Within-crate integration** (→ `crates/reinhardt-server/tests/`):
```rust
// Imports from same crate only
use reinhardt_server::{HttpServer, ShutdownCoordinator};

#[test]
fn test_server_lifecycle() {
    // Tests integration between server components
    let server = HttpServer::new();
    let coordinator = ShutdownCoordinator::new();
    // Test how they work together
}
```

✅ **Unit test** (→ `src/redirect.rs` with `#[cfg(test)]`):
```rust
// Tests single function
#[test]
fn test_redirect_status_code() {
    let response = redirect("/path");
    assert_eq!(response.status, 302);
}
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

#### Using TeardownGuard for Automatic Cleanup

For guaranteed cleanup even when tests panic, use `TeardownGuard` from `reinhardt-test`:

**Benefits:**
- Cleanup is guaranteed via RAII (Drop trait)
- Works even if test panics or fails assertions
- Reduces boilerplate code

**Setup:**
```rust
use reinhardt_test::resource::{TestResource, TeardownGuard};
use rstest::*;
use serial_test::serial;

/// Guard for global registry cleanup
struct RegistryGuard;

impl TestResource for RegistryGuard {
    fn setup() -> Self {
        // Clear before test
        GLOBAL_REGISTRY.clear();
        Self
    }

    fn teardown(&mut self) {
        // Clear after test (guaranteed even on panic)
        GLOBAL_REGISTRY.clear();
    }
}

#[fixture]
fn registry_guard() -> TeardownGuard<RegistryGuard> {
    TeardownGuard::new()
}
```

**Usage in tests:**
```rust
#[rstest]
#[serial(registry)]
#[tokio::test]
async fn test_with_global_state(
    _registry_guard: TeardownGuard<RegistryGuard>,
) {
    // Test code that modifies GLOBAL_REGISTRY
    GLOBAL_REGISTRY.insert("key", "value");

    // No manual cleanup needed - TeardownGuard handles it
}
```

**When to use TeardownGuard:**
- ✅ Tests that modify global static variables
- ✅ Tests that need guaranteed cleanup on panic
- ✅ Tests with complex setup/teardown logic
- ❌ Tests with no global state (use regular fixtures)
- ❌ Tests with RAII resources (TestContainers, temp dirs)

### TI-5 (MUST): Assertion Strictness

**Use strict assertions with exact value comparisons instead of loose pattern matching.**

Assertions MUST use the most strict and precise verification method available:

**Preferred Methods:**
- `assert_eq!(actual, expected)` - For exact value equality
- `assert_ne!(actual, unexpected)` - For exact value inequality
- `assert!(matches!(value, Pattern))` - For pattern matching with specific variants

**Avoid Loose Assertions:**
- ❌ `assert!(string.contains("substring"))` - Too permissive, may match unintended content
- ❌ `assert!(result.is_ok())` without checking the contained value
- ❌ `assert!(value > 0)` when you know the exact expected value

**Exception:**
Loose assertions are acceptable ONLY when strict assertions are impossible or impractical:
- Random values (e.g., UUIDs, timestamps, random numbers)
- System-dependent values (e.g., process IDs, file system paths)
- Non-deterministic operations (e.g., async race conditions with bounded outcomes)

**Examples:**

❌ **BAD - Loose Assertions:**
```rust
#[test]
fn test_error_message() {
    let result = validate_input("");
    let error = result.unwrap_err();
    // ❌ Too permissive - could match unintended substrings
    assert!(error.to_string().contains("invalid"));
}

#[test]
fn test_generate_id() {
    let id = generate_id();
    // ❌ Doesn't verify the actual format or value
    assert!(id.len() > 0);
}

#[test]
fn test_calculation() {
    let result = calculate_discount(100, 10);
    // ❌ Should check exact value, not just range
    assert!(result > 0);
    assert!(result < 100);
}
```

✅ **GOOD - Strict Assertions:**
```rust
#[test]
fn test_error_message() {
    let result = validate_input("");
    let error = result.unwrap_err();
    // ✅ Exact error message verification
    assert_eq!(error.to_string(), "Input cannot be empty");
}

#[test]
fn test_generate_id() {
    let id = generate_sequential_id();
    // ✅ Exact value verification for deterministic IDs
    assert_eq!(id, 1);
}

#[test]
fn test_calculation() {
    let result = calculate_discount(100, 10);
    // ✅ Exact value expected
    assert_eq!(result, 90);
}
```

✅ **GOOD - Acceptable Loose Assertions (Justified Cases):**
```rust
#[test]
fn test_generate_uuid() {
    let uuid = generate_uuid();
    // ✅ UUID is random, can only check format
    // NOTE: UUID value is cryptographically random, exact match impossible
    assert!(uuid.len() == 36);
    assert!(uuid.chars().filter(|&c| c == '-').count() == 4);
}

#[test]
fn test_timestamp_generation() {
    let before = SystemTime::now();
    let timestamp = get_current_timestamp();
    let after = SystemTime::now();
    // ✅ Timestamp is system-dependent, can only check range
    // NOTE: System clock resolution makes exact matching impractical
    assert!(timestamp >= before);
    assert!(timestamp <= after);
}

#[test]
fn test_random_selection() {
    let choices = vec!["a", "b", "c"];
    let selected = random_choice(&choices);
    // ✅ Random result, can only verify it's in the set
    // NOTE: Selection is non-deterministic by design
    assert!(choices.contains(&selected));
}

#[test]
fn test_csrf_token_in_cookie() {
    let cookie = generate_csrf_cookie();
    // ✅ CSRF token is cryptographically random
    // NOTE: Token value cannot be predicted, only format verified
    assert!(cookie.to_str().unwrap().contains("csrftoken="));
    assert_eq!(cookie.to_str().unwrap().split('=').count(), 2);
}

#[test]
fn test_sql_where_clause_with_multiple_conditions() {
    let sql = build_query_with_filters(&[("age", ">=18"), ("active", "true")]);
    // ✅ SQL clause order is not guaranteed by query builder
    // NOTE: Query optimizer may reorder clauses, verify presence not order
    assert!(sql.contains("age >= 18"));
    assert!(sql.contains("active = true"));
}

#[test]
fn test_counter_incremented_by_concurrent_threads() {
    let counter = Arc::new(AtomicCounter::new());
    // Spawn threads that increment counter
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let c = counter.clone();
            thread::spawn(move || c.increment())
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    // ✅ Exact value depends on thread scheduling
    // NOTE: Thread interleaving is non-deterministic, verify bounds only
    assert!(counter.get() > 0);
    assert!(counter.get() <= 10);
}
```

**Justification Requirement:**

When using loose assertions, add a comment explaining why strict assertions are not possible:

```rust
#[test]
fn test_concurrent_counter() {
    let counter = AtomicCounter::new();
    // Increment from multiple threads
    // ...

    // NOTE: Using range assertion because exact value depends on thread scheduling
    // which is non-deterministic. We verify the counter incremented at least once.
    assert!(counter.get() > 0);
    assert!(counter.get() <= expected_max);
}
```

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

## rstest Best Practices

### TF-1 (SHOULD): rstest Fixture Pattern

Use **rstest** fixtures for reusable test setup and dependency injection.

#### Basic Fixture

```rust
use rstest::*;

#[fixture]
fn test_data() -> Vec<String> {
    vec!["item1".to_string(), "item2".to_string()]
}

#[rstest]
fn test_with_fixture(test_data: Vec<String>) {
    assert_eq!(test_data.len(), 2);
}
```

#### Async Fixture

For async fixtures, use `#[future]` attribute on the parameter:

```rust
#[fixture]
async fn postgres_fixture() -> (ContainerAsync<GenericImage>, Arc<AdminDatabase>) {
    // Setup PostgreSQL container and database
    // ...
}

#[rstest]
#[tokio::test]
async fn test_with_async_fixture(
    #[future] postgres_fixture: (ContainerAsync<GenericImage>, Arc<AdminDatabase>)
) {
    let (_container, db) = postgres_fixture.await;  // Don't forget .await!
    // Test code
}
```

**IMPORTANT**: Always include `#[future]` for async fixtures, and `.await` them in the test body.

#### Shared Fixtures

Define shared fixtures in `reinhardt-test/src/fixtures.rs` for use across multiple test files:

```rust
// In reinhardt-test/src/fixtures.rs
#[fixture]
pub async fn postgres_container() -> (ContainerAsync<Postgres>, String) {
    // ...
}

// In test file
use reinhardt_test::fixtures::*;

#[rstest]
#[tokio::test]
async fn test_with_shared_fixture(#[future] postgres_container: (ContainerAsync<Postgres>, String)) {
    let (_container, url) = postgres_container.await;
    // ...
}
```

---

### TF-2 (SHOULD): TestContainers with rstest

Combine rstest fixtures with TestContainers for database/cache testing:

#### PostgreSQL Example

```rust
#[fixture]
async fn postgres_db() -> (ContainerAsync<GenericImage>, Arc<AdminDatabase>) {
    let postgres = GenericImage::new("postgres", "16-alpine")
        .with_wait_for(WaitFor::message_on_stderr("database system is ready"))
        .with_env_var("POSTGRES_PASSWORD", "test")
        .start()
        .await
        .expect("Failed to start PostgreSQL");

    let port = postgres.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:test@localhost:{}/test_db", port);

    let conn = DatabaseConnection::connect(&url).await.unwrap();
    let admin_db = Arc::new(AdminDatabase::new(Arc::new(conn)));

    (postgres, admin_db)
}

#[rstest]
#[tokio::test]
async fn test_database_operations(
    #[future] postgres_db: (ContainerAsync<GenericImage>, Arc<AdminDatabase>)
) {
    let (_container, db) = postgres_db.await;

    // Test database operations
    let result = db.list::<User>("users", vec![], 0, 100).await;
    assert!(result.is_ok());

    // Container is automatically cleaned up when dropped
}
```

**Benefits:**
- Automatic container lifecycle management
- Isolation between tests
- Real infrastructure for higher confidence

---

### TF-3 (OPTIONAL): TeardownGuard Pattern

For tests that need guaranteed cleanup (especially on panic), use the TeardownGuard pattern:

#### When to Use

- Modifying global state (environment variables, singleton instances)
- Creating external resources (files, directories)
- Tests that MUST cleanup even if they panic

#### Implementation Example

```rust
struct TeardownGuard<F: FnOnce()> {
    cleanup: Option<F>,
}

impl<F: FnOnce()> TeardownGuard<F> {
    fn new(cleanup: F) -> Self {
        Self {
            cleanup: Some(cleanup),
        }
    }
}

impl<F: FnOnce()> Drop for TeardownGuard<F> {
    fn drop(&mut self) {
        if let Some(cleanup) = self.cleanup.take() {
            cleanup();
        }
    }
}

#[test]
fn test_with_guaranteed_cleanup() {
    // Set up global state
    std::env::set_var("TEST_VAR", "value");

    // Create guard to ensure cleanup
    let _guard = TeardownGuard::new(|| {
        std::env::remove_var("TEST_VAR");
    });

    // Test code - cleanup happens even if this panics
    assert_eq!(std::env::var("TEST_VAR").unwrap(), "value");

    // _guard is dropped here, cleanup() is called
}
```

**Note**: Most Reinhardt integration tests use TestContainers, which handle cleanup automatically, so TeardownGuard is rarely needed.

---

### TF-4: Common Pitfalls and Solutions

#### Pitfall 1: Forgetting `.await` on Async Fixtures

❌ **BAD:**
```rust
#[rstest]
#[tokio::test]
async fn test_bad(#[future] postgres_fixture: DbFixture) {
    let result = postgres_fixture.query(...);  // ❌ Missing .await
}
```

✅ **GOOD:**
```rust
#[rstest]
#[tokio::test]
async fn test_good(#[future] postgres_fixture: DbFixture) {
    let db = postgres_fixture.await;  // ✅ Correct
    let result = db.query(...);
}
```

#### Pitfall 2: Incorrect Data Structure Access

When working with database results, be aware of the actual structure returned by different methods:

❌ **BAD - Assuming nested structure when data is flat:**
```rust
let users = db.list::<User>("users", vec![], 0, 100).await?;
let username = users[0].get("data")
    .and_then(|data| data.get("username"));  // ❌ db.list() returns flat structure
```

✅ **GOOD - Access flat structure directly:**
```rust
let users = db.list::<User>("users", vec![], 0, 100).await?;
// db.list() returns: [{"id": 1, "username": "alice", ...}, ...]
let username = users[0].get("username")
    .and_then(|v| v.as_str());  // ✅ Direct access for flat structure
```

✅ **GOOD - Handle nested structure when appropriate:**
```rust
// For session data or other nested structures
let stored_data: serde_json::Value = result.get("data");
assert_eq!(stored_data["user_id"], user_id);  // ✅ Nested access where applicable
```

**When to use each pattern:**
- **Flat structure**: `db.list()`, `db.get()`, most ORM operations
- **Nested structure**: Session data, serialized JSON fields, specific API responses

#### Pitfall 3: Fixture Parameter Order

Fixture parameters must match the exact type signature, including `#[future]`:

❌ **BAD:**
```rust
#[fixture]
async fn my_fixture() -> (Container, Database) { ... }

#[rstest]
#[tokio::test]
async fn test(my_fixture: (Container, Database)) {  // ❌ Missing #[future]
    // ...
}
```

✅ **GOOD:**
```rust
#[rstest]
#[tokio::test]
async fn test(#[future] my_fixture: (Container, Database)) {  // ✅ Correct
    let (container, db) = my_fixture.await;
    // ...
}
```

#### Pitfall 4: Serial Tests Without Cleanup

Tests using `#[serial]` MUST clean up global state:

❌ **BAD:**
```rust
#[test]
#[serial(global_state)]
fn test_modifies_state() {
    set_global_state(42);
    assert_eq!(get_global_state(), 42);
    // ❌ No cleanup
}
```

✅ **GOOD:**
```rust
#[test]
#[serial(global_state)]
fn test_modifies_state() {
    set_global_state(42);
    assert_eq!(get_global_state(), 42);
    clear_global_state();  // ✅ Cleanup
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
- [ ] Placeholders marked with `todo!()` or `// TODO:`
- [ ] All test artifacts cleaned up
- [ ] Global state tests use `#[serial(group_name)]`
- [ ] Cleanup functions called in serial tests
- [ ] Assertions use strict value comparisons (`assert_eq!`) instead of loose matching (`contains`)
- [ ] Loose assertions are justified with comments when necessary
- [ ] Infrastructure tests use TestContainers where appropriate
- [ ] Async fixtures use `#[future]` attribute on parameters
- [ ] Async fixtures are `.await`ed in test body
- [ ] Fixture types match exactly including `#[future]` annotation
- [ ] Shared fixtures are defined in reinhardt-test/src/fixtures.rs when used across multiple files
- [ ] JSON nested structures are accessed correctly (e.g., `db.list()` returns `{"data": {...}}`)

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

#### Pattern: rstest Basic Fixture
```rust
use rstest::*;

#[fixture]
fn test_data() -> TestData {
    TestData::new("test")
}

#[rstest]
fn test_with_fixture(test_data: TestData) {
    assert_eq!(test_data.name, "test");
}
```

#### Pattern: rstest Async Fixture
```rust
use rstest::*;

#[fixture]
async fn database_pool() -> Arc<AnyPool> {
    let pool = AnyPool::connect("sqlite::memory:").await.unwrap();
    Arc::new(pool)
}

#[rstest]
#[tokio::test]
async fn test_with_async_fixture(#[future] database_pool: Arc<AnyPool>) {
    let pool = database_pool.await;
    // Use pool in test
}
```

#### Pattern: rstest with TeardownGuard
```rust
use rstest::*;
use reinhardt_test::resource::{TestResource, TeardownGuard};

struct RegistryGuard;

impl TestResource for RegistryGuard {
    fn setup() -> Self {
        GLOBAL_REGISTRY.clear();
        Self
    }
    fn teardown(&mut self) {
        GLOBAL_REGISTRY.clear();
    }
}

#[fixture]
fn registry_guard() -> TeardownGuard<RegistryGuard> {
    TeardownGuard::new()
}

#[rstest]
#[serial(registry)]
fn test_with_cleanup(_registry_guard: TeardownGuard<RegistryGuard>) {
    // Test code - cleanup guaranteed even on panic
}
```

#### Pattern: rstest Serial Test
```rust
use rstest::*;
use serial_test::serial;

#[fixture]
fn init_drivers() {
    sqlx::any::install_default_drivers();
}

#[rstest]
#[serial(database)]
#[tokio::test]
async fn test_serial_with_fixture(_init_drivers: ()) {
    // Test code that modifies global state
}
```

#### Pattern: rstest Parameterized Test
```rust
use rstest::*;

#[rstest]
#[case("valid@email.com", true)]
#[case("invalid-email", false)]
#[case("", false)]
fn test_email_validation(#[case] email: &str, #[case] expected: bool) {
    let result = validate_email(email);
    assert_eq!(result.is_ok(), expected);
}
```

---

## Related Documentation

- Main standards: @CLAUDE.md
- Anti-patterns: @docs/ANTI_PATTERNS.md
- Module system: @docs/MODULE_SYSTEM.md
