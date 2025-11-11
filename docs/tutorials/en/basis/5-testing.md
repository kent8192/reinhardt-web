# Part 5: Testing

In this tutorial, we'll write automated tests using modern Rust testing tools: **rstest** for fixtures and **TestContainers** for database isolation.

## Why Testing Matters

Tests help you:

- **Save time**: Automated tests catch bugs faster than manual testing
- **Prevent bugs**: Tests illuminate unexpected behavior before production
- **Build confidence**: Well-tested code is easier to modify and extend
- **Enable collaboration**: Tests protect against accidental breakage by teammates

## Test Dependencies

Add testing dependencies to `Cargo.toml`:

```toml
[dev-dependencies]
rstest = { workspace = true }
reinhardt-test = { workspace = true }
testcontainers = { workspace = true }
tokio = { version = "1", features = ["full", "test-util"] }
```

## Writing Your First Test

Let's identify a bug in our `was_published_recently()` method. It returns `true` for questions whose `pub_date` is in the future, which is incorrect.

Create `polls/tests.rs`:

```rust
use super::models::Question;
use chrono::{Duration, Utc};

#[test]
fn test_was_published_recently_with_future_question() {
    // Create a question 30 days in the future
    let future_date = Utc::now() + Duration::days(30);
    let question = Question {
        id: Some(1),
        question_text: "Future question".to_string(),
        pub_date: future_date,
    };

    // Should return false for future questions
    assert_eq!(question.was_published_recently(), false);
}

#[test]
fn test_was_published_recently_with_old_question() {
    // Create a question 2 days ago
    let old_date = Utc::now() - Duration::days(2);
    let question = Question {
        id: Some(1),
        question_text: "Old question".to_string(),
        pub_date: old_date,
    };

    // Should return false for questions older than 1 day
    assert_eq!(question.was_published_recently(), false);
}

#[test]
fn test_was_published_recently_with_recent_question() {
    // Create a question from 23 hours ago
    let recent_date = Utc::now() - Duration::hours(23);
    let question = Question {
        id: Some(1),
        question_text: "Recent question".to_string(),
        pub_date: recent_date,
    };

    // Should return true for recent questions
    assert_eq!(question.was_published_recently(), true);
}
```

Run the tests:

```bash
cargo test --package polls
```

You'll see that the first test fails. The bug is already fixed in our implementation from Part 2:

```rust
impl Question {
    /// Check if this question was published recently (within the last day)
    pub fn was_published_recently(&self) -> bool {
        let now = Utc::now();
        let one_day_ago = now - chrono::Duration::days(1);
        // Fixed: Also check that pub_date is not in the future
        self.pub_date >= one_day_ago && self.pub_date <= now
    }
}
```

## Testing with Database using rstest + TestContainers

Let's test database operations using rstest fixtures and TestContainers for isolation.

### Understanding reinhardt-test Fixtures

Reinhardt provides shared fixtures in `reinhardt-test/src/fixtures.rs`:

```rust
use rstest::*;
use reinhardt_test::fixtures::postgres_fixture;
use testcontainers::{ContainerAsync, GenericImage};
use reinhardt_db::backends::DatabaseConnection;
use std::sync::Arc;

#[fixture]
async fn postgres_fixture() -> (ContainerAsync<GenericImage>, Arc<DatabaseConnection>) {
    // Automatically starts PostgreSQL container
    // Returns container handle and database connection
}
```

**Available fixtures:**
- `postgres_fixture` - PostgreSQL database in Docker container
- `sqlite_fixture` - SQLite in-memory database
- `mysql_fixture` - MySQL database in Docker container

### Using Fixtures in Tests

Create `polls/tests/database_tests.rs`:

```rust
use rstest::*;
use reinhardt_test::fixtures::postgres_fixture;
use testcontainers::{ContainerAsync, GenericImage};
use reinhardt_db::backends::DatabaseConnection;
use std::sync::Arc;
use chrono::Utc;
use crate::models::{Question, Choice};

#[rstest]
#[tokio::test]
async fn test_create_and_retrieve_question(
    #[future] postgres_fixture: (ContainerAsync<GenericImage>, Arc<DatabaseConnection>)
) {
    let (_container, conn) = postgres_fixture.await;

    // Create a question
    let question = Question::create(
        &conn,
        "What's your favorite language?".to_string(),
        Utc::now(),
    )
    .await
    .unwrap();

    assert!(question.id.is_some());

    // Retrieve it
    let retrieved = Question::get(&conn, question.id.unwrap())
        .await
        .unwrap()
        .expect("Question not found");

    assert_eq!(retrieved.question_text, "What's your favorite language?");

    // Container is automatically cleaned up when test ends
}

#[rstest]
#[tokio::test]
async fn test_question_choices_relationship(
    #[future] postgres_fixture: (ContainerAsync<GenericImage>, Arc<DatabaseConnection>)
) {
    let (_container, conn) = postgres_fixture.await;

    // Create question
    let question = Question::create(
        &conn,
        "Test question".to_string(),
        Utc::now(),
    )
    .await
    .unwrap();

    let question_id = question.id.unwrap();

    // Add choices
    Choice::create(&conn, question_id, "Rust".to_string()).await.unwrap();
    Choice::create(&conn, question_id, "Python".to_string()).await.unwrap();
    Choice::create(&conn, question_id, "Go".to_string()).await.unwrap();

    // Retrieve choices
    let choices = question.choices(&conn).await.unwrap();

    assert_eq!(choices.len(), 3);
    assert_eq!(choices[0].choice_text, "Rust");
}

#[rstest]
#[tokio::test]
async fn test_increment_votes(
    #[future] postgres_fixture: (ContainerAsync<GenericImage>, Arc<DatabaseConnection>)
) {
    let (_container, conn) = postgres_fixture.await;

    let question = Question::create(&conn, "Test".to_string(), Utc::now())
        .await
        .unwrap();

    let mut choice = Choice::create(&conn, question.id.unwrap(), "Option A".to_string())
        .await
        .unwrap();

    assert_eq!(choice.votes, 0);

    // Increment votes
    choice.increment_votes(&conn).await.unwrap();
    assert_eq!(choice.votes, 1);

    choice.increment_votes(&conn).await.unwrap();
    assert_eq!(choice.votes, 2);
}
```

**Key points:**

- `#[rstest]` - Enables fixture injection
- `#[future]` - Required for async fixtures
- `.await` - Don't forget to await the fixture!
- Container cleanup is automatic via RAII

## Testing Views with rstest

Create `polls/tests/view_tests.rs`:

```rust
use rstest::*;
use reinhardt_test::fixtures::postgres_fixture;
use testcontainers::{ContainerAsync, GenericImage};
use reinhardt_db::backends::DatabaseConnection;
use reinhardt_http::{Request, Response};
use std::sync::Arc;
use chrono::Utc;
use crate::models::Question;
use crate::views;

#[rstest]
#[tokio::test]
async fn test_index_no_questions(
    #[future] postgres_fixture: (ContainerAsync<GenericImage>, Arc<DatabaseConnection>)
) {
    let (_container, conn) = postgres_fixture.await;

    // Call index view with empty database
    let response = views::index(conn).await.unwrap();

    assert_eq!(response.status(), 200);
    // Verify response body contains "No polls"
}

#[rstest]
#[tokio::test]
async fn test_index_with_questions(
    #[future] postgres_fixture: (ContainerAsync<GenericImage>, Arc<DatabaseConnection>)
) {
    let (_container, conn) = postgres_fixture.await;

    // Create test data
    Question::create(&conn, "Test question 1".to_string(), Utc::now())
        .await
        .unwrap();
    Question::create(&conn, "Test question 2".to_string(), Utc::now())
        .await
        .unwrap();

    // Call index view
    let response = views::index(conn.clone()).await.unwrap();

    assert_eq!(response.status(), 200);
    // Verify both questions appear in response
}

#[rstest]
#[tokio::test]
async fn test_detail_not_found(
    #[future] postgres_fixture: (ContainerAsync<GenericImage>, Arc<DatabaseConnection>)
) {
    let (_container, conn) = postgres_fixture.await;

    let mut request = Request::new();
    request.path_params.insert("question_id".to_string(), "999".to_string());

    let response = views::detail(request, conn).await;

    // Should return 404 for non-existent question
    assert!(response.is_err() || response.unwrap().status() == 404);
}
```

