+++
title = "Part 2: Your First Feature - the Poll Index"
description = "Create the polls app, add the first migration, expose get_questions, and render the index page."
weight = 20

[extra]
sidebar_weight = 20
+++

# Part 2: Your First Feature - the Poll Index

In this part you will build the first vertical slice: a `polls` app with `Question` and `Choice` models, a first migration, a `get_questions` server function, and a WASM index page that lists polls.

Ownership and authentication come later. That is deliberate. The first migration creates polls without an `author_id` column; Part 5 adds ownership with a second migration.

## Create the Polls App

Generate a pages app:

```bash
reinhardt-admin startapp polls --template pages
```

The app should be registered in `src/config/apps.rs`:

```rust
use reinhardt::installed_apps;

installed_apps! {
    polls: "polls",
}
```

The completed example also registers `users`, but do not add it until Part 4.

## Add the Initial Models

Open `src/apps/polls/models.rs` and add the first version of the poll models:

```rust
use chrono::{DateTime, Utc};
use reinhardt::db::associations::ForeignKeyField;
use reinhardt::prelude::*;
use serde::{Deserialize, Serialize};

#[model(app_label = "polls", table_name = "questions")]
#[derive(Serialize, Deserialize)]
pub struct Question {
    #[field(primary_key = true)]
    pub id: i64,

    #[field(max_length = 200)]
    pub question_text: String,

    #[field(auto_now_add = true)]
    pub pub_date: DateTime<Utc>,
}

#[model(app_label = "polls", table_name = "choices")]
#[derive(Serialize, Deserialize)]
pub struct Choice {
    #[field(primary_key = true)]
    pub id: i64,

    #[rel(foreign_key, related_name = "choices")]
    pub question: ForeignKeyField<Question>,

    #[field(max_length = 200)]
    pub choice_text: String,

    #[field(default = 0)]
    pub votes: i32,
}
```

This is the first-slice version. In the completed example, `Question` also has an `author` foreign key to `User`; Part 5 adds that field and migration.

## Generate and Apply Migration 0001

Create the first migration:

```bash
cargo make makemigrations
cargo make migrate
```

The generated migration should create `questions` with `id`, `pub_date`, and `question_text`, but no `author_id`. The reference migration's `questions` table contains only these columns:

```rust
Operation::CreateTable {
    name: "questions".to_string(),
    columns: vec![
        ColumnDefinition {
            name: "id".to_string(),
            type_definition: FieldType::BigInteger,
            not_null: true,
            unique: false,
            primary_key: true,
            auto_increment: true,
            default: None,
        },
        ColumnDefinition {
            name: "pub_date".to_string(),
            type_definition: FieldType::TimestampTz,
            not_null: true,
            unique: false,
            primary_key: false,
            auto_increment: false,
            default: None,
        },
        ColumnDefinition {
            name: "question_text".to_string(),
            type_definition: FieldType::VarChar(200u32),
            not_null: true,
            unique: false,
            primary_key: false,
            auto_increment: false,
            default: None,
        },
    ],
    constraints: vec![],
    without_rowid: None,
    interleave_in_parent: None,
    partition: None,
}
```

If `author_id` appears in `0001_initial.rs`, you have accidentally skipped ahead to Part 5.

## Re-export the Shared Info Types

The `#[model]` macro generates model-info companion types that are safe to send to the browser. Re-export them from `src/shared/types.rs`:

```rust
pub use crate::apps::polls::models::{ChoiceInfo, QuestionInfo};
```

This keeps the server function return type and the WASM component type identical.

## Add the Server Function

Create `src/apps/polls/server_fn.rs` and expose a query for the index page:

```rust
use crate::shared::types::QuestionInfo;
use reinhardt::pages::server_fn::{ServerFnError, server_fn};

#[server_fn]
pub async fn get_questions(
    #[inject] _db: reinhardt::DatabaseConnection,
) -> std::result::Result<Vec<QuestionInfo>, ServerFnError> {
    use crate::apps::polls::models::Question;
    use reinhardt::Model;

    let manager = Question::objects();
    let questions = manager
        .all()
        .all()
        .await
        .map_err(|e| ServerFnError::application(e.to_string()))?;

    let latest: Vec<QuestionInfo> = questions
        .into_iter()
        .take(5)
        .map(QuestionInfo::from)
        .collect();

    Ok(latest)
}
```

The current reference implementation takes five rows from the manager query. Do not rely on a specific ordering until you add one explicitly.

## Split Server and Client Routes

