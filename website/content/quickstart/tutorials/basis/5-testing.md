+++
title = "Part 5: Testing"
weight = 50

[extra]
sidebar_weight = 50
+++

# Part 5: Testing

By Part 5 the polls application has three places where things can break: the typestate-built model layer, the native server functions the WASM client calls, and the WASM components that run those calls in the browser. The reference example covers all three with three distinct test surfaces, each with its own toolbox.

> **Reference implementation**: the finished tests live at [`examples/examples-tutorial-basis/tests/integration.rs`](https://github.com/kent8192/reinhardt-web/tree/main/examples/examples-tutorial-basis/tests/integration.rs) and [`examples/examples-tutorial-basis/tests/wasm/polls_mock_test.rs`](https://github.com/kent8192/reinhardt-web/tree/main/examples/examples-tutorial-basis/tests/wasm/polls_mock_test.rs), with smaller in-file unit tests under `src/apps/polls/models.rs` and `src/shared/forms.rs`. If the prose below ever drifts from those files, **the files win.**

**Conventions enforced in every test** (see also `instructions/TESTING_STANDARDS.md`):

- Use `#[rstest]` — never plain `#[test]`.
- Follow Arrange / Act / Assert with explicit `// Arrange`, `// Act`, `// Assert` comments. The labels may be omitted in tests of 5 lines or fewer.
- Every test MUST exercise at least one component from the `reinhardt` crate or from the example crate itself.
- Strict assertions (`assert_eq!`, `assert!(matches!(...))`) — no `contains(...)` matches without a justification comment.
- Tests that touch global state go inside a named `#[serial(group_name)]` group.
- All temp files and database connections are cleaned up — `NamedTempFile` + `Drop` does this automatically when you keep the handle alive for the lifetime of the test.

## The three test surfaces

| Surface | Where it lives | Target | Purpose |
|---|---|---|---|
| **In-file unit tests** | `#[cfg(test)] mod tests` inside `src/apps/polls/models.rs`, `src/shared/forms.rs` | Native | Pure-Rust coverage for the typestate builder and form metadata. |
| **Native integration tests** | `tests/integration.rs` | Native, `required-features = ["with-reinhardt"]` | Drive the real server functions against an isolated SQLite database, with `serial_test` to serialise anything touching global state. |
| **WASM mock tests** | `tests/wasm/polls_mock_test.rs` | WASM, `required-features = ["msw"]` | Run inside `wasm-pack test` under headless Chrome. Invoke the typed `#[server_fn]` client stubs while MSW (`MockServiceWorker`) intercepts the underlying `window.fetch()` call. |

Each surface compiles for exactly one target, gated by `cfg` attributes and `required-features` so `wasm-pack test` and `cargo nextest run` never link the wrong binary.

## Step 1 — `Cargo.toml`: feature flags and `[[test]]` targets

Open `Cargo.toml` and look at the testing-related blocks. Every line below is verbatim from the reference example.

### Features

```toml
[features]
default = ["with-reinhardt", "client-router"]
# client-router: Enable client-side routing support (required for #[routes] macro with UnifiedRouter)
client-router = []
with-reinhardt = []
# `msw` is forwarded to the facade so `#[server_fn]` generates the
# `MockableServerFn` markers consumed by `tests/wasm/polls_mock_test.rs`.
# This feature only resolves once the framework `msw` facade flag ships
# (tracked in #4287 / PR #4288); until then, leaving it absent makes the
# typed MSW test target skip cleanly via its `required-features = ["msw"]`.
msw = []
```

Three feature flags matter for testing:

- `with-reinhardt` (in the default set) gates the native binary build and the native integration test target. Without it, `cargo build --tests` for `wasm32-unknown-unknown` would try to compile `src/bin/manage.rs` and the integration target — both pull in tokio / sqlx — and fail.
- `client-router` is also default; the `#[routes]` macro returns a `UnifiedRouter`, which the WASM SPA composes via `mount_unified`.
- `msw` opts in to the `MockableServerFn` marker types that `#[server_fn]` generates. The reference example deliberately leaves `msw = []` empty until the upstream facade flag ships, so the typed WASM test target *skips cleanly* on `wasm-pack test` instead of failing to resolve the feature.

### Dev-dependencies, gated per target

```toml
[dev-dependencies]
reinhardt = { workspace = true, features = ["test"] }
rstest = { version = "0.26", default-features = false }

# Native-only dev-dependencies (consumed by `tests/integration.rs`). Kept out
# of the shared `[dev-dependencies]` block because tokio/sqlx/tempfile and
# their transitive deps (mio etc.) do not build for `wasm32-unknown-unknown`,
# which would otherwise break `wasm-pack test`.
[target.'cfg(not(all(target_family = "wasm", target_os = "unknown")))'.dev-dependencies]
serial_test = "3.2"
tokio = { version = "1.48.0", features = ["rt", "macros"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }
tempfile = "3.15"

# WASM-specific dev-dependencies
[target.'cfg(all(target_family = "wasm", target_os = "unknown"))'.dev-dependencies]
wasm-bindgen-test = "0.3.56"
```

The **shared** block pulls in `reinhardt` with its `test` feature (re-exports `MockServiceWorker`) and `rstest 0.26`. The **native-only** block adds `serial_test`, `tokio` (`rt + macros`), `sqlx` (`runtime-tokio + sqlite`), and `tempfile`; they are excluded from the WASM build because mio (a sqlx transitive dep) does not link for `wasm32-unknown-unknown`. The **WASM-only** block adds `wasm-bindgen-test 0.3.56`, the test harness `wasm-pack test` looks for.

### `[[test]]` targets

```toml
[[test]]
name = "integration"
required-features = ["with-reinhardt"]

# WASM-only test target. The file is `#![cfg(wasm)]`-gated, so it compiles
# to a no-op on native and only fires under `wasm-pack test`. Declaring it
# explicitly is necessary because Cargo does not auto-discover test files
# under `tests/<subdir>/`.
[[test]]
name = "polls_mock_test"
path = "tests/wasm/polls_mock_test.rs"
required-features = ["msw"]
```

The first block is auto-discovered (the file lives at `tests/integration.rs`); the `required-features = ["with-reinhardt"]` lets `wasm-pack test` skip it cleanly.

The second block is **explicitly declared** with both `name` and `path`. This is the key Cargo subtlety: **Cargo does not auto-discover test files under `tests/<subdir>/`**. Putting the WASM test under `tests/wasm/` keeps it separated from the native target, but you have to spell it out in `Cargo.toml`. The `required-features = ["msw"]` gate then makes it disappear from `cargo nextest run` and from any `wasm-pack test` invocation that does not opt in.

## Step 2 — In-file unit tests with `rstest`

The smallest unit of testing is an inline `#[cfg(test)] mod tests` block in the source file it covers.

### `src/apps/polls/models.rs` — typestate builder coverage

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn test_question_build_typestate() {
        // `Question::build()` surfaces each required field as a named setter,
        // which keeps tutorial call sites stable as the `Question` schema grows.
        // The same typestate constructor (introduced for `#[model]` in issue
        // #4400 and extended to FK fields in #4413) is also used by
        // `Choice::build()` in the integration / wasm tests, where the model
        // has a live database backing it. Persisting `vote()` therefore
        // belongs in those tests, not in this synchronous unit test.
        let question = Question::build()
            .question_text("What's your favorite color?")
            .author(1_i64)
            .finish();
        assert_eq!(question.question_text(), "What's your favorite color?");
        // `pub_date` is `auto_now_add`, so `finish()` populates it just like
        // `new()` would.
        assert!(question.was_published_recently());
    }
}
```

Two things to notice:

1. **No `// Arrange` / `// Act` / `// Assert` labels** — the test is well under the 5-line threshold per phase, and the call-site context makes the intent self-evident. The labels become mandatory once a test grows or setup involves more than one statement.
2. **Every assertion exercises a Reinhardt component** — `Question::build()` (typestate builder generated by `#[model]`), `Question::question_text()` accessor, and `Question::was_published_recently()`. No `assert_eq!(1 + 1, 2)`-style filler. `Choice::vote()` is *not* unit-tested here because it is now `async fn` and calls `self.save().await` internally; that exercise lives in `tests/integration.rs::test_choice_vote_increment`, which seeds a real SQLite row first.

