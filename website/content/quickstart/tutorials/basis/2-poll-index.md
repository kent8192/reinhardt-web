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

`startapp` updates `src/config/apps.rs` for you. Check that `polls` is present, but do not hand-edit this file unless you created the app directory manually:

```rust
use reinhardt::installed_apps;

installed_apps! {
    polls: "polls",
}
```

The completed example also contains `users`, but that entry is added by the Part 4 `startapp users --template pages` command.

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

## Use the Generated Model Info DTOs

`#[model]` generates serializable info companions for models that are not marked `server_only`. In this tutorial, `QuestionInfo` and `ChoiceInfo` come from `src/apps/polls/models.rs`; do not hand-write duplicate DTOs.

Keep the `models` module available on both targets, while server-only helpers such as migrations, admin, and service code stay behind their module-level gates. Then server functions and WASM components can import the same generated DTOs:

```rust
use crate::apps::polls::models::{ChoiceInfo, QuestionInfo};
```

This keeps the server function return type and the WASM component type identical. Part 5 adds the generated `author` relation to `QuestionInfo` after the `users` app exists.

## Add the Server Function

Create `src/apps/polls/server_fn.rs` and expose a query for the index page:

```rust
use crate::apps::polls::models::{Question, QuestionInfo};
use reinhardt::{DatabaseConnection, Model};
use reinhardt::pages::server_fn::{ServerFnError, server_fn};
use std::result::Result;

#[server_fn]
pub async fn get_questions(
    #[inject] _db: DatabaseConnection,
) -> Result<Vec<QuestionInfo>, ServerFnError> {
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

The app-level `src/apps/polls/urls.rs` stays target-neutral. It aggregates the split router modules:

```rust
#[cfg(not(client))]
mod client_route_specs;

#[cfg(client)]
pub mod client_router;

#[cfg(client)]
pub use client_router::{client_url_patterns, reverse};
#[cfg(not(client))]
pub use client_route_specs::{client_url_patterns, reverse};

#[cfg(server)]
pub mod server_router;

#[cfg(server)]
pub use server_router::server_url_patterns;
```

Put the client route table in `src/apps/polls/urls/client_router.rs`:

```rust
use crate::apps::polls::client::components;
use reinhardt::ClientRouter;

pub fn client_url_patterns() -> ClientRouter {
    ClientRouter::new().component(components::polls_index::polls_index)
}

pub fn reverse(name: &str, params: &[(&str, &str)]) -> String {
    client_url_patterns()
        .reverse(name, params)
        .unwrap_or_else(|error| panic!("failed to reverse polls client route `{name}`: {error}"))
}
```

`client_router.rs` is gated by `#[cfg(client)]` at the module declaration. Native builds still need the same route names for `mount_unified()` and `reverse()`, so keep target-neutral metadata in `src/apps/polls/urls/client_route_specs.rs`:

```rust
use reinhardt::ClientRouter;
use reinhardt::pages::component::Page;

pub fn client_url_patterns() -> ClientRouter {
    ClientRouter::new().route("index", "/", Page::empty)
}

pub fn reverse(name: &str, params: &[(&str, &str)]) -> String {
    client_url_patterns()
        .reverse(name, params)
        .unwrap_or_else(|error| panic!("failed to reverse polls client route `{name}`: {error}"))
}
```

Register the server function in `src/apps/polls/urls/server_router.rs`:

```rust
use crate::apps::polls::server_fn::get_questions;
use reinhardt::ServerRouter;
use reinhardt::pages::server_fn::ServerFnRouterExt;

pub fn server_url_patterns() -> ServerRouter {
    ServerRouter::new().server_fn(get_questions::marker)
}
```

At the project level, aggregate app routers in `src/config/urls.rs`. Do not list individual poll server functions here:

```rust
use crate::apps::polls::urls as polls_urls;

#[routes]
pub fn routes() -> UnifiedRouter {
    let router = UnifiedRouter::new();

    #[cfg(server)]
    let router = router.server(|s| {
        s.mount("/", polls_urls::server_url_patterns())
    });

    let router = router.mount_unified(
        "/",
        UnifiedRouter::new().client(|_| polls_urls::client_url_patterns()),
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

Create the app-local route-backed wrapper in `src/apps/polls/client/components/polls_index.rs`. The component macro owns the route metadata, so no separate `pages.rs` wrapper is needed:

```rust
use reinhardt::pages::component;
use reinhardt::pages::component::Page;

use crate::client::components::nav::with_nav;

#[component("/", "index")]
pub fn polls_index() -> Page {
    with_nav(super::polls_index())
}
```

Then implement the page body in `src/apps/polls/client/components.rs` so tests can exercise it directly and routes can wrap it with the shared nav:

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

Until the admin arrives in Part 6, the quickest local seed is SQL. Use the same PostgreSQL database that `cargo make dev` points at. In the example profile, open it with:

```bash
PGPASSWORD=reinhardt psql -h localhost -p 5433 -U reinhardt -d examples_tutorial_basis
```

Then paste:

```sql
WITH inserted_question AS (
    INSERT INTO questions (question_text, pub_date)
    VALUES ('What should we build next?', NOW())
    RETURNING id
)
INSERT INTO choices (question_id, choice_text, votes)
SELECT id, 'More tutorials', 0 FROM inserted_question
UNION ALL
SELECT id, 'More examples', 0 FROM inserted_question;
```

If you switch the tutorial profile to SQLite, open that database with `sqlite3 <path-to-db.sqlite3>` and use SQLite's inserted-ID syntax instead of `RETURNING`.

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