The app-level `src/apps/polls/urls.rs` exposes separate router builders for the two targets:

```rust
#[cfg(server)]
pub mod server_urls;

#[cfg(client)]
pub mod client_router;

#[cfg(server)]
pub fn server_url_patterns() -> reinhardt::ServerRouter {
    server_urls::server_url_patterns()
}

#[cfg(client)]
pub fn client_url_patterns() -> reinhardt::ClientRouter {
    client_router::client_url_patterns()
}
```

Register the server function in `src/apps/polls/urls/server_urls.rs`:

```rust
use crate::apps::polls::server_fn::get_questions;
use reinhardt::ServerRouter;
use reinhardt::pages::server_fn::ServerFnRouterExt;

pub fn server_url_patterns() -> ServerRouter {
    ServerRouter::new().server_fn(get_questions::marker)
}
```

Register the index client route in `src/apps/polls/urls/client_router.rs`:

```rust
use crate::client::pages::index_page;
use reinhardt::ClientRouter;

pub fn client_url_patterns() -> ClientRouter {
    ClientRouter::new().route("index", "/", index_page)
}
```

At the project level, aggregate app routers in `src/config/urls.rs`. Do not list individual poll server functions here:

```rust
#[routes]
pub fn routes() -> UnifiedRouter {
    let router = UnifiedRouter::new();

    #[cfg(server)]
    let router = router.server(|s| {
        s.mount("/", crate::apps::polls::urls::server_url_patterns())
    });

    #[cfg(client)]
    let router = router.mount_unified(
        "/",
        UnifiedRouter::new().client(|_| crate::apps::polls::urls::client_url_patterns()),
    );

    router
}
```

## Render the Index Page

The client entry point from Part 1 loads routes from inventory:

```rust
ClientLauncher::new("#root")
    .register_routes_from_inventory()
    .launch()
```

The page aggregator maps the named route to the polls component:

```rust
pub fn index_page() -> Page {
    with_nav(crate::apps::polls::client::components::polls_index())
}
```

The index component uses the server function as an async resource:

```rust
pub fn polls_index() -> Page {
    let load_questions = use_resource(
        || async move { get_questions().await.map_err(|e| e.to_string()) },
        (),
    );

    page!(|load_questions: Resource<Vec<QuestionInfo>, String>| {
        div {
            class: "max-w-4xl mx-auto px-4 mt-12",
            h1 { "Polls" }
            {
                match load_questions.get() {
                    ResourceState::Loading => page!(|| {
                        p { "Loading..." }
                    })(),
                    ResourceState::Success(questions) if questions.is_empty() => page!(|| {
                        p {
                            class: "text-muted",
                            "No polls are available."
                        }
                    })(),
                    ResourceState::Success(questions) => page!(|questions: Vec<QuestionInfo>| {
                        div {
                            class: "space-y-2",
                            for question in questions {
                                a {
                                    href: polls_routes::reverse("detail", &[("question_id", question.id.to_string().as_str())]),
                                    class: "block p-4 border border-border rounded-lg bg-surface-primary hover:bg-surface-secondary transition-colors",
                                    { question.question_text.clone() }
                                }
                            }
                        }
                    })(questions),
                    ResourceState::Error(error) => page!(|error: String| {
                        div {
                            class: "alert-danger",
                            { error }
                        }
                    })(error),
                }
            }
        }
    })(load_questions)
}
```

The final example adds a "Create new poll" button and owner-only controls. Leave those out for now; they need authentication and ownership from Parts 4 and 5.

## Seed a Poll

Until the admin arrives in Part 6, the quickest local seed is SQL. Use the same database that `cargo make dev` points at:

```sql
insert into questions (question_text, pub_date)
values ('What should we build next?', now());

insert into choices (question_id, choice_text, votes)
values (1, 'More tutorials', 0), (1, 'More examples', 0);
```

If your database does not support `now()`, use the timestamp literal form it expects.

## Checkpoint

Run the app:

```bash
cargo make dev
```

Open `http://127.0.0.1:8000/`. You should see the poll list rendered by the WASM client. Clicking a poll may route to a not-found or unfinished page until Part 3 adds the detail route.

Before continuing:

- `migrations/polls/0001_initial.rs` has no `author_id`.
- `get_questions` returns `Vec<QuestionInfo>`.
- `src/config/urls.rs` aggregates `polls::urls::server_url_patterns()` and `polls::urls::client_url_patterns()`.
- The browser renders the poll index through `ClientLauncher`.