### `src/shared/forms.rs` — form metadata coverage

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use reinhardt::forms::wasm_compat::FormExt;
    use rstest::rstest;

    #[rstest]
    fn test_vote_form_metadata() {
        let form = create_vote_form();
        let metadata = form.to_metadata();

        assert_eq!(metadata.fields.len(), 1);
        assert_eq!(metadata.fields[0].name, "choice");
        assert!(metadata.fields[0].required);
    }
}
```

`Form::to_metadata()` lives in `reinhardt::forms::wasm_compat::FormExt`. The test confirms that the single `choice` field — the one the WASM voter sees as a hidden input — survives the conversion to the DTO that crosses the WASM↔native boundary.

These in-file tests live next to the code they exercise, are auto-discovered by Cargo (no `[[test]]` block needed), and run as part of `cargo make test-unit`.

## Step 3 — Native integration tests in `tests/integration.rs`

This is the workhorse. The file is gated by `#![cfg(native)]` and `required-features = ["with-reinhardt"]`, and split into three `mod` blocks: `database_tests`, `server_fn_tests`, and `auth_tests`. Each module addresses a different concern.

### File-level gating

```rust
// Native-only: this file uses tokio/sqlx/tempfile which don't build for wasm32.
// `wasm-pack test` builds all `--tests` targets; without this gate the test
// binary tries (and fails) to link sqlx for wasm32.
#![cfg(native)]
```

