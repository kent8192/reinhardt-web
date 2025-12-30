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

### IT-2 (MUST): Prevent Flaky Tests with TestContainers

When using TestContainers for parallel test execution, follow these practices to prevent resource contention and flaky tests:

#### Problem: Resource Exhaustion During Parallel Execution

Parallel tests spawning multiple containers can exhaust:
- Docker daemon connection pool
- System file descriptors
- Memory resources
- Database connection pools

**Symptoms:**
- Tests pass individually but fail in full test suite
- Extremely long execution times (10x+ slower)
- Intermittent failures without code changes

#### Solution 1: Limit Parallel Execution (Recommended)

Create `.cargo/nextest.toml`:

```toml
[profile.default]
# Limit concurrent tests to prevent Docker resource exhaustion
max-tests-per-run = 8

# Increase timeout for database operations
slow-timeout = "60s"
timeout = "120s"

# Enable retries for flaky infrastructure tests
retries = { backoff = "exponential", max-retries = 2, seed = 12345 }

# Separate integration tests into dedicated worker threads
[profile.default.overrides]
filter = 'test(integration)'
threads-required = 1
max-threads = 4
```

#### Solution 2: Optimize Container Configuration

Configure PostgreSQL containers with higher resource limits:

```rust
let postgres = GenericImage::new("postgres", "17-alpine")
    .with_wait_for(WaitFor::message_on_stderr(
        "database system is ready to accept connections",
    ))
    .with_env_var("POSTGRES_HOST_AUTH_METHOD", "trust")
    .with_env_var("POSTGRES_INITDB_ARGS", "-c max_connections=200")
    .start()
    .await
    .expect("Failed to start PostgreSQL container");
```

#### Solution 3: Improve Connection Pool Settings

```rust
let pool = sqlx::postgres::PgPoolOptions::new()
    .max_connections(5)
    .min_connections(1)
    .acquire_timeout(std::time::Duration::from_secs(5))
    .idle_timeout(std::time::Duration::from_secs(30))
    .max_lifetime(std::time::Duration::from_secs(120))
    .connect(&database_url)
    .await
    .expect("Failed to connect");
```

**Key Settings:**
- `max_connections`: Limit per test to prevent pool exhaustion
- `acquire_timeout`: Fail fast instead of indefinite wait
- `idle_timeout`: Release idle connections for other tests
- `max_lifetime`: Prevent long-lived connection issues

#### Solution 4: Health-Check Based Waiting

Replace fixed timeouts with actual connectivity verification:

```rust
// Retry connection with exponential backoff
let mut retry_count = 0;
let max_retries = 5;
let pool = loop {
    match PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
    {
        Ok(pool) => break pool,
        Err(_e) if retry_count < max_retries => {
            retry_count += 1;
            let delay = Duration::from_millis(100 * 2_u64.pow(retry_count));
            tokio::time::sleep(delay).await;
        }
        Err(e) => panic!(
            "Failed to connect after {} retries: {}",
            max_retries, e
        ),
    }
};
```

**Reference:** See `reinhardt-test/src/fixtures/testcontainers.rs` for production implementation.

---

## rstest Best Practices

### TF-0 (MUST): rstest for All Test Cases

**ALL** test cases in this project MUST use **rstest** as the test framework.

**Why?**
- Consistent fixture pattern across all tests
- Easy dependency injection for test setup
- Parameterized testing support
- Better integration with `reinhardt-test` fixtures

**Requirements:**
- Import `rstest::*` in all test modules
- Use `#[rstest]` attribute instead of `#[test]`
- Use `#[rstest]` with `#[tokio::test]` for async tests
- Leverage fixtures for setup/teardown

**Examples:**

❌ **BAD - Using standard #[test]:**
```rust
#[test]
fn test_basic_operation() {
    // Setup code duplicated in every test
    let db = setup_database();
    assert!(db.is_connected());
}
```

✅ **GOOD - Using rstest:**
```rust
use rstest::*;

#[rstest]
fn test_basic_operation(db_fixture: Database) {
    assert!(db_fixture.is_connected());
}

#[rstest]
#[tokio::test]
async fn test_async_operation(#[future] postgres_container: PostgresFixture) {
    let (container, pool) = postgres_container.await;
    assert!(pool.is_connected());
}
```

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

