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
throughout `0.4.x`, with removal planned for `0.5.0`. Migrate from
`SimpleHandler::new(|request| { ... })` and
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

## API client headers

`APIClient::get_with_headers` and `APIClient::post_raw_with_headers` replace
client default headers with the same name, including names that differ only
in letter case. For example, a request with `Authorization: Bearer alice`
replaces a default `Authorization: Bearer bob` with a single header value.
Unrelated default headers remain present, and subsequent requests retain the
client defaults. Multiple per-request values for the same header are retained
in order, including names that differ only in letter case. Invalid per-request
header names or values return an error.

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

## API Client Requests and Forks

`APIClient::request` builds requests for any HTTP method. Use `.json(&data)`,
`.form(&data)`, or `.body(bytes)` for payloads, then `.send().await`. Conversion
and serialization errors are returned by `send()`.

```rust,no_run
use http::{Method, header::AUTHORIZATION};
use reinhardt_testkit::APIClient;

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let client = APIClient::new();
client.set_header("Authorization", "Bearer bob").await?;
let response = client.request(Method::PUT, "/users/1")
    .header(AUTHORIZATION, "Bearer alice")
    .json(&serde_json::json!({"name": "Alice"}))
    .send().await?;
# Ok(())
# }
```

`.header(name, value)` replaces every existing value of that name;
`.append_header(name, value)` preserves them and appends another value.
`.without_header(name)` removes the header for this request. Operations run in
call order after default headers, the payload's implied Content-Type, manual
cookies, and the forced-auth X-Test-User header. They never change client defaults.
Automatic cookie-jar cookies added by reqwest at send time cannot be removed
per request. A raw body does not set or remove Content-Type.

Derive separate per-credential or subject clients from a fixture with
`let alice = base.fork().await;`. A fork snapshots default headers, manual
cookies, and the forced-auth user into independent state. Its handlers and DI
context are shared, while its connection pool and automatic cookie jar are new.
Timeout, HTTP version, and cookie-store configuration are inherited. Automatic
jar cookies are not inherited; manual cookies are. Forks can make concurrent
requests without racing on shared credential headers.

`get_with_headers` and `post_raw_with_headers` are deprecated in 0.4.0, with
removal planned for 0.5. Migrate to `request(..).header(..).send()`. Existing verb
methods such as `get`, `post`, and `put` remain supported. The deprecated wrappers
also replace defaults when given a header of the same name.