`#![cfg(native)]` doubles up with `required-features = ["with-reinhardt"]` in `Cargo.toml` — belt and braces.

### Pattern A — raw `sqlx` + `tempfile` fixture

When you need full control over the schema (or migrations are not yet generated), set up the database by hand with `sqlx`:

```rust
#[cfg(with_reinhardt)]
mod database_tests {
    use rstest::*;
    use sqlx::SqlitePool;
    use std::sync::Arc;
    use tempfile::NamedTempFile;

    /// Fixture: SQLite database with tables created
    #[fixture]
    async fn sqlite_with_polls_tables() -> (NamedTempFile, Arc<SqlitePool>) {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let db_path = temp_file.path().to_str().unwrap().to_string();
        let database_url = format!("sqlite://{}?mode=rwc", db_path);

        let pool = SqlitePool::connect(&database_url)
            .await
            .expect("Failed to connect to SQLite");
        let pool = Arc::new(pool);

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS polls_question (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                question_text VARCHAR(200) NOT NULL,
                pub_date DATETIME NOT NULL
            )
            "#,
        )
        .execute(pool.as_ref())
        .await
        .expect("Failed to create polls_question table");
        // ... polls_choice table elided ...

        (temp_file, pool)
    }
}
```

Three things are worth calling out:

- The fixture **returns the `NamedTempFile` together with the pool.** Keeping the temp file alive for the lifetime of the test is what makes Drop-based cleanup work — drop the temp file early and SQLite's underlying file disappears mid-query.
- `sqlite://...?mode=rwc` is `sqlx`'s way of saying "create the file if it doesn't exist". The reinhardt ORM equivalent (`sqlite:///...`) appears in Pattern B below.
- The fixture is an `async fn` annotated with `#[fixture]`. `rstest` accepts it as a `#[future]` parameter in the consumer test.

A consumer test follows the AAA pattern:

```rust
#[rstest]
#[tokio::test]
async fn test_question_database_create(
    #[future] sqlite_with_polls_tables: (NamedTempFile, Arc<SqlitePool>),
) {
    let (_file, pool) = sqlite_with_polls_tables.await;

    // Create a question
    let question_text = "What's your favorite color?";
    let row = sqlx::query_as::<_, (i64, String, chrono::NaiveDateTime)>(
        "INSERT INTO polls_question (question_text, pub_date) VALUES ($1, CURRENT_TIMESTAMP) RETURNING id, question_text, pub_date"
    )
    .bind(question_text)
    .fetch_one(pool.as_ref())
    .await
    .expect("Failed to insert question");

    assert_eq!(row.1, question_text);
}
```