## Custom Fixtures

You can create your own fixtures for common test data:

```rust
use rstest::*;

#[fixture]
fn sample_question() -> Question {
    Question {
        id: None,
        question_text: "Sample question".to_string(),
        pub_date: Utc::now(),
    }
}

#[fixture]
async fn question_with_choices(
    #[future] postgres_fixture: (ContainerAsync<GenericImage>, Arc<DatabaseConnection>)
) -> (Question, Vec<Choice>) {
    let (_container, conn) = postgres_fixture.await;

    let question = Question::create(&conn, "Test".to_string(), Utc::now())
        .await
        .unwrap();

    let choices = vec![
        Choice::create(&conn, question.id.unwrap(), "A".to_string()).await.unwrap(),
        Choice::create(&conn, question.id.unwrap(), "B".to_string()).await.unwrap(),
        Choice::create(&conn, question.id.unwrap(), "C".to_string()).await.unwrap(),
    ];

    (question, choices)
}

#[rstest]
#[tokio::test]
async fn test_with_custom_fixture(
    #[future] question_with_choices: (Question, Vec<Choice>)
) {
    let (question, choices) = question_with_choices.await;

    assert_eq!(choices.len(), 3);
    assert!(question.id.is_some());
}
```

## Testing Best Practices

### 1. Test Organization

```
polls/
├── lib.rs
├── models.rs
├── views.rs
└── tests.rs           # Unit tests
    ├── database_tests.rs  # Database integration tests
    └── view_tests.rs      # View integration tests
```

### 2. Assertion Strictness

Use exact assertions, not loose matching:

```rust
// ✅ GOOD - Exact assertion
assert_eq!(question.question_text, "Expected text");

// ❌ BAD - Loose assertion
assert!(response.contains("text"));
```

### 3. Test Isolation

Each test should be independent:

```rust
// ✅ GOOD - Each test gets its own database container
#[rstest]
#[tokio::test]
async fn test_a(#[future] postgres_fixture: ...) {
    let (_container, conn) = postgres_fixture.await;
    // Test code
}

#[rstest]
#[tokio::test]
async fn test_b(#[future] postgres_fixture: ...) {
    let (_container, conn) = postgres_fixture.await;
    // Different container, complete isolation
}
```

### 4. Cleanup is Automatic

TestContainers handles cleanup via RAII:

```rust
#[rstest]
#[tokio::test]
async fn test_with_container(
    #[future] postgres_fixture: (ContainerAsync<GenericImage>, Arc<DatabaseConnection>)
) {
    let (_container, conn) = postgres_fixture.await;

    // Use database
    // ...

    // No manual cleanup needed!
    // Container is automatically stopped and removed when _container drops
}
```

## Running Tests

Run all tests:

```bash
cargo test --workspace
```

Run specific test file:

```bash
cargo test --package polls -- database_tests
```

Run with output:

```bash
cargo test -- --nocapture
```

## Summary

In this tutorial, you learned:

- How to use rstest for fixture-based testing
- How to use TestContainers for database isolation
- How to use reinhardt-test shared fixtures (postgres_fixture, etc.)
- How to test models with database operations
- How to test views with dependency injection
- How to create custom fixtures for common test data
- Best practices for test organization and isolation
- The importance of automatic cleanup via RAII

## What's Next?

Now that we have a well-tested application, let's add static files (CSS, JavaScript, images) to improve the user interface.

Continue to [Part 6: Static Files](6-static-files.md).