## reinhardt-test Fixture Standards

### RF-1 (MUST): Use reinhardt-test Fixtures for Setup/Teardown

**ALL** test setup and teardown MUST use fixtures from `reinhardt-test` crate.

**Available Generic Fixtures:**
- `postgres_container` - PostgreSQL database container
- `mysql_container` - MySQL database container
- `redis_container` - Redis cache container
- `mongodb_container` - MongoDB container
- `cockroachdb_container` - CockroachDB container
- `rabbitmq_container` - RabbitMQ message queue container
- `localstack_fixture` - AWS LocalStack for S3, DynamoDB, etc.
- `postgres_with_migrations_from` - PostgreSQL with migrations applied
- `mysql_with_migrations_from` - MySQL with migrations applied
- `sqlite_with_migrations_from` - SQLite with migrations applied

**Why?**
- Consistent infrastructure setup across all tests
- Proper resource cleanup via RAII pattern
- Optimized container configuration with retry logic
- Prevents flaky tests from resource exhaustion

**Example:**
```rust
use reinhardt_test::fixtures::*;
use rstest::*;

#[rstest]
#[tokio::test]
async fn test_database_operation(
    #[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String)
) {
    let (_container, pool, _port, _url) = postgres_container.await;

    // Test code using the database pool
    let result = sqlx::query("SELECT 1").fetch_one(pool.as_ref()).await;
    assert!(result.is_ok());
}
```

### RF-2 (MUST): Specialized Fixture Pattern

Create **test-specific specialized fixtures** that wrap generic `reinhardt-test` fixtures to inject test data.

**Pattern:**
1. Create a specialized fixture for each test case or test group
2. Call generic `reinhardt-test` fixtures from the specialized fixture
3. Inject test-specific data using the prepared infrastructure
4. Return both infrastructure and test data to the test case

**Why?**
- Maintains abstraction between infrastructure and test data
- Reusable test data setup
- Clear separation of concerns
- Easy to modify test data without touching infrastructure setup

**Example - Specialized Fixture Pattern:**
```rust
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use sea_query::{Iden, PostgresQueryBuilder, Query};
use sqlx::Row;

/// Test-specific user data structure
struct TestUserData {
    pool: Arc<sqlx::PgPool>,
    admin_user_id: i64,
    regular_user_id: i64,
}

/// Specialized fixture for user authentication tests
#[fixture]
async fn user_auth_fixture(
    #[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String)
) -> (ContainerAsync<GenericImage>, TestUserData) {
    let (container, pool, _port, _url) = postgres_container.await;

    // Create test schema using sea-query
    #[derive(Iden)]
    enum Users {
        Table,
        Id,
        Username,
        Email,
        IsAdmin,
    }

    let create_table = sea_query::Table::create()
        .table(Users::Table)
        .if_not_exists()
        .col(sea_query::ColumnDef::new(Users::Id).big_integer().not_null().auto_increment().primary_key())
        .col(sea_query::ColumnDef::new(Users::Username).string().not_null())
        .col(sea_query::ColumnDef::new(Users::Email).string().not_null())
        .col(sea_query::ColumnDef::new(Users::IsAdmin).boolean().not_null().default(false))
        .build(PostgresQueryBuilder);

    sqlx::query(&create_table).execute(pool.as_ref()).await.unwrap();

    // Insert test data using sea-query
    let insert_admin = Query::insert()
        .into_table(Users::Table)
        .columns([Users::Username, Users::Email, Users::IsAdmin])
        .values_panic(["admin".into(), "admin@example.com".into(), true.into()])
        .returning_col(Users::Id)
        .build(PostgresQueryBuilder);

    let admin_row = sqlx::query(&insert_admin.0)
        .fetch_one(pool.as_ref())
        .await
        .unwrap();
    let admin_user_id: i64 = admin_row.get(0);

    let insert_user = Query::insert()
        .into_table(Users::Table)
        .columns([Users::Username, Users::Email, Users::IsAdmin])
        .values_panic(["user".into(), "user@example.com".into(), false.into()])
        .returning_col(Users::Id)
        .build(PostgresQueryBuilder);

    let user_row = sqlx::query(&insert_user.0)
        .fetch_one(pool.as_ref())
        .await
        .unwrap();
    let regular_user_id: i64 = user_row.get(0);

    let test_data = TestUserData {
        pool,
        admin_user_id,
        regular_user_id,
    };

    (container, test_data)
}

/// Test case using the specialized fixture
#[rstest]
#[tokio::test]
async fn test_admin_permissions(
    #[future] user_auth_fixture: (ContainerAsync<GenericImage>, TestUserData)
) {
    let (_container, test_data) = user_auth_fixture.await;

    // Test admin user permissions
    #[derive(Iden)]
    enum Users {
        Table,
        Id,
        IsAdmin,
    }

    let query = Query::select()
        .column(Users::IsAdmin)
        .from(Users::Table)
        .and_where(sea_query::Expr::col(Users::Id).eq(test_data.admin_user_id))
        .build(PostgresQueryBuilder);

    let row = sqlx::query(&query.0)
        .fetch_one(test_data.pool.as_ref())
        .await
        .unwrap();

    let is_admin: bool = row.get(0);
    assert!(is_admin);
}
```