The `_file` underscore prefix tells the compiler the binding is held intentionally even though we never read it — its `Drop` impl fires at the *end* of the test, not the moment the fixture returns.

### Pattern B — server function tests against the ORM

A different module in the same file, `server_fn_tests`, exercises the actual `#[server_fn]` functions. These need both a `sqlx` pool *and* a reinhardt `DatabaseConnection`, plus the global ORM database has to be initialised before any `Question::objects()` call:

```rust
#[cfg(all(with_reinhardt, server))]
mod server_fn_tests {
    use reinhardt::DatabaseConnection;
    use reinhardt::db::orm::reinitialize_database;
    use rstest::*;
    use serial_test::serial;
    use sqlx::SqlitePool;
    use std::sync::Arc;
    use tempfile::NamedTempFile;

    use examples_tutorial_basis::apps::polls::server_fn::{
        get_question_detail, get_question_results, get_questions, vote,
    };
    use examples_tutorial_basis::shared::types::VoteRequest;

    /// Fixture: SQLite database with tables, test data, and DatabaseConnection.
    /// Also initializes the global ORM database connection for server functions.
    #[fixture]
    async fn sqlite_with_test_data() -> (NamedTempFile, Arc<SqlitePool>, DatabaseConnection) {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let db_path = temp_file.path().to_str().unwrap().to_string();

        // URL for sqlx direct connection (with mode parameter for create-if-missing)
        let sqlx_url = format!("sqlite://{}?mode=rwc", db_path);
        // URL for reinhardt ORM (use sqlite:/// for absolute path, no query parameters).
        // reinhardt's connect_sqlite automatically sets create_if_missing(true).
        let orm_url = format!("sqlite:///{}", db_path);

        // ... pool + schema creation + test data insert elided ...

        // Initialize the global ORM database for server functions.
        // Server functions use Question::objects() which relies on global database.
        reinitialize_database(&orm_url)
            .await
            .expect("Failed to initialize global database");

        let db_conn = DatabaseConnection::connect_sqlite(&orm_url)
            .await
            .expect("Failed to create DatabaseConnection");

        (temp_file, pool, db_conn)
    }

    #[rstest]
    #[tokio::test]
    #[serial(server_fn_tests)]
    async fn test_get_questions_server_fn(
        #[future] sqlite_with_test_data: (NamedTempFile, Arc<SqlitePool>, DatabaseConnection),
    ) {
        let (_file, _pool, db_conn) = sqlite_with_test_data.await;

        // Test: Get questions via server function (pass DatabaseConnection as argument)
        let result = get_questions(db_conn).await;
        let questions = result.expect("get_questions should succeed");
        assert_eq!(questions.len(), 1, "Should have 1 question");
        assert_eq!(questions[0].question_text, "What's your favorite color?");
    }
}
```

Two ideas converge here:

- **Two URL forms for the same SQLite file.** `sqlx` wants `sqlite://path?mode=rwc`; reinhardt's `connect_sqlite` wants `sqlite:///path` and sets `create_if_missing(true)` itself. Both point at the same `NamedTempFile`.
- **Direct invocation of `#[server_fn]`s.** `get_questions(db_conn).await` calls the server function exactly as if it were any other async Rust function. The `#[inject]` attributes that show up in the function signature are stripped at expansion time on the server side (see `examples/CLAUDE.md` § TS-3), so the integration test passes `DatabaseConnection` positionally.

### `serial_test`: the global-state guardrail

Look closely at the attribute stack on `test_get_questions_server_fn`:

```rust
#[rstest]
#[tokio::test]
#[serial(server_fn_tests)]
async fn test_get_questions_server_fn(...) { ... }
```

