# reinhardt-testkit

Core testing infrastructure for the Reinhardt framework.

## Model-Derived Test Databases

`TestDatabase` creates an isolated database for tests and applies schema from
either `Model` metadata or application migrations.

```rust,ignore
use reinhardt_testkit::fixtures::{TestDatabase, test_database};

#[rstest]
#[tokio::test]
async fn uses_model_schema() {
    let db = test_database!(WritingProject, Document).await.unwrap();

    let rows = db
        .connection()
        .fetch_all("SELECT * FROM writing_project", Vec::new())
        .await
        .unwrap();

    assert!(rows.is_empty());
}
```

Use the builder when a test needs migrations or integration options:

```rust,ignore
let db = TestDatabase::builder()
    .migrations::<WritingSourcesMigrations>()
    .with_di_context()
    .build()
    .await
    .unwrap();
```

## DI mock fixtures

`reinhardt-testkit` exposes three layers for mocking DI dependencies in tests:

1. `with_di_overrides!` macro (most ergonomic)
2. `DiOverrideBuilder` + `injection_context_with_di_overrides` (closure form)
3. `injection_context_with_overrides` (legacy; scope-seeding only)

```rust,no_run
use reinhardt_testkit::with_di_overrides;
use rstest::*;

#[rstest]
#[tokio::test]
async fn example() {
    let (ctx, _di) = with_di_overrides! {
        singleton Config { url: "test".into() },
    };
    // Use `ctx` as your test InjectionContext.
}
```

See `instructions/TESTING_STANDARDS.md` (the TI- entry about `with_di_overrides!`) for the full rule set.