### RF-3 (MUST): Use SeaQuery for SQL Construction

**NEVER** use raw SQL strings in tests. **ALWAYS** use SeaQuery (v1.0.0-rc) for building SQL queries.

**Why?**
- Type-safe SQL construction
- Prevents SQL injection vulnerabilities
- Database-agnostic query building
- Compile-time validation of table/column names
- Consistent with production code patterns

**Requirements:**
- Use `#[derive(Iden)]` for table and column definitions
- Use appropriate query builder for target database:
  - `PostgresQueryBuilder` for PostgreSQL
  - `MysqlQueryBuilder` for MySQL
  - `SqliteQueryBuilder` for SQLite
- Build queries using `Query::select()`, `Query::insert()`, `Query::update()`, `Query::delete()`

**Examples:**

❌ **BAD - Raw SQL strings:**
```rust
#[rstest]
#[tokio::test]
async fn test_user_query(#[future] postgres_fixture: DbFixture) {
    let (_container, pool) = postgres_fixture.await;

    // ❌ Raw SQL string - avoid this
    sqlx::query("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool.as_ref())
        .await
        .unwrap();

    // ❌ Raw SQL for insert
    sqlx::query("INSERT INTO users (name, email) VALUES ($1, $2)")
        .bind("Alice")
        .bind("alice@example.com")
        .execute(pool.as_ref())
        .await
        .unwrap();
}
```

✅ **GOOD - Using SeaQuery:**
```rust
use sea_query::{Iden, PostgresQueryBuilder, Query, Expr};

#[derive(Iden)]
enum Users {
    Table,
    Id,
    Name,
    Email,
}

#[rstest]
#[tokio::test]
async fn test_user_query(#[future] postgres_fixture: DbFixture) {
    let (_container, pool) = postgres_fixture.await;

    // ✅ Type-safe query with SeaQuery
    let (sql, values) = Query::select()
        .columns([Users::Id, Users::Name, Users::Email])
        .from(Users::Table)
        .and_where(Expr::col(Users::Id).eq(user_id))
        .build_sqlx(PostgresQueryBuilder);

    sqlx::query_with(&sql, values)
        .fetch_one(pool.as_ref())
        .await
        .unwrap();

    // ✅ Type-safe insert with SeaQuery
    let (sql, values) = Query::insert()
        .into_table(Users::Table)
        .columns([Users::Name, Users::Email])
        .values_panic(["Alice".into(), "alice@example.com".into()])
        .build_sqlx(PostgresQueryBuilder);

    sqlx::query_with(&sql, values)
        .execute(pool.as_ref())
        .await
        .unwrap();
}
```

**SeaQuery Common Patterns:**

```rust
use sea_query::{Iden, PostgresQueryBuilder, Query, Expr, Order};

#[derive(Iden)]
enum Posts {
    Table,
    Id,
    Title,
    AuthorId,
    CreatedAt,
}

// SELECT with JOIN and ORDER
let query = Query::select()
    .columns([Posts::Id, Posts::Title])
    .from(Posts::Table)
    .inner_join(
        Users::Table,
        Expr::col((Posts::Table, Posts::AuthorId))
            .equals((Users::Table, Users::Id))
    )
    .order_by(Posts::CreatedAt, Order::Desc)
    .limit(10)
    .build(PostgresQueryBuilder);

// UPDATE with conditions
let query = Query::update()
    .table(Posts::Table)
    .value(Posts::Title, "Updated Title")
    .and_where(Expr::col(Posts::Id).eq(post_id))
    .build(PostgresQueryBuilder);

// DELETE with conditions
let query = Query::delete()
    .from_table(Posts::Table)
    .and_where(Expr::col(Posts::AuthorId).eq(author_id))
    .build(PostgresQueryBuilder);
```