`#[serial(group_name)]` from `serial_test 3.2` stops tests in the same group from running in parallel. The reason it matters here is `reinitialize_database(&orm_url)` — the call inside the fixture mutates a process-global. Two parallel tests would race on it and produce mysterious "no such table" or "database is locked" failures.

Every test in `mod server_fn_tests` that calls `reinitialize_database` carries `#[serial(server_fn_tests)]` so they run one at a time within that named group. Tests in unrelated groups (`database_tests`, `auth_tests`) keep running in parallel — `serial_test`'s named-group form gives you exactly that granularity. Reach for `#[serial(group_name)]` any time a test mutates an env var, a singleton, an admin registration, or a settings override.

### Pattern C — authorization tests with no schema

The third module, `auth_tests`, demonstrates that not every native integration test needs a full schema. The CUD `#[server_fn]`s in `apps::polls::server_fn` all start with a `(*session_user).as_ref().map_err(ServerFnError::from)?` gate (resolved from the `Depends<Result<User, SessionError>>` DI factory in `apps::polls::di`); when the session is empty the factory yields `Err(SessionError::Anonymous)` and the `From<&SessionError> for ServerFnError` conversion returns 401 *before* the handler touches the database, so the fixture is an empty SQLite file:

```rust
#[cfg(with_reinhardt)]
mod auth_tests {
    use examples_tutorial_basis::apps::polls::di::SessionError;
    use examples_tutorial_basis::apps::polls::server_fn::{
        create_choice, create_question, delete_choice, delete_question, update_choice,
        update_question,
    };
    use examples_tutorial_basis::apps::users::models::User;
    use reinhardt::DatabaseConnection;
    use reinhardt::di::Depends;
    use rstest::*;
    use tempfile::NamedTempFile;

    /// Fixture: an empty SQLite database + DatabaseConnection wired through
    /// reinhardt-orm. No tables are created; the authorization tests below
    /// short-circuit on the `SessionError` gate before any query runs.
    #[fixture]
    async fn empty_db_conn() -> (NamedTempFile, DatabaseConnection) {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let db_path = temp_file.path().to_str().unwrap().to_string();
        let orm_url = format!("sqlite:///{}", db_path);
        let db_conn = DatabaseConnection::connect_sqlite(&orm_url)
            .await
            .expect("Failed to create DatabaseConnection");
        (temp_file, db_conn)
    }

    /// Anonymous session error wrapped in `Depends<Result<User, SessionError>>`
    /// — the same value the request-scoped `session_user_factory` in
    /// `apps::polls::di` would produce for a `SessionData` without a
    /// `user_id` key. We construct it directly with `Depends::from_value`
    /// so the test does not need to spin up the middleware stack or the DI
    /// container; the gate under test is `From<&SessionError> for
    /// ServerFnError`, not the factory itself.
    fn anonymous_session_user() -> Depends<Result<User, SessionError>> {
        Depends::from_value(Err(SessionError::Anonymous))
    }

    #[rstest]
    #[tokio::test]
    async fn test_create_question_requires_auth(
        #[future] empty_db_conn: (NamedTempFile, DatabaseConnection),
    ) {
        // Arrange
        let (_file, db_conn) = empty_db_conn.await;
        let session_user = anonymous_session_user();

        // Act
        let result = create_question(
            "Anonymous attempt".to_string(),
            "csrf-token-ignored".to_string(),
            db_conn,
            session_user,
        )
        .await;

        // Assert
        assert_unauthorized(result, "create_question");
    }

    // ... five more parallel tests for update_question / delete_question /
    //     create_choice / update_choice / delete_choice ...
}
```

This is the canonical example of a test that follows AAA explicitly: three lines for Arrange, one block for Act, one line for Assert. The named `anonymous_session_user()` helper hides the only piece of setup shared across all six tests in this module.

The `assert_unauthorized` helper near the top of the module shows how to keep brittle string-matching contained:

```rust
fn assert_unauthorized<T>(
    result: std::result::Result<T, reinhardt::pages::server_fn::ServerFnError>,
    operation: &str,
) {
    let err = result
        .err()
        .unwrap_or_else(|| panic!("{} should reject anonymous callers", operation));
    // The `From<&SessionError>` impl emits
    // `ServerFnError::server(401, ...)`. We match on the Debug-formatted
    // output because `ServerFnError` does not expose the inner status as
    // a typed accessor in the public API, and its `Debug` impl is the
    // most stable representation that includes the numeric status.
    let rendered = format!("{:?}", err);
    assert!(
        rendered.contains("401") || rendered.contains("Authentication required"),
        "{} should fail with 401, got: {}",
        operation,
        rendered
    );
}
```

The comment above the `format!("{:?}", err)` line is the justification for a `contains(...)` assertion — exactly the kind of comment `instructions/TESTING_STANDARDS.md` § TI-5 asks for when strict assertions are impractical. As soon as `ServerFnError` grows a typed status accessor, this helper collapses to `assert_eq!(err.status(), 401)`.

## Step 4 — WASM mock tests via MSW

The WASM target answers a different question: when the client-side `#[server_fn]` stub fires its `window.fetch()` request, do the polls components handle the typed response correctly?

