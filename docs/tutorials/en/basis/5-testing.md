# Part 5: Testing

In this tutorial, we'll write automated tests to ensure our application works correctly.

## Why Testing Matters

Tests help you:

- **Save time**: Automated tests catch bugs faster than manual testing
- **Prevent bugs**: Tests illuminate unexpected behavior before production
- **Build confidence**: Well-tested code is easier to modify and extend
- **Enable collaboration**: Tests protect against accidental breakage by teammates

## Writing Your First Test

Let's identify a bug in our `was_published_recently()` method. It returns `True` for questions whose `pub_date` is in the future, which is incorrect.

Create `src/models.rs` tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    #[test]
    fn test_was_published_recently_with_future_question() {
        // Create a question 30 days in the future
        let future_date = Utc::now() + Duration::days(30);
        let question = Question {
            id: 1,
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
            id: 1,
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
            id: 1,
            question_text: "Recent question".to_string(),
            pub_date: recent_date,
        };

        // Should return true for recent questions
        assert_eq!(question.was_published_recently(), true);
    }
}
```

Run the tests:

```bash
cargo test
```

You'll see that the first test fails. Let's fix the bug by updating the `was_published_recently()` method in `src/models.rs`:

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

Run the tests again:

```bash
cargo test
```

All tests should pass now!

## Testing Views

Let's test our views using the Reinhardt test client.

Add the test dependency to `Cargo.toml`:

```toml
[dev-dependencies]
reinhardt = { version = "0.1.0", features = ["test"] }
```

Create tests for the index view in `src/polls.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use reinhardt::test::{APIClient, APITestCase};
    use chrono::Utc;

    #[tokio::test]
    async fn test_no_questions() {
        let client = APIClient::new();
        let response = client.get("/polls/").await.unwrap();

        assert_eq!(response.status(), 200);
        assert!(response.text().contains("No polls are available"));
    }

    #[tokio::test]
    async fn test_past_question() {
        // Create a question in the past
        let pool = setup_test_db().await;
        let past_date = Utc::now() - chrono::Duration::days(30);

        crate::models::Question::create(
            &pool,
            "Past question.".to_string(),
            past_date,
        )
        .await
        .unwrap();

        let client = APIClient::with_pool(pool);
        let response = client.get("/polls/").await.unwrap();

        assert_eq!(response.status(), 200);
        assert!(response.text().contains("Past question"));
    }

    #[tokio::test]
    async fn test_future_question() {
        // Create a question in the future
        let pool = setup_test_db().await;
        let future_date = Utc::now() + chrono::Duration::days(30);

        crate::models::Question::create(
            &pool,
            "Future question.".to_string(),
            future_date,
        )
        .await
        .unwrap();

        let client = APIClient::with_pool(pool);
        let response = client.get("/polls/").await.unwrap();

        assert_eq!(response.status(), 200);
        // Future questions should not be displayed
        assert!(!response.text().contains("Future question"));
    }

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }
}
```

## Testing the Detail View

Add tests for the detail view:

```rust
#[tokio::test]
async fn test_future_question_detail() {
    let pool = setup_test_db().await;
    let future_date = Utc::now() + chrono::Duration::days(5);

    let question_id = crate::models::Question::create(
        &pool,
        "Future question.".to_string(),
        future_date,
    )
    .await
    .unwrap();

    let client = APIClient::with_pool(pool);
    let response = client.get(&format!("/polls/{}/", question_id)).await.unwrap();

    // Should return 404 for future questions
    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn test_past_question_detail() {
    let pool = setup_test_db().await;
    let past_date = Utc::now() - chrono::Duration::days(5);

    let question_id = crate::models::Question::create(
        &pool,
        "Past Question.".to_string(),
        past_date,
    )
    .await
    .unwrap();

    let client = APIClient::with_pool(pool);
    let response = client.get(&format!("/polls/{}/", question_id)).await.unwrap();

    assert_eq!(response.status(), 200);
    assert!(response.text().contains("Past Question"));
}
```

## Testing Best Practices

1. **Test one thing at a time**: Each test should focus on a single behavior
2. **Use descriptive names**: Test names should clearly describe what they test
3. **Arrange-Act-Assert**: Structure tests with setup, execution, and verification
4. **Use test fixtures**: Share common test data setup
5. **Test edge cases**: Don't just test the happy path

## Test Organization

Organize tests by:

- **Unit tests**: Test individual functions and methods (in the same file)
- **Integration tests**: Test multiple components together (in `tests/` directory)
- **Model tests**: Test database models and queries
- **View tests**: Test HTTP endpoints and responses

## Running Specific Tests

Run all tests:

```bash
cargo test
```

Run tests matching a pattern:

```bash
cargo test test_was_published
```

Run tests in a specific module:

```bash
cargo test models::tests
```

## Test Coverage

To check test coverage, use `cargo-tarpaulin`:

```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

This generates a coverage report showing which lines of code are tested.

## Summary

In this tutorial, you learned:

- Why automated testing is important
- How to write unit tests for models
- How to write integration tests for views
- How to use the test client to simulate HTTP requests
- Testing best practices and organization
- How to run and organize tests

Testing is an essential part of professional software development. Well-tested code is easier to maintain, modify, and deploy with confidence.

## What's Next?

In the next tutorial, we'll add CSS styling and images to make our polls app look better.

Continue to [Part 6: Static Files](6-static-files.md).