---

## Migration Registry Testing

### MRT-1 (MUST): Use LocalRegistry for Unit Tests

**NEVER** use the global migration registry in unit tests. Always use `LocalRegistry` for test isolation.

**Why?** Global registry (using linkme's `distributed_slice`) causes "duplicate distributed_slice" errors when tests run in parallel.

❌ **BAD - Global Registry in Tests:**
```rust
use reinhardt_migrations::registry::{global_registry, MigrationRegistry};

#[test]
fn test_migration_registration() {
    // ❌ Uses global registry - will conflict with other tests
    let migrations = global_registry().all_migrations();
    assert!(!migrations.is_empty());
}
```

✅ **GOOD - LocalRegistry for Isolation:**
```rust
use reinhardt_migrations::registry::{LocalRegistry, MigrationRegistry};

#[test]
fn test_migration_registration() {
    let registry = LocalRegistry::new();

    registry.register(Migration {
        app_label: "polls".to_string(),
        name: "0001_initial".to_string(),
        operations: vec![],
        dependencies: vec![],
    }).unwrap();

    assert_eq!(registry.all_migrations().len(), 1);
}
```

### MRT-2 (SHOULD): Use reinhardt-test Fixtures

For convenience, use the `migration_registry` fixture from `reinhardt-test`:

```rust
use reinhardt_test::fixtures::*;
use reinhardt_migrations::Migration;
use rstest::*;

#[rstest]
fn test_with_fixture(migration_registry: LocalRegistry) {
    // Registry starts empty
    assert!(migration_registry.all_migrations().is_empty());

    migration_registry.register(Migration {
        app_label: "polls".to_string(),
        name: "0001_initial".to_string(),
        operations: vec![],
        dependencies: vec![],
    }).unwrap();

    assert_eq!(migration_registry.all_migrations().len(), 1);
}
```

### MRT-3 (MUST): Global Registry Tests Must Use #[serial]

When testing the global registry itself, use `#[serial(global_registry)]` to prevent concurrent access:

```rust
use serial_test::serial;

#[test]
#[serial(global_registry)]
fn test_global_registry() {
    let registry = global_registry();

    // Clear before test to ensure clean state
    registry.clear();

    registry.register(migration).unwrap();

    // Test code...

    // Clean up after test
    registry.clear();
}
```

**Critical Rules:**
- Always call `registry.clear()` at start and end of test
- Never rely on global state from other tests
- Use `#[serial(global_registry)]` on ALL global registry tests

### MRT-4 (SHOULD): Test Migration Registration in Examples

In example projects, register migrations via `collect_migrations!` macro:

```rust
// examples/my-project/src/apps/polls/migrations.rs
pub mod _0001_initial;
pub mod _0002_add_fields;

reinhardt::collect_migrations!(
    app_label = "polls",
    _0001_initial,
    _0002_add_fields,
);
```

Then test the global registry in integration tests:

```rust
// examples/my-project/tests/migration_tests.rs
use reinhardt_migrations::registry::{global_registry, MigrationRegistry};

#[test]
fn test_polls_migrations_registered() {
    let registry = global_registry();
    let polls_migrations = registry.migrations_for_app("polls");

    assert_eq!(polls_migrations.len(), 2);
    assert!(polls_migrations.iter().any(|m| m.name == "0001_initial"));
    assert!(polls_migrations.iter().any(|m| m.name == "0002_add_fields"));
}
```

---

## Related Documentation

- **Main Quick Reference**: @CLAUDE.md (see Quick Reference section)
- **Testing Checklist**: See Testing Philosophy and Implementation sections above
- **Test Patterns**: See rstest Best Practices and Common Pitfalls sections above
- **Main standards**: @CLAUDE.md
- **Anti-patterns**: @docs/ANTI_PATTERNS.md