The test file is `tests/wasm/polls_mock_test.rs`. Cargo cannot auto-discover files under `tests/<subdir>/`, which is why the explicit `[[test]] name = "polls_mock_test", path = "..."` block exists in `Cargo.toml`.

### Why MSW?

The `msw` feature on the `reinhardt` facade flips on a `MockableServerFn` marker generation pass inside the `#[server_fn]` macro. Each server function — `get_questions`, `get_question_detail`, `get_question_results`, `vote` — emits a `marker` type that test code passes to `MockServiceWorker::handle_server_fn::<...::marker>(...)`. At runtime the worker intercepts `window.fetch()` for the URL the typed client stub would have hit, serves whatever the test handler returned, and counts the call so the test can assert it actually happened.

In other words: the WASM test runs the **real** application code path, all the way down to the typed client stub, and only short-circuits the actual HTTP hop. No fake polls components, no fake server functions — just a fake network.

### File-level setup

```rust
#![cfg(wasm)]

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// Import actual components from the application
use examples_tutorial_basis::apps::polls::client::components::{
    polls_detail, polls_index, polls_results,
};
use examples_tutorial_basis::apps::polls::server_fn::{
    get_question_detail, get_question_results, get_questions, vote,
};
use examples_tutorial_basis::shared::types::{ChoiceInfo, QuestionInfo, VoteRequest};
use reinhardt::pages::component::Page;
use reinhardt::pages::server_fn::ServerFnError;
use reinhardt::test::msw::MockServiceWorker;
```

Three things in this preamble do real work:

- `#![cfg(wasm)]` makes the whole file a no-op on native builds. Combined with `required-features = ["msw"]`, this means `cargo nextest run --all-features` skips it silently and `wasm-pack test --headless --chrome` is the only thing that runs it.
- `wasm_bindgen_test_configure!(run_in_browser)` tells `wasm-bindgen-test` to drive headless Chrome instead of a Node.js shim.
- `MockServiceWorker` comes from `reinhardt::test::msw` — enabled by the `test` feature on the shared `reinhardt` dev-dependency. The `msw` Cargo feature on the application crate is a *separate* gate that controls *whether the application's own `#[server_fn]`s emit marker types*.

### Mock fixtures

The file declares plain (non-fixture) helpers because each test composes them differently:

```rust
fn mock_question() -> QuestionInfo {
    QuestionInfo {
        id: 1,
        question_text: "What is your favorite programming language?".to_string(),
        pub_date: chrono::Utc::now(),
    }
}

fn mock_questions_list() -> Vec<QuestionInfo> { /* three QuestionInfo values */ }
fn mock_choices() -> Vec<ChoiceInfo>          { /* three ChoiceInfo values, "Rust" / "Python" / "JavaScript" */ }
```

The DTOs (`QuestionInfo`, `ChoiceInfo`, `VoteRequest`) are the same `serde`-derived structs from `src/shared/types.rs` — the WASM test uses them verbatim because that is what flows back through the wire from a real call.

