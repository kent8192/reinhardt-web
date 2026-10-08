# reinhardt-testkit

Core testing infrastructure for the Reinhardt framework.

## Async Stub Routes

`stub::StubRouter` builds native test servers from async closures that can
capture shared state. Register each path with its HTTP method:

```rust
use reinhardt_testkit::{http::Response, stub::StubRouter};

let router = StubRouter::new()
    .post("/webhook", |request| async move {
        Ok(Response::ok().with_body(request.body().clone()))
    })
    .get("/health", |_request| async { Ok(Response::ok()) })
    .into_server_router();
```

In an async test, start the server with
`reinhardt_testkit::test_server_guard(router).await`. Keep the returned guard
alive while sending requests; dropping it shuts down the server. The router
uses the framework's 404/405 handling, and duplicate `(path, method)`
registrations panic immediately. GET, POST, PUT, PATCH, and DELETE have
helpers; use `.route(path, method, handler)` for other methods.

`SimpleHandler` is deprecated starting in `0.4.0-alpha.21` and remains available
for compatibility. Migrate from `SimpleHandler::new(|request| { ... })` and
manual method checks to `StubRouter::new().post(path, |request| async move {
... })` (or the matching HTTP method). Return the response from the async
closure, then convert the builder into a `ServerRouter`. Application endpoints
that need extractors or named routes should use HTTP method macros and
`ServerRouter::endpoint`.

## Model-Derived Test Databases

`TestDatabase` creates an isolated database for tests and applies schema from
either `Model` metadata or application migrations.

Model-derived schemas preserve physical column names, defaults, generated
columns, relationships and through tables, model constraints, and indexes so
ORM behavior matches migration-backed databases.

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