### A success-path test

```rust
/// `get_questions()` returns the list mocked by MSW (success path).
#[wasm_bindgen_test]
async fn test_get_questions_returns_mocked_list() {
    let worker = MockServiceWorker::new();
    worker.handle_server_fn::<get_questions::marker>(|_args| Ok(mock_questions_list()));
    worker.start().await;

    let questions = get_questions().await.expect("server_fn should succeed");

    assert_eq!(questions.len(), 3);
    assert_eq!(
        questions[0].question_text,
        "What is your favorite programming language?"
    );
    assert_eq!(questions[1].id, 2);

    worker
        .calls_to_server_fn::<get_questions::marker>()
        .assert_called();
}
```

The four lines around `worker` are the entire MSW lifecycle:

1. `MockServiceWorker::new()` constructs a worker but does not yet register it with the browser.
2. `worker.handle_server_fn::<get_questions::marker>(|_args| Ok(...))` says "when `get_questions::marker`'s URL is requested, return this `Ok(...)`".
3. `worker.start().await` installs the worker on the page so subsequent `fetch()` calls go through it.
4. `worker.calls_to_server_fn::<get_questions::marker>().assert_called()` after the act phase confirms the production code actually reached the worker.

In between, `get_questions().await` is the *real* client stub generated by `#[server_fn]`. It serializes its argument list, fires a `fetch()`, MSW catches it and returns the mock payload, the stub deserializes the response into `Vec<QuestionInfo>`, and the test then asserts on the typed result.

### An error-path test

```rust
/// `get_questions()` surfaces a server-side error from MSW (error path).
#[wasm_bindgen_test]
async fn test_get_questions_surfaces_server_error() {
    let worker = MockServiceWorker::new();
    worker.handle_server_fn::<get_questions::marker>(|_args| {
        Err(ServerFnError::server(500, "Internal server error"))
    });
    worker.start().await;

    let err = get_questions().await.expect_err("expected server error");
    match err {
        ServerFnError::Server { status, message } => {
            assert_eq!(status, 500, "expected HTTP 500 status");
            assert_eq!(
                message, "Internal server error",
                "expected mocked server message to propagate verbatim"
            );
        }
        other => panic!("expected ServerFnError::Server, got: {other:?}"),
    }
}
```

`assert!(matches!(...))` would also work here, but the `match` form makes the *expected* and *unexpected* arms equally explicit. Either form satisfies the strict-assertion rule from `instructions/TESTING_STANDARDS.md` § TI-5 (loose `contains(...)` matches need a justification comment; pattern matches on a typed variant do not).

### A component-with-MSW smoke test

The file also mounts real polls components while MSW is active. These tests do not await reactive re-renders — that would require a scheduler flush outside the scope of this file — but they prove the component constructs without panicking when MSW is in place:

```rust
/// `polls_index()` constructs cleanly with MSW intercepting `get_questions`.
#[wasm_bindgen_test]
async fn test_polls_index_with_msw_active() {
    let worker = MockServiceWorker::new();
    worker.handle_server_fn::<get_questions::marker>(|_args| Ok(mock_questions_list()));
    worker.start().await;

    let view = polls_index();
    assert!(matches!(view, Page::Element(_)));
}
```

`polls_index`, `polls_detail`, and `polls_results` are the exact `page!` components from `src/client/components/polls.rs`. The mock returns a `Vec<QuestionInfo>` the component knows how to render, the test asserts the produced `Page` is a `Page::Element` variant, and that is enough to catch the regressions this layer cares about: a panic in component construction, an MSW handler that fails to register, or a misshapen DTO that does not deserialize.

## Step 5 — Run the tests with `cargo make`

The `Makefile.toml` exposes four entry points:

```toml
[tasks.test]
description = "Run all tests"
command = "cargo"
args = ["nextest", "run", "--all-features"]

[tasks.test-unit]
description = "Run unit tests only"
command = "cargo"
args = ["nextest", "run", "--lib", "--all-features"]

[tasks.test-integration]
description = "Run integration tests only"
command = "cargo"
args = ["nextest", "run", "--test", "*", "--all-features"]

[tasks.wasm-test]
description = "Run WASM tests in headless Chrome"
command = "wasm-pack"
args = ["test", "--headless", "--chrome", "--", "--no-default-features"]
```

In practice:

```bash
# Everything that compiles for the host target (in-file tests + integration.rs).
cargo make test

# Only the in-file `#[cfg(test)] mod tests` blocks under src/.
cargo make test-unit

# Only the `tests/integration.rs` target.
cargo make test-integration

# Headless Chrome run of tests/wasm/polls_mock_test.rs.
cargo make wasm-test
```

Two details about `wasm-test` are worth highlighting:

- The trailing `["--", "--no-default-features"]` forwards `--no-default-features` to `cargo build`. That excludes `with-reinhardt` from the build (so the native `manage` binary and the integration test target are not pulled into the WASM compile). `wasm-pack` re-enables `client-router` because the WASM build needs it for the SPA routing setup; `with-reinhardt` is the one that has to disappear.
- `wasm-pack test --headless --chrome` requires Chrome / Chromium on `PATH`. The first run will also pull `wasm-bindgen-cli` if it is not already cached.

You typically run `cargo make test` and `cargo make wasm-test` side by side — one covers everything exercisable on the native target, the other covers the typed client-stub round-trip that only happens in the browser.

## What we deliberately left out

A few things from earlier drafts of this chapter are intentionally absent:

- **TestContainers / Postgres fixtures.** The basis example is single-binary, single-SQLite, by design. Adding a Docker dependency for a tutorial that walks through `Question` and `Choice` would buy nothing and break local-dev setup for anyone without Docker. When you do need real Postgres, the `reinhardt-test` workspace ships fixtures under `reinhardt::test::fixtures`; reach for them in a follow-up project, not here.
- **Workspace-only `cargo make` targets** such as `cargo make clippy-todo-check` and `cargo make placeholder-check`. Those tasks exist in the umbrella `Makefile.toml` for the reinhardt-web workspace itself, not in this example's `Makefile.toml`. Running them from the example directory will fail to find a task.
- **Skeleton tests with a single `assert!(true)`.** Forbidden by `instructions/TESTING_STANDARDS.md` § TP-1.

## Recap

You now have three concentric rings of test coverage:

1. **Inline `#[rstest]` blocks** for the small things — typestate builders, form metadata, anything pure-Rust that does not need a database. They live in the same file as the code, and `cargo make test-unit` runs them.
2. **`tests/integration.rs`** for the things that need state — raw `sqlx` for full schema control, the reinhardt ORM via `DatabaseConnection::connect_sqlite` for real `#[server_fn]` invocation, `serial_test` to serialise tests that touch `reinitialize_database` or any other process-global, and a deliberately schema-less module for the session-user DI factory's 401 gate (the `From<&SessionError>` conversion on an `Err(SessionError::Anonymous)`). `cargo make test` and `cargo make test-integration` run it.
3. **`tests/wasm/polls_mock_test.rs`** for the typed client stubs — MSW intercepts the `window.fetch()` call from `get_questions` / `get_question_detail` / `get_question_results` / `vote`, the test asserts on the typed return value, and the polls components are mounted in headless Chrome to confirm they construct without panicking. `cargo make wasm-test` runs it.

If you change a server function signature, the native integration tests notice. If you change a DTO field, the WASM mock tests notice. If you change a typestate builder field, the in-file unit tests notice. Each ring catches a different class of regression, and the `Cargo.toml` feature flags make sure they only ever compile for the target they belong to.

## What's Next?

Now that the polls application is covered by tests at every layer, we can confidently iterate on the front-end. The next chapter wires up the static-assets pipeline: how `dist-wasm/` becomes `staticfiles/` and how the Pages template ships both server-rendered assets and the WASM bundle.

Continue to [Part 6: Static Files](../6-static-files/).
